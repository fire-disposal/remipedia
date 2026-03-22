//! 跌倒检测器类型定义

use serde::{Deserialize, Serialize};

/// 跌倒事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallEventType {
    Normal,
    FallDetected,
    ImpactDetected,
    Unknown,
}

/// 跌倒检测数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallDetectorData {
    pub event_type: FallEventType,
    pub confidence: f32,
    pub raw_data: Vec<u8>,
}

/// 跌倒事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallDetectorEvent {
    Normal {
        confidence: f32,
        timestamp: String,
    },
    FallDetected {
        confidence: f32,
        timestamp: String,
        severity: String,
    },
    ImpactDetected {
        confidence: f32,
        timestamp: String,
        intensity: String,
    },
}