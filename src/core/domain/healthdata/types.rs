//! 健康数据类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::domain::shared::DeviceId;

/// 健康数据实体（领域层）
#[derive(Debug, Clone)]
pub struct HealthData {
    id: Uuid,
    time: DateTime<Utc>,
    device_id: DeviceId,
    subject_id: Option<Uuid>,
    data_type: DataType,
    payload: serde_json::Value,
    source: DataSource,
    quality: DataQuality,
    ingested_at: DateTime<Utc>,
}

/// 数据类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataType {
    HeartRate,
    SpO2,
    BloodPressure,
    Temperature,
    FallEvent,
    MattressStatus,
    Sleep,
    Activity,
    TurnOverEvent,
    BedEntryEvent,
    BedExitEvent,
    SignificantMovementEvent,
    MeasurementSnapshot,
    Custom(String),
}

/// 数据来源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Mqtt,
    Http,
    Tcp,
    WebSocket,
    Internal,
}

/// 数据质量
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataQuality {
    Good,
    Fair,
    Poor,
    Invalid,
}

impl HealthData {
    /// 创建健康数据
    pub fn create(
        time: DateTime<Utc>,
        device_id: DeviceId,
        subject_id: Option<Uuid>,
        data_type: DataType,
        payload: serde_json::Value,
        source: DataSource,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            time,
            device_id,
            subject_id,
            data_type,
            payload,
            source,
            quality: DataQuality::Good,
            ingested_at: Utc::now(),
        }
    }

    /// 从持久化重建
    pub fn reconstruct(
        id: Uuid,
        time: DateTime<Utc>,
        device_id: DeviceId,
        subject_id: Option<Uuid>,
        data_type: DataType,
        payload: serde_json::Value,
        source: DataSource,
        quality: DataQuality,
        ingested_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            time,
            device_id,
            subject_id,
            data_type,
            payload,
            source,
            quality,
            ingested_at,
        }
    }

    /// 设置数据质量
    pub fn set_quality(&mut self, quality: DataQuality) {
        self.quality = quality;
    }

    /// 关联到患者
    pub fn assign_to_subject(&mut self, subject_id: Uuid) {
        self.subject_id = Some(subject_id);
    }

    // Getters
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub fn subject_id(&self) -> Option<Uuid> {
        self.subject_id
    }

    pub fn data_type(&self) -> &DataType {
        &self.data_type
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    pub fn source(&self) -> DataSource {
        self.source
    }

    pub fn quality(&self) -> DataQuality {
        self.quality
    }

    pub fn ingested_at(&self) -> DateTime<Utc> {
        self.ingested_at
    }
}

impl DataType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::HeartRate => "heart_rate",
            Self::SpO2 => "spo2",
            Self::BloodPressure => "blood_pressure",
            Self::Temperature => "temperature",
            Self::FallEvent => "fall_event",
            Self::MattressStatus => "mattress_status",
            Self::Sleep => "sleep",
            Self::Activity => "activity",
            Self::TurnOverEvent => "turn_over_event",
            Self::BedEntryEvent => "bed_entry_event",
            Self::BedExitEvent => "bed_exit_event",
            Self::SignificantMovementEvent => "significant_movement_event",
            Self::MeasurementSnapshot => "measurement_snapshot",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "heart_rate" => Self::HeartRate,
            "spo2" => Self::SpO2,
            "blood_pressure" => Self::BloodPressure,
            "temperature" => Self::Temperature,
            "fall_event" => Self::FallEvent,
            "mattress_status" => Self::MattressStatus,
            "sleep" => Self::Sleep,
            "activity" => Self::Activity,
            "turn_over_event" => Self::TurnOverEvent,
            "bed_entry_event" => Self::BedEntryEvent,
            "bed_exit_event" => Self::BedExitEvent,
            "significant_movement_event" => Self::SignificantMovementEvent,
            "measurement_snapshot" => Self::MeasurementSnapshot,
            _ => Self::Custom(s.to_string()),
        }
    }

    /// 是否为事件类型（而非连续测量）
    pub fn is_event(&self) -> bool {
        matches!(
            self,
            Self::FallEvent
                | Self::TurnOverEvent
                | Self::BedEntryEvent
                | Self::BedExitEvent
                | Self::SignificantMovementEvent
        )
    }

    /// 是否为生命体征数据
    pub fn is_vital_sign(&self) -> bool {
        matches!(
            self,
            Self::HeartRate | Self::SpO2 | Self::BloodPressure | Self::Temperature
        )
    }
}

impl DataSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Mqtt => "mqtt",
            Self::Http => "http",
            Self::Tcp => "tcp",
            Self::WebSocket => "websocket",
            Self::Internal => "internal",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "mqtt" => Self::Mqtt,
            "http" => Self::Http,
            "tcp" => Self::Tcp,
            "websocket" => Self::WebSocket,
            "internal" => Self::Internal,
            _ => Self::Internal,
        }
    }
}

impl DataQuality {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Good => "good",
            Self::Fair => "fair",
            Self::Poor => "poor",
            Self::Invalid => "invalid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "good" => Self::Good,
            "fair" => Self::Fair,
            "poor" => Self::Poor,
            "invalid" => Self::Invalid,
            _ => Self::Good,
        }
    }
}

impl Default for DataQuality {
    fn default() -> Self {
        Self::Good
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_conversion() {
        assert_eq!(DataType::HeartRate.as_str(), "heart_rate");
        assert_eq!(DataType::from_str("heart_rate"), DataType::HeartRate);
        assert_eq!(
            DataType::from_str("custom_type"),
            DataType::Custom("custom_type".to_string())
        );
    }

    #[test]
    fn test_data_type_classification() {
        assert!(DataType::FallEvent.is_event());
        assert!(!DataType::HeartRate.is_event());
        
        assert!(DataType::HeartRate.is_vital_sign());
        assert!(!DataType::FallEvent.is_vital_sign());
    }

    #[test]
    fn test_health_data_create() {
        let data = HealthData::create(
            Utc::now(),
            DeviceId::new(),
            None,
            DataType::HeartRate,
            serde_json::json!({"bpm": 72}),
            DataSource::Mqtt,
        );
        
        assert!(data.subject_id().is_none());
        assert_eq!(data.quality(), DataQuality::Good);
    }

    #[test]
    fn test_assign_subject() {
        let mut data = HealthData::create(
            Utc::now(),
            DeviceId::new(),
            None,
            DataType::HeartRate,
            serde_json::json!({}),
            DataSource::Mqtt,
        );
        
        let subject_id = Uuid::now_v7();
        data.assign_to_subject(subject_id);
        assert_eq!(data.subject_id(), Some(subject_id));
    }
}
