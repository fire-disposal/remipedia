//! 设备接入框架
//!
//! 提供统一的设备接入、状态管理和事件处理接口

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::application::healthdata::HealthDataAppService;
use crate::core::domain::device::DeviceRepository;
use crate::core::domain::healthdata::{DataSource, DataType};
use crate::core::domain::shared::DeviceId;
use crate::core::value_object::DeviceTypeId;
use crate::errors::{AppError, AppResult};
use crate::infrastructure::persistence::SqlxDeviceRepository;

/// 设备元信息
#[derive(Debug, Clone, Serialize)]
pub struct DeviceMetadata {
    pub device_type: DeviceTypeId,
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
    fn device_type(&self) -> DeviceTypeId {
        self.metadata().device_type.clone()
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
    pub device_type: DeviceTypeId,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub severity: Option<String>,
    pub payload: Value,
}

/// 设备实例
pub struct DeviceInstance {
    pub device_id: String,
    pub device_type: DeviceTypeId,
    pub serial_number: String,
    pub last_seen: DateTime<Utc>,
    pub adapter: Arc<dyn DeviceAdapter>,
    pub state: Option<Box<dyn DeviceState>>,
}

impl DeviceInstance {
    pub fn new(
        device_id: String,
        device_type: DeviceTypeId,
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

    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    pub fn is_idle(&self, timeout: chrono::Duration) -> bool {
        Utc::now() - self.last_seen > timeout
    }

    /// 处理原始数据
    pub fn process(&mut self, raw: &[u8]) -> AppResult<AdapterOutput> {
        let output = self.adapter.parse(raw)?;
        self.adapter.validate(&output)?;

        // 更新状态
        if let Some(ref mut state) = self.state {
            if let AdapterOutput::Messages(ref msgs) = output {
                for msg in msgs {
                    state.update(msg)?;
                }
            }
        }

        self.touch();
        Ok(output)
    }

    /// 获取状态快照
    pub fn state_snapshot(&self) -> Option<Value> {
        self.state.as_ref().map(|s| s.snapshot())
    }
}

/// 设备管理器
pub struct DeviceManager {
    devices: RwLock<HashMap<String, DeviceInstance>>,
    adapters: RwLock<HashMap<DeviceTypeId, Arc<dyn DeviceAdapter>>>,
    pool: Arc<sqlx::PgPool>,
    idle_timeout: chrono::Duration,
}

impl DeviceManager {
    pub fn new(pool: Arc<sqlx::PgPool>) -> Self {
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
    pub async fn get_adapter(
        &self,
        device_type: &DeviceTypeId,
    ) -> Option<Arc<dyn DeviceAdapter>> {
        let adapters = self.adapters.read().await;
        adapters.get(device_type).cloned()
    }

    /// 处理设备数据
    pub async fn process(
        &self,
        serial_number: &str,
        device_type: DeviceTypeId,
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
                    adapters
                        .get(&device_type)
                        .cloned()
                        .ok_or_else(|| {
                            AppError::ValidationError(format!(
                                "未找到设备类型 {} 的适配器",
                                device_type
                            ))
                        })?
                };

                // 查询设备真实 ID
                let device_repo = SqlxDeviceRepository::new(&self.pool);
                let device_id = match device_repo
                    .find_by_serial(serial_number)
                    .await
                {
                    Ok(Some(device)) => device.id().clone(),
                    _ => {
                        log::warn!("未找到序列号 {} 对应的设备，使用 nil UUID", serial_number);
                        DeviceId::from_uuid(uuid::Uuid::nil())
                    }
                };

                // 创建新设备实例
                let new_device = DeviceInstance::new(
                    device_id.as_uuid().to_string(),
                    device_type.clone(),
                    serial_number.to_string(),
                    adapter,
                    None, // 状态可以后续添加
                );

                devices.insert(serial_number.to_string(), new_device);
                devices.get_mut(serial_number).unwrap()
            }
        };

        // 处理数据
        let output = device.process(&raw)?;

        // 使用应用服务保存数据
        let mut persisted = 0;
        let mut events = 0;

        if let AdapterOutput::Messages(msgs) = output {
            // 获取设备 ID
            let device_id = DeviceId::from_uuid(
                uuid::Uuid::parse_str(&device.device_id).unwrap_or_else(|_| uuid::Uuid::nil()),
            );

            // 创建健康数据服务
            let health_service = HealthDataAppService::new(&self.pool);

            for msg in msgs {
                if msg.message_type.is_some() {
                    events += 1;
                }

                // 将字符串 data_type 转换为 DataType 枚举
                let data_type = match msg.data_type.as_str() {
                    "smart_mattress" => DataType::MattressStatus,
                    "heart_rate" => DataType::HeartRate,
                    "spo2" => DataType::SpO2,
                    "blood_pressure" => DataType::BloodPressure,
                    "temperature" => DataType::Temperature,
                    "fall_event" => DataType::FallEvent,
                    _ => DataType::Custom(msg.data_type.clone()),
                };

                // 使用应用服务保存数据
                let source_enum = match source {
                    "tcp" => DataSource::Tcp,
                    "mqtt" => DataSource::Mqtt,
                    "http" => DataSource::Http,
                    "websocket" => DataSource::WebSocket,
                    _ => DataSource::Internal,
                };

                match health_service
                    .ingest_data(device_id.clone(), data_type, msg.payload, source_enum)
                    .await
                {
                    Ok(_) => persisted += 1,
                    Err(e) => log::error!("保存健康数据失败: {}", e),
                }
            }
        }

        log::debug!(
            "处理完成: {} -> {}条, {}事件",
            serial_number,
            persisted,
            events
        );

        Ok(ProcessResult {
            serial_number: serial_number.to_string(),
            device_type: device_type.to_string(),
            persisted,
            events,
            errors: vec![],
        })
    }

    /// 清理空闲设备
    pub async fn cleanup_idle(&self) -> usize {
        let mut devices = self.devices.write().await;
        let before = devices.len();

        devices.retain(|_, device| !device.is_idle(self.idle_timeout));

        let removed = before - devices.len();
        removed
    }

    /// 获取所有设备信息
    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;

        devices
            .iter()
            .map(|(_id, device)| DeviceInfo {
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
    adapters: HashMap<DeviceTypeId, Arc<dyn DeviceAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// 注册适配器
    pub fn register(&mut self, adapter: Arc<dyn DeviceAdapter>) {
        let device_type = adapter.device_type();
        self.adapters.insert(device_type, adapter);
    }

    /// 获取适配器
    pub fn get(&self, device_type: &DeviceTypeId) -> Option<Arc<dyn DeviceAdapter>> {
        self.adapters.get(device_type).cloned()
    }

    /// 检查是否支持该设备类型
    pub fn is_supported(&self, device_type: &DeviceTypeId) -> bool {
        self.adapters.contains_key(device_type)
    }

    /// 获取所有支持的设备类型
    pub fn supported_types(&self) -> Vec<DeviceTypeId> {
        self.adapters.keys().cloned().collect()
    }

    /// 获取所有设备元信息
    pub fn all_metadata(&self) -> Vec<DeviceMetadata> {
        self.adapters.values().map(|a| a.metadata()).collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
