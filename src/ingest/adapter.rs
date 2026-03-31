//! 设备适配器 trait 定义

use crate::core::entity::DataPoint;
use crate::errors::AppResult;
use crate::ingest::ParsedData;
use crate::ingest::protocol::ProtocolDecoder;
use crate::ingest::state::DeviceState;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 设备类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    SmartMattress,
    HeartRateMonitor,
    FallDetector,
    BloodPressureMonitor,
    GlucoseMeter,
    Other(String),
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::SmartMattress => write!(f, "smart_mattress"),
            DeviceType::HeartRateMonitor => write!(f, "heart_rate_monitor"),
            DeviceType::FallDetector => write!(f, "fall_detector"),
            DeviceType::BloodPressureMonitor => write!(f, "blood_pressure_monitor"),
            DeviceType::GlucoseMeter => write!(f, "glucose_meter"),
            DeviceType::Other(s) => write!(f, "{}", s),
        }
    }
}

impl DeviceType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "smart_mattress" => Some(DeviceType::SmartMattress),
            "heart_rate_monitor" => Some(DeviceType::HeartRateMonitor),
            "fall_detector" => Some(DeviceType::FallDetector),
            "blood_pressure_monitor" => Some(DeviceType::BloodPressureMonitor),
            "glucose_meter" => Some(DeviceType::GlucoseMeter),
            _ => Some(DeviceType::Other(s.to_string())),
        }
    }
}

/// 设备元信息
#[derive(Debug, Clone)]
pub struct DeviceMetadata {
    pub device_type: DeviceType,
    pub display_name: String,
    pub description: String,
    pub protocol_version: String,
    pub supports_events: bool,
}

impl Default for DeviceMetadata {
    fn default() -> Self {
        Self {
            device_type: DeviceType::Other("unknown".to_string()),
            display_name: "Unknown Device".to_string(),
            description: "".to_string(),
            protocol_version: "1.0".to_string(),
            supports_events: false,
        }
    }
}

/// 设备适配器 trait - 所有设备必须实现
#[async_trait]
pub trait DeviceAdapter: Send + Sync {
    /// 获取设备元信息
    fn metadata(&self) -> DeviceMetadata;

    /// 是否是有状态适配器（需要状态管理）
    fn is_stateful(&self) -> bool {
        false
    }

    /// 协议解码器（可选，用于TCP粘包等）
    fn protocol_decoder(&self) -> Option<&dyn ProtocolDecoder> {
        None
    }

    /// 创建初始状态（有状态适配器必须实现）
    fn create_state(&self) -> Option<Box<dyn DeviceState>> {
        None
    }

    /// 解析原始数据为结构化数据
    ///
    /// # Arguments
    /// * `raw` - 原始字节数据
    ///
    /// # Returns
    /// * `ParsedData` - 解析后的结构化数据
    fn parse(&self, raw: &[u8]) -> AppResult<ParsedData>;

    /// 处理数据（无状态适配器）
    ///
    /// 默认实现：直接转换为单个DataPoint
    async fn process(&self, data: ParsedData) -> AppResult<Vec<DataPoint>> {
        let point = DataPoint::metric(
            None, // device_id将在pipeline中填充
            None, // patient_id将在pipeline中填充
            data.device_type,
            0.0, // 数值型数据可以在这里提取
            data.payload,
        );
        Ok(vec![point])
    }

    /// 处理数据（有状态适配器）
    ///
    /// 有状态适配器需要重写此方法，在方法内部使用state
    async fn process_with_state(
        &self,
        data: ParsedData,
        _state: &mut dyn DeviceState,
    ) -> AppResult<Vec<DataPoint>> {
        // 默认行为：忽略state，调用无状态版本
        self.process(data).await
    }
}

/// 适配器注册表
pub struct AdapterRegistry {
    adapters: std::collections::HashMap<DeviceType, Box<dyn DeviceAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn DeviceAdapter>) {
        let device_type = adapter.metadata().device_type;
        log::info!("注册适配器: {:?}", device_type);
        self.adapters.insert(device_type, adapter);
    }

    pub fn get(&self, device_type: &DeviceType) -> Option<&dyn DeviceAdapter> {
        self.adapters.get(device_type).map(|a| a.as_ref())
    }

    pub fn list(&self) -> Vec<DeviceMetadata> {
        self.adapters.values().map(|a| a.metadata()).collect()
    }

    pub fn supported_types(&self) -> Vec<DeviceType> {
        self.adapters.keys().cloned().collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
