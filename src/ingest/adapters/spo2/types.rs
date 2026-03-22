//! 血氧传感器类型定义

use serde::{Deserialize, Serialize};

/// 血氧数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpO2Data {
    pub spo2: u32,
    pub pulse_rate: u32,
    pub unit: String,
}
