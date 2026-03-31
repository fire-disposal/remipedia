//! 设备接入框架
//! 
//! 提供统一的设备接入、状态管理和事件处理接口

use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tokio::sync::RwLock;
use sqlx::PgPool;

use crate::errors::{AppError, AppResult};
use crate::core::entity::DataPoint;
use crate::core::value_object::DeviceType;
use crate::repository::{BindingRepository, DeviceRepository};
use crate::service::DataService;

/// 设备元信息
#[derive(Debug, Clone, Serialize)]
pub struct DeviceMetadata {
    pub device_type: DeviceType,
    pub display_name: String,
    pub description: String,
    pub supported_data_types: Vec<String>,
    pub protocol_version: String,
}

/// 统一消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub time: DateTime<Utc>,
    pub data_type: String,
    pub message_type: Option<String>,
    pub severity: Option<String>,
    pub payload: Value,
}

/// 适配器输出
#[derive(Debug)]
pub enum AdapterOutput {
    Messages(Vec<MessagePayload>),
    Empty,
}

/// 设备适配器 trait
/// 
/// 负责解析原始数据并验证
pub trait DeviceAdapter: Send + Sync {
    /// 获取设备元信息
    fn metadata(&self) -> DeviceMetadata;
    
    /// 解析原始数据
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput>;
    
    /// 验证解析后的输出
    fn validate(&self, output: &AdapterOutput) -> AppResult<()>;
    
    /// 获取设备类型
    fn device_type(&self) -> DeviceType {
        self.metadata().device_type
    }
}

/// 设备状态 trait
/// 
/// 管理设备特定状态
pub trait DeviceState: Send + Sync {
    /// 更新状态
    fn update(&mut self, data: &MessagePayload) -> AppResult<()>;
    
    /// 获取状态快照
    fn snapshot(&self) -> Value;
    
    /// 重置状态
    fn reset(&mut self);
}

/// 设备事件
#[derive(Debug, Clone, Serialize)]
pub struct DeviceEvent {
    pub device_id: String,
    pub device_type: DeviceType,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub severity: Option<String>,
    pub payload: Value,
}

/// 设备实例
pub struct DeviceInstance {
    pub device_id: String,
    pub device_type: DeviceType,
    pub serial_number: String,
    pub last_seen: DateTime<Utc>,
    pub adapter: Arc<dyn DeviceAdapter>,
    pub state: Option<Box<dyn DeviceState>>,
}

impl DeviceInstance {
    pub fn new(
        device_id: String,
        device_type: DeviceType,
        serial_number: String,
        adapter: Arc<dyn DeviceAdapter>,
        state: Option<Box<dyn DeviceState>>,
    ) -> Self {
        Self {
            device_id,
            device_type,
            serial_number,
            last_seen: Utc::now(),
            adapter,
            state,
        }
    }
    
    /// 处理原始数据
    pub fn process(&mut self, raw: &[u8]) -> AppResult<AdapterOutput> {
        // 更新最后活跃时间
        self.last_seen = Utc::now();
        
        // 解析数据
        let output = self.adapter.parse(raw)?;
        
        // 验证数据
        self.adapter.validate(&output)?;
        
        Ok(output)
    }
    
    /// 检查是否空闲
    pub fn is_idle(&self, timeout: chrono::Duration) -> bool {
        let elapsed = Utc::now() - self.last_seen;
        elapsed > timeout
    }
}

/// 设备管理器
/// 
/// 管理所有接入的设备实例
pub struct DeviceManager {
    devices: RwLock<HashMap<String, DeviceInstance>>,
    adapters: RwLock<HashMap<DeviceType, Arc<dyn DeviceAdapter>>>,
    pool: Arc<PgPool>,
    idle_timeout: chrono::Duration,
}

impl DeviceManager {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            devices: RwLock::new(HashMap::new()),
            adapters: RwLock::new(HashMap::new()),
            pool,
            idle_timeout: chrono::Duration::minutes(30),
        }
    }
    
    /// 注册适配器
    pub async fn register_adapter(&self, adapter: Arc<dyn DeviceAdapter>) {
        let device_type = adapter.device_type();
        let mut adapters = self.adapters.write().await;
        adapters.insert(device_type, adapter);
    }
    
    /// 获取适配器
    pub async fn get_adapter(&self, device_type: &DeviceType) -> Option<Arc<dyn DeviceAdapter>> {
        let adapters = self.adapters.read().await;
        adapters.get(device_type).cloned()
    }
    
    /// 处理设备数据
    pub async fn process(
        &self,
        serial_number: &str,
        device_type: DeviceType,
        raw: Vec<u8>,
        source: &str,
    ) -> AppResult<ProcessResult> {
        let mut devices = self.devices.write().await;

        // 获取或创建设备实例
        let device = match devices.get_mut(serial_number) {
            Some(d) => d,
            None => {
                // 获取适配器
                let adapter = {
                    let adapters = self.adapters.read().await;
                    adapters.get(&device_type)
                        .cloned()
                        .ok_or_else(|| AppError::ValidationError(format!("未找到设备类型 {} 的适配器", device_type)))?
                };

                // 创建新设备实例
                let new_device = DeviceInstance::new(
                    serial_number.to_string(),
                    device_type,
                    serial_number.to_string(),
                    adapter,
                    None, // 状态可以后续添加
                );

                // 安全地插入并获取可变引用
                devices.insert(serial_number.to_string(), new_device);
                devices.get_mut(serial_number)
                    .ok_or_else(|| AppError::InternalError)?
            }
        };
        
        // 处理数据
        let output = device.process(&raw)?;
        
        // 入库
        let data_service = DataService::new(&self.pool);
        let mut persisted = 0;
        let mut events = 0;
        let mut errors = Vec::new();
        
        if let AdapterOutput::Messages(msgs) = output {
            for msg in msgs {
                if msg.message_type.is_some() {
                    events += 1;
                }
                
                // 从数据库获取设备信息和绑定
                let device_repo = DeviceRepository::new(&self.pool);
                let binding_repo = BindingRepository::new(&self.pool);
                
                let (device_uuid, patient_id) = match device_repo.find_by_serial(&device.serial_number).await {
                    Ok(Some(device)) => {
                        // 查找当前绑定
                        let patient = binding_repo.find_active_by_device(&device.id).await.ok().flatten();
                        (Some(device.id), patient.map(|b| b.patient_id))
                    }
                    _ => (None, None),
                };
                
                let datapoint = DataPoint {
                    time: msg.time,
                    device_id: device_uuid,
                    patient_id,
                    data_type: msg.data_type.clone(),
                    data_category: crate::core::entity::DataCategory::Metric,
                    value_numeric: None,
                    value_text: None,
                    severity: msg.severity.and_then(|s| s.parse().ok()),
                    status: None,
                    payload: msg.payload,
                    source: source.to_string(),
                };
                
                match data_service.ingest(
                    datapoint.device_id,
                    datapoint.patient_id,
                    datapoint.data_type,
                    datapoint.payload,
                ).await {
                    Ok(_) => persisted += 1,
                    Err(e) => {
                        log::error!("入库失败: {}", e);
                        errors.push(format!("{}", e));
                    }
                }
            }
        }
        
        log::debug!("处理完成: {} -> {}条, {}事件", serial_number, persisted, events);
        
        Ok(ProcessResult {
            serial_number: serial_number.to_string(),
            device_type: device_type.to_string(),
            persisted,
            events,
            errors,
        })
    }
    
    /// 清理空闲设备
    pub async fn cleanup_idle(&self) -> usize {
        let mut devices = self.devices.write().await;
        let before = devices.len();
        
        devices.retain(|_, device| !device.is_idle(self.idle_timeout));
        
        
        before - devices.len()
    }
    
    /// 获取所有设备信息
    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;
        
        devices.values().map(|device| DeviceInfo {
                serial_number: device.serial_number.clone(),
                device_type: device.device_type.to_string(),
                last_seen: device.last_seen,
                is_idle: device.is_idle(self.idle_timeout),
            })
            .collect()
    }
    
    /// 获取设备数量
    pub async fn device_count(&self) -> usize {
        let devices = self.devices.read().await;
        devices.len()
    }
    
    /// 获取支持的设备类型列表
    pub async fn supported_device_types(&self) -> Vec<DeviceMetadata> {
        let adapters = self.adapters.read().await;
        adapters.values().map(|a| a.metadata()).collect()
    }
}

/// 处理结果
#[derive(Debug)]
pub struct ProcessResult {
    pub serial_number: String,
    pub device_type: String,
    pub persisted: usize,
    pub events: usize,
    pub errors: Vec<String>,
}

/// 设备信息
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub serial_number: String,
    pub device_type: String,
    pub last_seen: DateTime<Utc>,
    pub is_idle: bool,
}

/// 适配器注册表
pub struct AdapterRegistry {
    adapters: HashMap<DeviceType, Arc<dyn DeviceAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, adapter: Arc<dyn DeviceAdapter>) {
        let device_type = adapter.device_type();
        log::info!("注册适配器: {:?}", device_type);
        self.adapters.insert(device_type, adapter);
    }
    
    pub fn get(&self, device_type: &DeviceType) -> Option<Arc<dyn DeviceAdapter>> {
        self.adapters.get(device_type).cloned()
    }
    
    pub fn list(&self) -> Vec<(DeviceType, DeviceMetadata)> {
        self.adapters
            .iter()
            .map(|(k, v)| (*k, v.metadata()))
            .collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
