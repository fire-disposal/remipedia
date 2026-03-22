use std::fmt;
use serde::{Deserialize, Serialize};

/// 设备类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    HeartRateMonitor,
    FallDetector,
    SpO2Sensor,
    SmartMattress,
}

impl DeviceType {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "heart_rate_monitor" => Some(Self::HeartRateMonitor),
            "fall_detector" => Some(Self::FallDetector),
            "spo2_sensor" => Some(Self::SpO2Sensor),
            "smart_mattress" => Some(Self::SmartMattress),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HeartRateMonitor => "heart_rate_monitor",
            Self::FallDetector => "fall_detector",
            Self::SpO2Sensor => "spo2_sensor",
            Self::SmartMattress => "smart_mattress",
        }
    }

    /// 获取所有设备类型
    pub fn all() -> &'static [Self] {
        &[Self::HeartRateMonitor, Self::FallDetector, Self::SpO2Sensor, Self::SmartMattress]
    }
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}