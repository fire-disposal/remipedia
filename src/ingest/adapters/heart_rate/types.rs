//! 心率监测器数据类型

use serde::{Deserialize, Serialize};

/// 心率数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateData {
    /// 心率值 (bpm)
    pub heart_rate: i32,
    /// 设备状态
    pub status: String,
    /// 电池电量 (可选)
    pub battery: Option<i32>,
}

/// 心率事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeartRateEvent {
    /// 心率异常 - 过高
    HighHeartRate {
        heart_rate: i32,
        threshold: i32,
        duration_seconds: i32,
    },
    /// 心率异常 - 过低
    LowHeartRate {
        heart_rate: i32,
        threshold: i32,
        duration_seconds: i32,
    },
    /// 心率正常恢复
    HeartRateNormal { heart_rate: i32 },
    /// 设备断开
    DeviceDisconnected,
    /// 设备低电量
    LowBattery { battery_level: i32 },
}
