//! 床垫设备 TCP Msgpack 模块
//!
//! 独立模块：监听TCP端口，处理床垫设备的Msgpack协议数据
//! 包含：TCP监听 + 帧解码 + 状态管理 + 事件检测

use crate::core::entity::{DataPoint, DataCategory, Severity};
use crate::errors::AppResult;
use crate::repository::{DataRepository, RawDataRepository};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use uuid::Uuid;

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
    last_status: Option<u8>,
    last_position: Option<u8>,
    last_heart_rate: Option<u8>,
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
            match extract_msgpack_frame(&mut buffer, max_frame_size) {
                Ok(Some(frame)) => {
                    // 归档原始数据
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
                    log::warn!("帧提取错误 {}: {}", addr, e);
                    // 尝试恢复：丢弃到下一个magic头
                    if let Some(pos) = find_next_magic(&buffer[1..]) {
                        buffer.drain(..pos + 1);
                    } else {
                        buffer.clear();
                    }
                }
            }
        }
    }

    Ok(())
}

/// 提取Msgpack帧: [0xAB, 0xCD, len, crc, data...]
fn extract_msgpack_frame(buffer: &mut Vec<u8>, max_size: usize) -> AppResult<Option<Vec<u8>>> {
    if buffer.len() < 4 {
        return Ok(None);
    }

    // 查找magic头
    if buffer[0] != 0xAB || buffer[1] != 0xCD {
        return Err(crate::errors::AppError::ValidationError(
            format!("无效的magic头: {:02X} {:02X}", buffer[0], buffer[1])
        ));
    }

    let data_len = buffer[2] as usize;
    if data_len > max_size {
        return Err(crate::errors::AppError::ValidationError(
            format!("帧长度 {} 超过最大值 {}", data_len, max_size)
        ));
    }

    let total_len = 4 + data_len;
    if buffer.len() < total_len {
        return Ok(None); // 需要更多数据
    }

    // CRC校验 (可选)
    let _expected_crc = buffer[3];
    let _data = &buffer[4..4 + data_len];
    // TODO: CRC校验

    let frame = buffer[..total_len].to_vec();
    buffer.drain(..total_len);
    Ok(Some(frame))
}

/// 查找下一个magic头位置
fn find_next_magic(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|w| w == [0xAB, 0xCD])
}

/// 解析床垫数据包
fn parse_mattress_packet(frame: &[u8]) -> AppResult<MattressPacket> {
    if frame.len() < 5 {
        return Err(crate::errors::AppError::ValidationError("数据包太短".into()));
    }

    let data = &frame[4..];
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

/// 解析或创建设备
async fn resolve_or_create_device(pool: &PgPool, serial: &str) -> AppResult<Uuid> {
    use crate::repository::DeviceRepository;
    use crate::core::entity::NewDevice;

    let repo = DeviceRepository::new(pool);
    
    // 尝试查找
    if let Some(device) = repo.find_by_serial(serial).await? {
        return Ok(device.id);
    }
    
    // 自动创建设备
    let new_device = NewDevice {
        serial_number: serial.to_string(),
        device_type: "smart_mattress".to_string(),
        status: "active".to_string(),
        firmware_version: None,
        metadata: None,
    };
    
    let device = repo.insert(&new_device).await?;
    log::info!("自动注册床垫设备: {} -> {}", serial, device.id);
    Ok(device.id)
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
