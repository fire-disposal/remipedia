//! 智能床垫类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 床垫数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattressData {
    pub manufacturer: String,
    pub model: String,
    pub version: i32,
    pub serial_number: String,
    pub firmware_version: i32,
    pub status: String,
    pub heart_rate: i32,
    pub breath_rate: i32,
    pub wet_status: bool,
    pub apnea_count: i32,
    pub weight_value: i32,
    pub position: [i32; 2],
    pub timestamp: DateTime<Utc>,
}

impl MattressData {
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            manufacturer: value.get("manufacturer")?.as_str()?.to_string(),
            model: value.get("model")?.as_str()?.to_string(),
            version: value.get("version")?.as_i64()? as i32,
            serial_number: value.get("serial_number")?.as_str()?.to_string(),
            firmware_version: value.get("firmware_version")?.as_i64()? as i32,
            status: value.get("status")?.as_str()?.to_string(),
            heart_rate: value.get("heart_rate")?.as_i64()? as i32,
            breath_rate: value.get("breath_rate")?.as_i64()? as i32,
            wet_status: value.get("wet_status")?.as_bool()?,
            apnea_count: value.get("apnea_count")?.as_i64()? as i32,
            weight_value: value.get("weight_value")?.as_i64()? as i32,
            position: [
                value.get("position")?.get(0)?.as_i64()? as i32,
                value.get("position")?.get(1)?.as_i64()? as i32,
            ],
            timestamp: Utc::now(),
        })
    }
}

/// 床垫状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MattressState {
    OffBed,
    OnBed,
    Moving,
    Calling,
}

/// 警报级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AlertLevel {
    Normal,
    Warning,
    Critical,
}

impl AlertLevel {
    pub fn from_vital_signs(heart_rate: i32, breath_rate: i32) -> (AlertLevel, AlertLevel) {
        let hr_level = if heart_rate < 50 || heart_rate > 120 {
            AlertLevel::Critical
        } else if heart_rate < 60 || heart_rate > 100 {
            AlertLevel::Warning
        } else {
            AlertLevel::Normal
        };

        let br_level = if breath_rate < 8 || breath_rate > 25 {
            AlertLevel::Critical
        } else if breath_rate < 12 || breath_rate > 20 {
            AlertLevel::Warning
        } else {
            AlertLevel::Normal
        };

        (hr_level, br_level)
    }
}

/// 床垫事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum MattressEvent {
    BedEntry {
        timestamp: DateTime<Utc>,
        confidence: f32,
        weight_value: i32,
    },
    BedExit {
        timestamp: DateTime<Utc>,
        confidence: f32,
        duration_minutes: f32,
    },
    SignificantMovement {
        timestamp: DateTime<Utc>,
        intensity: f32,
        position_change: f32,
        score: i32,
    },
    VitalSignsAnomaly {
        timestamp: DateTime<Utc>,
        heart_rate: i32,
        heart_rate_level: AlertLevel,
        breath_rate: i32,
        breath_rate_level: AlertLevel,
        anomaly_type: String,
    },
    ApneaEvent {
        timestamp: DateTime<Utc>,
        duration_seconds: i32,
        severity: AlertLevel,
        apnea_count: i32,
    },
    MoistureAlert {
        timestamp: DateTime<Utc>,
        wet_status: bool,
        duration_minutes: i32,
        severity: AlertLevel,
    },
    ScheduledMeasurement {
        timestamp: DateTime<Utc>,
        heart_rate: i32,
        breath_rate: i32,
        apnea_count: i32,
        wet_status: bool,
        weight_value: i32,
        measurement_reason: String,
    },
}

impl MattressEvent {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            MattressEvent::BedEntry { timestamp, .. } => *timestamp,
            MattressEvent::BedExit { timestamp, .. } => *timestamp,
            MattressEvent::SignificantMovement { timestamp, .. } => *timestamp,
            MattressEvent::VitalSignsAnomaly { timestamp, .. } => *timestamp,
            MattressEvent::ApneaEvent { timestamp, .. } => *timestamp,
            MattressEvent::MoistureAlert { timestamp, .. } => *timestamp,
            MattressEvent::ScheduledMeasurement { timestamp, .. } => *timestamp,
        }
    }

    pub fn event_type(&self) -> String {
        match self {
            MattressEvent::BedEntry { .. } => "bed_entry".to_string(),
            MattressEvent::BedExit { .. } => "bed_exit".to_string(),
            MattressEvent::SignificantMovement { .. } => "significant_movement".to_string(),
            MattressEvent::VitalSignsAnomaly { .. } => "vital_signs_anomaly".to_string(),
            MattressEvent::ApneaEvent { .. } => "apnea_event".to_string(),
            MattressEvent::MoistureAlert { .. } => "moisture_alert".to_string(),
            MattressEvent::ScheduledMeasurement { .. } => "scheduled_measurement".to_string(),
        }
    }

    pub fn severity(&self) -> Option<AlertLevel> {
        match self {
            MattressEvent::BedEntry { .. } => None,
            MattressEvent::BedExit { .. } => None,
            MattressEvent::SignificantMovement { .. } => None,
            MattressEvent::VitalSignsAnomaly {
                heart_rate_level,
                breath_rate_level,
                ..
            } => {
                if *heart_rate_level == AlertLevel::Critical
                    || *breath_rate_level == AlertLevel::Critical
                {
                    Some(AlertLevel::Critical)
                } else if *heart_rate_level == AlertLevel::Warning
                    || *breath_rate_level == AlertLevel::Warning
                {
                    Some(AlertLevel::Warning)
                } else {
                    Some(AlertLevel::Normal)
                }
            }
            MattressEvent::ApneaEvent { severity, .. } => Some(*severity),
            MattressEvent::MoistureAlert { severity, .. } => Some(*severity),
            MattressEvent::ScheduledMeasurement { .. } => None,
        }
    }
}

/// 智能采样配置
#[derive(Debug, Clone)]
pub struct SmartSamplingConfig {
    pub normal_interval_minutes: i32,
    pub warning_interval_minutes: i32,
    pub critical_interval_seconds: i32,
}

impl Default for SmartSamplingConfig {
    fn default() -> Self {
        Self {
            normal_interval_minutes: 5,
            warning_interval_minutes: 1,
            critical_interval_seconds: 10,
        }
    }
}

/// 生命体征配置
#[derive(Debug, Clone)]
pub struct VitalSignsConfig {
    pub heart_rate_normal_range: (i32, i32),
    pub breath_rate_normal_range: (i32, i32),
    pub apnea_critical_threshold: i32,
    pub moisture_alert_threshold_minutes: i32,
}

impl Default for VitalSignsConfig {
    fn default() -> Self {
        Self {
            heart_rate_normal_range: (60, 100),
            breath_rate_normal_range: (12, 20),
            apnea_critical_threshold: 10,
            moisture_alert_threshold_minutes: 30,
        }
    }
}
