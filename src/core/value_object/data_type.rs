use serde::{Deserialize, Serialize};
use std::fmt;

/// 数据类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    HeartRate,
    FallEvent,
    SpO2,
    MattressStatus,
    TurnOverEvent,
    BedEntryEvent,
    BedExitEvent,
    SignificantMovementEvent,
    MeasurementSnapshot,
}

impl DataType {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "heart_rate" => Some(Self::HeartRate),
            "fall_event" => Some(Self::FallEvent),
            "spo2" => Some(Self::SpO2),
            "mattress_status" => Some(Self::MattressStatus),
            "turn_over_event" => Some(Self::TurnOverEvent),
            "bed_entry_event" => Some(Self::BedEntryEvent),
            "bed_exit_event" => Some(Self::BedExitEvent),
            "significant_movement_event" => Some(Self::SignificantMovementEvent),
            "measurement_snapshot" => Some(Self::MeasurementSnapshot),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HeartRate => "heart_rate",
            Self::FallEvent => "fall_event",
            Self::SpO2 => "spo2",
            Self::MattressStatus => "mattress_status",
            Self::TurnOverEvent => "turn_over_event",
            Self::BedEntryEvent => "bed_entry_event",
            Self::BedExitEvent => "bed_exit_event",
            Self::SignificantMovementEvent => "significant_movement_event",
            Self::MeasurementSnapshot => "measurement_snapshot",
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
