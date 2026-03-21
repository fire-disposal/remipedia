use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::DeviceAdapter;
use serde_json::json;

/// 心率监测器适配器
pub struct HeartRateAdapter;

impl DeviceAdapter for HeartRateAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 假设数据格式：[心率高字节, 心率低字节]
        if raw.len() < 2 {
            return Err(AppError::ValidationError("数据长度不足".into()));
        }

        let heart_rate = u16::from_be_bytes([raw[0], raw[1]]) as u32;

        Ok(json!({
            "heart_rate": heart_rate,
            "unit": "bpm"
        }))
    }

    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let hr = payload["heart_rate"].as_u64().unwrap_or(0);

        if hr < 30 {
            return Err(AppError::ValidationError("心率过低".into()));
        }
        if hr > 220 {
            return Err(AppError::ValidationError("心率过高".into()));
        }

        Ok(())
    }

    fn data_type(&self) -> &'static str {
        "heart_rate"
    }

    fn device_type(&self) -> &'static str {
        "heart_rate_monitor"
    }
}