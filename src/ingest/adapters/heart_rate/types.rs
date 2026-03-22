//! 心率监测器类型定义

use serde::{Deserialize, Serialize};

/// 心率数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateData {
    pub heart_rate: u32,
    pub unit: String,
}

/// 心率事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeartRateEvent {
    Normal {
        heart_rate: u32,
        timestamp: String,
    },
    Abnormal {
        heart_rate: u32,
        timestamp: String,
        reason: String,
    },
}
