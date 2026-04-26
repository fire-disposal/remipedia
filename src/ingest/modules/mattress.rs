//! 床垫设备 TCP Msgpack 模块
//!
//! 独立模块：监听TCP端口，处理床垫设备的Msgpack协议数据
//! 包含：TCP监听 + 帧解码 + 状态管理 + 事件检测
//!
//! # 线协议格式 (v2)
//!
//! ```text
//! [0xAB, 0xCD] [len_hi: u8] [len_lo: u8] [crc: u8] [payload: N bytes]
//! ```
//!
//! - Magic: `0xAB 0xCD` (2字节)
//! - 载荷长度: `len_hi << 8 | len_lo` (2字节, 大端序, 最大 65535)
//! - CRC8: 对 `magic + len + payload` 做校验 (1字节)
//! - 载荷: Msgpack 编码的床垫数据 (N字节)

use crate::core::entity::{DataPoint, DataCategory, Severity};
use crate::errors::AppResult;
use crate::repository::{DataRepository, RawDataRepository};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use uuid::Uuid;

/// 线协议: magic 2B + len 2B + crc 1B = 5B 头部
const FRAME_HEADER_SIZE: usize = 5;

/// 最大连续帧错误容忍次数，超过则断开连接防止死循环
const MAX_CONSECUTIVE_FRAME_ERRORS: u32 = 10;

/// 床垫模块配置
#[derive(Debug, Clone)]
pub struct MattressConfig {
    pub bind_addr: SocketAddr,
    pub max_frame_size: usize,
    pub auto_register_device: bool,
}

impl Default for MattressConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:9001".parse().unwrap(),
            max_frame_size: 64 * 1024,
            auto_register_device: true,
        }
    }
}

/// 床垫数据包 (Msgpack解码后)
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MattressPacket {
    serial_number: String,
    manufacturer: String,
    model: String,
    firmware_version: String,
    status: u8,
    heart_rate: u8,
    breath_rate: u8,
    wet_status: u8,
    apnea_count: u8,
    weight_value: u16,
    position: u8,
    timestamp: i64,
}

/// 床垫状态
#[derive(Debug, Default)]
struct MattressState {
    #[allow(dead_code)]
    last_status: Option<u8>,
    #[allow(dead_code)]
    last_position: Option<u8>,
    last_heart_rate: Option<u8>,
    #[allow(dead_code)]
    last_breath_rate: Option<u8>,
    on_bed_since: Option<chrono::DateTime<chrono::Utc>>,
    last_event_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// 床垫设备模块
pub struct MattressModule {
    config: MattressConfig,
}

impl MattressModule {
    pub fn new(config: MattressConfig) -> Self {
        Self { config }
    }

    /// 启动模块
    pub async fn start(&self, pool: &PgPool) -> AppResult<()> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        log::info!("床垫模块启动，监听: {}", self.config.bind_addr);

        let pool = pool.clone();
        let max_frame_size = self.config.max_frame_size;

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let pool = pool.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, addr, &pool, max_frame_size).await {
                                log::error!("床垫连接处理错误 {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("床垫TCP接受连接失败: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

/// 处理单个TCP连接
///
/// 包含帧计数器保护，防止损坏数据导致无限循环。
/// 连续错误超过 [`MAX_CONSECUTIVE_FRAME_ERRORS`] 后主动断开。
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    pool: &PgPool,
    max_frame_size: usize,
) -> AppResult<()> {
    log::info!("床垫设备连接: {}", addr);
    
    let data_repo = DataRepository::new(pool);
    let raw_repo = RawDataRepository::new(pool);
    
    let mut buffer = Vec::with_capacity(4096);
    let mut temp_buf = [0u8; 4096];
    let mut state: Option<MattressState> = None;
    let mut device_id: Option<Uuid> = None;
    let mut consecutive_errors: u32 = 0;

    loop {
        let n = match stream.read(&mut temp_buf).await {
            Ok(0) => {
                log::info!("床垫连接关闭: {}", addr);
                break;
            }
            Ok(n) => n,
            Err(e) => {
                log::error!("床垫TCP读取错误 {}: {}", addr, e);
                break;
            }
        };

        buffer.extend_from_slice(&temp_buf[..n]);

        // 处理所有完整帧
        loop {
            // 帧计数器保护: 连续错误过多时断开连接
            if consecutive_errors >= MAX_CONSECUTIVE_FRAME_ERRORS {
                log::error!(
                    "床垫 {} 连续帧错误达到 {} 次，断开连接",
                    addr,
                    MAX_CONSECUTIVE_FRAME_ERRORS
                );
                return Err(crate::errors::AppError::InternalError(
                    format!("连续帧错误过多 {}", addr)
                ));
            }

            match extract_msgpack_frame(&mut buffer, max_frame_size) {
                Ok(Some(frame)) => {
                    consecutive_errors = 0; // 成功解析，重置计数器
                    
                    // 归档原始数据 (仅归档完整帧)
                    let raw_id = raw_repo.archive_raw("mattress_tcp", &frame, addr.to_string()).await.ok();
                    
                    // 解析数据包
                    match parse_mattress_packet(&frame) {
                        Ok(packet) => {
                            // 首次连接时解析设备ID
                            if device_id.is_none() {
                                device_id = resolve_or_create_device(pool, &packet.serial_number).await.ok();
                            }
                            
                            // 处理数据
                            if let Some(ref dev_id) = device_id {
                                let (points, new_state) = process_mattress_data(
                                    packet,
                                    state.take(),
                                    *dev_id
                                );
                                state = new_state;
                                
                                // 存储数据点
                                if !points.is_empty() {
                                    if let Err(e) = data_repo.insert_datapoints(&points).await {
                                        log::error!("存储床垫数据失败: {}", e);
                                    }
                                }
                                
                                // 标记成功
                                if let Some(id) = raw_id {
                                    let _ = raw_repo.mark_status(id, crate::core::entity::RawIngestStatus::Ingested, None).await;
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("解析床垫数据包失败 {}: {}", addr, e);
                            if let Some(id) = raw_id {
                                let _ = raw_repo.mark_status(id, crate::core::entity::RawIngestStatus::FormatError, Some(&e.to_string())).await;
                            }
                        }
                    }
                }
                Ok(None) => break, // 需要更多数据
                Err(e) => {
                    consecutive_errors += 1;
                    log::warn!("帧提取错误 ({}次) {}: {}", consecutive_errors, addr, e);
                    
                    // 尝试恢复：丢弃损坏字节直到下一个 magic 头
                    if let Some(pos) = find_next_magic(&buffer) {
                        buffer.drain(..pos);
                    } else {
                        buffer.clear();
                    }
                }
            }
        }
    }

    Ok(())
}

/// 计算CRC8校验值 (CRC-8/ITU 标准)
fn crc8(data: &[u8]) -> u8 {
    use crc::{Crc, Algorithm};
    const CRC8: Crc<u8> = Crc::<u8>::new(&Algorithm {
        width: 8,
        poly: 0x07,
        init: 0x00,
        refin: false,
        refout: false,
        xorout: 0x00,
        check: 0xf4,
        residue: 0x00,
    });
    let mut digest = CRC8.digest();
    digest.update(data);
    digest.finalize()
}

/// 提取 Msgpack 帧 (协议 v2)
///
/// 帧格式: `[0xAB, 0xCD] [len_hi] [len_lo] [crc] [payload: N]`
///
/// - `len = (len_hi << 8) | len_lo` 表示 payload 字节数
/// - `crc` 校验范围: magic + len + payload 全部字节
/// - 返回完整帧 (含头部 + payload) 或 None (半包)
fn extract_msgpack_frame(buffer: &mut Vec<u8>, max_size: usize) -> AppResult<Option<Vec<u8>>> {
    if buffer.len() < FRAME_HEADER_SIZE {
        return Ok(None);
    }

    // 查找 magic 头，无法匹配则返回错误供上层恢复
    if buffer[0] != 0xAB || buffer[1] != 0xCD {
        return Err(crate::errors::AppError::ValidationError(
            format!("无效的 magic 头: {:02X} {:02X}", buffer[0], buffer[1])
        ));
    }

    let payload_len = u16::from_be_bytes([buffer[2], buffer[3]]) as usize;

    // 校验 payload_len 是否在合理范围 [1, max_size]
    if payload_len == 0 || payload_len > max_size {
        return Err(crate::errors::AppError::ValidationError(
            format!("载荷长度 {} 不在有效范围 [1, {}]", payload_len, max_size)
        ));
    }

    let total_len = FRAME_HEADER_SIZE + payload_len;
    if buffer.len() < total_len {
        return Ok(None); // 半包，等待更多数据
    }

    // CRC 校验 (对整个帧除 CRC 自身字节外的所有字节)
    let expected_crc = buffer[4];
    let mut crc_data = Vec::with_capacity(4 + payload_len);
    crc_data.extend_from_slice(&buffer[..4]);          // magic + len
    crc_data.extend_from_slice(&buffer[5..total_len]); // payload (跳过 crc 字节)
    let actual_crc = crc8(&crc_data);

    if actual_crc != expected_crc {
        return Err(crate::errors::AppError::ValidationError(
            format!("CRC 校验失败: 期望 {:02X}, 实际 {:02X}", expected_crc, actual_crc)
        ));
    }

    let frame = buffer[..total_len].to_vec();
    buffer.drain(..total_len);
    Ok(Some(frame))
}

/// 查找下一个 magic 头位置 (向前扫描恢复同步)
fn find_next_magic(buffer: &[u8]) -> Option<usize> {
    // 从 index=1 开始查找，以避免匹配到当前帧自身的 magic
    buffer[1..].windows(2).position(|w| w == [0xAB, 0xCD]).map(|p| p + 1)
}

/// 解析床垫数据包 (协议 v2)
///
/// 帧格式: `[magic(2)] [len(2)] [crc(1)] [payload(N)]`
/// 此函数接收完整帧 (含头部), 提取 payload 做 msgpack 解码。
fn parse_mattress_packet(frame: &[u8]) -> AppResult<MattressPacket> {
    if frame.len() < FRAME_HEADER_SIZE + 1 {
        return Err(crate::errors::AppError::ValidationError(
            format!("数据包太短: {} 字节", frame.len())
        ));
    }

    let data = &frame[FRAME_HEADER_SIZE..]; // 跳过 5 字节头部
    let value: serde_json::Value = rmp_serde::from_slice(data)
        .map_err(|e| crate::errors::AppError::ValidationError(format!("Msgpack解析失败: {}", e)))?;

    Ok(MattressPacket {
        serial_number: extract_str(&value, &["sn", "serial_number"])?,
        manufacturer: extract_str(&value, &["ma", "manufacturer"]).unwrap_or_default(),
        model: extract_str(&value, &["md", "model"]).unwrap_or_default(),
        firmware_version: extract_str(&value, &["fv", "firmware_version"]).unwrap_or_default(),
        status: extract_u8(&value, "status").unwrap_or(0),
        heart_rate: extract_u8(&value, "heart_rate").unwrap_or(0),
        breath_rate: extract_u8(&value, "breath_rate").unwrap_or(0),
        wet_status: extract_u8(&value, "wet_status").unwrap_or(0),
        apnea_count: extract_u8(&value, "apnea_count").unwrap_or(0),
        weight_value: extract_u16(&value, "weight_value").unwrap_or(0),
        position: extract_u8(&value, "position").unwrap_or(0),
        timestamp: value.get("ts").and_then(|v| v.as_i64()).unwrap_or_else(|| {
            chrono::Utc::now().timestamp()
        }),
    })
}

/// 处理床垫数据，生成数据点和事件
fn process_mattress_data(
    packet: MattressPacket,
    prev_state: Option<MattressState>,
    device_id: Uuid,
) -> (Vec<DataPoint>, Option<MattressState>) {
    let mut points = Vec::new();
    let mut state = prev_state.unwrap_or_default();
    let now = chrono::Utc::now();

    // 1. 基础指标数据点
    let metric_payload = serde_json::json!({
        "heart_rate": packet.heart_rate,
        "breath_rate": packet.breath_rate,
        "weight": packet.weight_value,
        "position": packet.position,
        "status": packet.status,
    });

    points.push(DataPoint {
        time: now,
        device_id: Some(device_id),
        patient_id: None, // 从绑定关系获取
        data_type: "mattress_metric".to_string(),
        data_category: DataCategory::Metric,
        value_numeric: Some(packet.heart_rate as f64),
        value_text: None,
        severity: None,
        status: None,
        payload: metric_payload,
        source: "mattress_tcp".to_string(),
    });

    // 2. 状态变化检测
    if state.last_status != Some(packet.status) {
        // 上床/离床事件
        if packet.status == 1 && state.last_status == Some(0) {
            // 上床
            state.on_bed_since = Some(now);
            points.push(create_event(
                device_id, 
                "on_bed", 
                Severity::Info, 
                "用户上床"
            ));
        } else if packet.status == 0 && state.last_status == Some(1) {
            // 离床
            if let Some(since) = state.on_bed_since {
                let duration = now.signed_duration_since(since);
                points.push(create_event(
                    device_id,
                    "off_bed",
                    Severity::Info,
                    &format!("用户离床，卧床时长: {}分钟", duration.num_minutes())
                ));
            }
            state.on_bed_since = None;
        }
        state.last_status = Some(packet.status);
    }

    // 3. 心率异常检测
    if let Some(last_hr) = state.last_heart_rate {
        if packet.heart_rate > 120 && last_hr <= 120 {
            points.push(create_event(
                device_id,
                "heart_rate_high",
                Severity::Warning,
                &format!("心率过高: {}", packet.heart_rate)
            ));
        } else if packet.heart_rate < 50 && last_hr >= 50 {
            points.push(create_event(
                device_id,
                "heart_rate_low",
                Severity::Warning,
                &format!("心率过低: {}", packet.heart_rate)
            ));
        }
    }
    state.last_heart_rate = Some(packet.heart_rate);

    // 4. 呼吸异常检测
    if packet.apnea_count > 0 {
        points.push(create_event(
            device_id,
            "apnea_detected",
            Severity::Alert,
            &format!("检测到呼吸暂停，次数: {}", packet.apnea_count)
        ));
    }

    // 5. 体位变化
    if state.last_position != Some(packet.position) {
        state.last_position = Some(packet.position);
    }

    state.last_event_time = Some(now);
    (points, Some(state))
}

/// 创建事件数据点
fn create_event(device_id: Uuid, event_type: &str, severity: Severity, message: &str) -> DataPoint {
    DataPoint {
        time: chrono::Utc::now(),
        device_id: Some(device_id),
        patient_id: None,
        data_type: event_type.to_string(),
        data_category: DataCategory::Event,
        value_numeric: None,
        value_text: Some(message.to_string()),
        severity: Some(severity),
        status: None,
        payload: serde_json::json!({"message": message}),
        source: "mattress_tcp".to_string(),
    }
}

/// 解析或创建设备 (委托给共享实现)
async fn resolve_or_create_device(pool: &PgPool, serial: &str) -> AppResult<Uuid> {
    crate::ingest::modules::resolve_or_create_device(pool, serial, "smart_mattress", None).await
}

// 辅助函数
fn extract_str(value: &serde_json::Value, keys: &[&str]) -> AppResult<String> {
    for key in keys {
        if let Some(v) = value.get(key).and_then(|v| v.as_str()) {
            return Ok(v.to_string());
        }
    }
    Err(crate::errors::AppError::ValidationError(
        format!("缺少字段: {:?}", keys)
    ))
}

fn extract_u8(value: &serde_json::Value, key: &str) -> Option<u8> {
    value.get(key).and_then(|v| v.as_u64()).map(|v| v as u8)
}

fn extract_u16(value: &serde_json::Value, key: &str) -> Option<u16> {
    value.get(key).and_then(|v| v.as_u64()).map(|v| v as u16)
}
