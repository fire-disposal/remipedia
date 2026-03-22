use crate::config::TcpConfig;
use crate::core::value_object::DeviceType;
use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::AdapterRegistry;
use crate::service::{BindingService, DataService, DeviceService};
use chrono::Utc;
use log::{error, info};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

/// TCP数据接入服务器
pub struct TcpServer {
    config: TcpConfig,
    pool: Arc<PgPool>,
    adapter_registry: Arc<AdapterRegistry>,
}

impl TcpServer {
    pub fn new(config: TcpConfig, pool: Arc<PgPool>) -> Self {
        let adapter_registry = Arc::new(AdapterRegistry::new());
        
        Self {
            config,
            pool,
            adapter_registry,
        }
    }
    
    /// 启动TCP服务器
    pub async fn start(&self) -> AppResult<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        let listener = TcpListener::bind(addr).await
            .map_err(|e| AppError::ConfigError(format!("TCP服务器绑定失败: {}", e)))?;
        
        info!("TCP服务器启动，监听端口: {}", self.config.port);
        
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("接受新的TCP连接: {}", addr);
                    
                    let pool = self.pool.clone();
                    let adapter_registry = self.adapter_registry.clone();
                    
                    // 处理每个连接
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, addr, pool, adapter_registry).await {
                            error!("处理TCP连接失败 {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("接受TCP连接失败: {}", e);
                }
            }
        }
    }
    
    /// 处理单个TCP连接
    async fn handle_connection(
        mut stream: TcpStream,
        addr: SocketAddr,
        pool: Arc<PgPool>,
        adapter_registry: Arc<AdapterRegistry>,
    ) -> AppResult<()> {
        let mut buffer = vec![0u8; 4096]; // 4KB缓冲区
        let mut remaining_data = Vec::new();
        
        info!("开始处理TCP连接: {}", addr);
        
        loop {
            // 读取数据
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    info!("TCP连接关闭: {}", addr);
                    break;
                }
                Ok(n) => {
                    remaining_data.extend_from_slice(&buffer[..n]);
                    
                    // 处理完整的数据包
                    while let Some(packet) = Self::extract_packet(&mut remaining_data)? {
                        if let Err(e) = Self::process_packet(&stream, &pool, &adapter_registry, packet).await {
                            error!("处理数据包失败: {}", e);
                            // 继续处理其他数据包，不中断连接
                        }
                    }
                }
                Err(e) => {
                    return Err(AppError::ConfigError(format!("读取TCP数据失败: {}", e)));
                }
            }
        }
        
        Ok(())
    }
    
    /// 提取完整的数据包
    pub fn extract_packet(buffer: &mut Vec<u8>) -> AppResult<Option<Vec<u8>>> {
        if buffer.len() < 4 {
            return Ok(None); // 数据不足，等待更多数据
        }
        
        // 验证魔数
        if buffer[0] != 0xab || buffer[1] != 0xcd {
            // 跳过无效数据，寻找下一个魔数
            for i in 1..buffer.len() - 1 {
                if buffer[i] == 0xab && buffer[i + 1] == 0xcd {
                    buffer.drain(..i);
                    return Self::extract_packet(buffer);
                }
            }
            
            // 没有找到有效的魔数，清空缓冲区
            buffer.clear();
            return Ok(None);
        }
        
        let data_len = buffer[2] as usize;
        let total_len = data_len + 4;
        
        if buffer.len() < total_len {
            return Ok(None); // 数据不足，等待更多数据
        }
        
        // 提取完整的数据包
        let packet = buffer[..total_len].to_vec();
        buffer.drain(..total_len);
        
        Ok(Some(packet))
    }
    
    /// 处理数据包
    async fn process_packet(
        _stream: &TcpStream,
        pool: &Arc<PgPool>,
        adapter_registry: &Arc<AdapterRegistry>,
        packet: Vec<u8>,
    ) -> AppResult<()> {
        // 获取智能床垫适配器
        let adapter = adapter_registry.get(&DeviceType::SmartMattress)
            .ok_or_else(|| AppError::ValidationError("找不到智能床垫适配器".to_string()))?;
        
        // 解析数据
        let payload = adapter.parse_payload(&packet)?;
        
        // 验证数据
        adapter.validate(&payload)?;
        
        // 获取设备序列号
        let serial_number = payload["serial_number"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少序列号".into()))?;
        
        // 创建设备服务
        let device_service = DeviceService::new(pool);
        let binding_service = BindingService::new(pool);
        let data_service = DataService::new(pool);
        
        // 自动注册设备（如果不存在）
        let device = device_service.auto_register_or_get(serial_number, "smart_mattress").await?;
        
        // 获取绑定的患者
        let subject_id = binding_service.get_current_binding_subject(&device.id).await?;
        
        // 准备数据入库
        let _ingest_data = crate::core::entity::IngestData {
            time: Utc::now(),
            device_id: device.id,
            subject_id,
            data_type: adapter.data_type().to_string(),
            payload: payload.clone(),
            source: "tcp".to_string(),
        };
        
        // 处理智能过滤后的事件，只存储有价值的数据
        if let Some(events) = payload["valuable_events"].as_array() {
            let mut event_count = 0;
            
            for event_data in events {
                let event_type = event_data["type"].as_str().unwrap_or("unknown");
                let data_type = match event_type {
                    "BedEntry" => "bed_entry_event",
                    "BedExit" => "bed_exit_event", 
                    "SignificantMovement" => "significant_movement_event",
                    "MeasurementSnapshot" => "measurement_snapshot",
                    _ => continue,
                };
                
                let event_ingest_data = crate::core::entity::IngestData {
                    time: Utc::now(),
                    device_id: device.id,
                    subject_id,
                    data_type: data_type.to_string(),
                    payload: event_data.clone(),
                    source: "tcp".to_string(),
                };
                
                data_service.ingest(event_ingest_data).await?;
                event_count += 1;
            }
            
            info!("智能床垫有价值事件入库成功: device_id={}, events={}, subject_id={:?}", 
                  device.id, event_count, subject_id);
        }
        
        // 可选：存储原始状态数据（降低频率）
        if should_save_raw_data(&payload) {
            let raw_data = crate::core::entity::IngestData {
                time: Utc::now(),
                device_id: device.id,
                subject_id,
                data_type: "mattress_status_raw".to_string(),
                payload: serde_json::json!({
                    "status": payload["status"],
                    "heart_rate": payload["heart_rate"],
                    "breath_rate": payload["breath_rate"],
                    "weight_value": payload["weight_value"],
                    "timestamp": Utc::now(),
                }),
                source: "tcp".to_string(),
            };
            
            data_service.ingest(raw_data).await?;
            info!("智能床垫原始数据存档: device_id={}", device.id);
        }
        
        Ok(())
    }
}

/// 判断是否应该保存原始数据（降低存储频率）
fn should_save_raw_data(_payload: &serde_json::Value) -> bool {
    // 每30秒保存一次原始数据，或者状态发生变化时
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let current_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // 简单的基于时间戳的判断，实际应用中可以使用更复杂的逻辑
    current_seconds % 30 == 0
}