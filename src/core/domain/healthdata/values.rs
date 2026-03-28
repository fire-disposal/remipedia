//! 健康数据值对象

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


/// 基础测量数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
}

/// 心率数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateData {
    pub bpm: u32,
    pub confidence: Option<u8>,
}

/// 血氧数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpO2Data {
    pub percentage: f32,
    pub confidence: Option<u8>,
}

/// 血压数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodPressureData {
    pub systolic: u32,
    pub diastolic: u32,
    pub pulse: Option<u32>,
}

/// 体温数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureData {
    pub celsius: f32,
    pub location: TemperatureLocation,
}

/// 体温测量部位
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureLocation {
    Oral,
    Axillary,
    Rectal,
    Tympanic,
    Forehead,
    Other,
}

/// 跌倒检测数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallDetectionData {
    pub detected: bool,
    pub confidence: f32,
    pub severity: FallSeverity,
}

/// 跌倒严重程度
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// 床垫传感器数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattressSensorData {
    pub heart_rate: Option<u32>,
    pub breath_rate: Option<u32>,
    pub is_on_bed: bool,
    pub wet_status: bool,
    pub apnea_count: Option<u32>,
    pub weight_value: Option<f32>,
    pub position: Option<Vec<i32>>,
}

/// 睡眠数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepData {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub deep_sleep_minutes: u32,
    pub light_sleep_minutes: u32,
    pub rem_sleep_minutes: u32,
    pub awake_minutes: u32,
    pub score: Option<u8>,
}

/// 运动数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityData {
    pub steps: u32,
    pub distance_meters: Option<f32>,
    pub calories: Option<u32>,
    pub active_minutes: Option<u32>,
}

/// 数据质量
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataQuality {
    Good,
    Fair,
    Poor,
    Invalid,
}

impl Default for DataQuality {
    fn default() -> Self {
        Self::Good
    }
}

/// 数据包（用于数据上报）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDataPacket {
    pub device_id: String,
    pub timestamp: DateTime<Utc>,
    pub data_type: String,
    pub payload: serde_json::Value,
    pub quality: DataQuality,
    pub metadata: Option<serde_json::Value>,
}

impl HealthDataPacket {
    /// 创建心率数据包
    pub fn heart_rate(device_id: &str, bpm: u32) -> Self {
        let data = HeartRateData { bpm, confidence: None };
        Self {
            device_id: device_id.to_string(),
            timestamp: Utc::now(),
            data_type: "heart_rate".to_string(),
            payload: serde_json::to_value(data).unwrap_or_default(),
            quality: DataQuality::Good,
            metadata: None,
        }
    }

    /// 创建床垫传感器数据包
    pub fn mattress_sensor(device_id: &str, data: MattressSensorData) -> Self {
        Self {
            device_id: device_id.to_string(),
            timestamp: Utc::now(),
            data_type: "mattress_sensor".to_string(),
            payload: serde_json::to_value(data).unwrap_or_default(),
            quality: DataQuality::Good,
            metadata: None,
        }
    }

    /// 验证数据完整性
    pub fn validate(&self) -> Result<(), String> {
        if self.device_id.is_empty() {
            return Err("设备ID不能为空".to_string());
        }
        if self.payload.is_null() {
            return Err("数据内容不能为空".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heart_rate_packet() {
        let packet = HealthDataPacket::heart_rate("device001", 72);
        assert_eq!(packet.data_type, "heart_rate");
        assert!(packet.validate().is_ok());
    }

    #[test]
    fn test_mattress_sensor_packet() {
        let data = MattressSensorData {
            heart_rate: Some(72),
            breath_rate: Some(18),
            is_on_bed: true,
            wet_status: false,
            apnea_count: Some(0),
            weight_value: Some(70.5),
            position: Some(vec![1, 2, 3]),
        };
        let packet = HealthDataPacket::mattress_sensor("device002", data);
        assert_eq!(packet.data_type, "mattress_sensor");
        assert!(packet.validate().is_ok());
    }

    #[test]
    fn test_invalid_packet() {
        let packet = HealthDataPacket {
            device_id: "".to_string(),
            timestamp: Utc::now(),
            data_type: "test".to_string(),
            payload: serde_json::json!({}),
            quality: DataQuality::Good,
            metadata: None,
        };
        assert!(packet.validate().is_err());
    }
}
