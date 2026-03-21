use std::fmt;
use serde::{Deserialize, Serialize};

/// 数据类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    HeartRate,
    FallEvent,
    SpO2,
}

impl DataType {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "heart_rate" => Some(Self::HeartRate),
            "fall_event" => Some(Self::FallEvent),
            "spo2" => Some(Self::SpO2),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HeartRate => "heart_rate",
            Self::FallEvent => "fall_event",
            Self::SpO2 => "spo2",
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}