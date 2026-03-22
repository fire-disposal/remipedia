//! 血氧传感器适配器

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::DeviceAdapter;
use chrono::Utc;
use serde_json::json;

use super::types::SpO2Data;

/// 血氧传感器适配器
pub struct SpO2Adapter;

impl SpO2Adapter {
    pub fn new() -> Self {
        Self
    }

    /// 解析血氧数据
    pub fn parse_spo2_data(&self, raw: &[u8]) -> AppResult<SpO2Data> {
        // 假设数据格式：[血氧饱和度, 脉率]
        if raw.len() < 2 {
            return Err(AppError::ValidationError("数据长度不足".into()));
        }

        let spo2 = raw[0] as u32;
        let pulse_rate = raw[1] as u32;

        Ok(SpO2Data {
            spo2,
            pulse_rate,
            unit: "%".to_string(),
        })
    }
}

impl DeviceAdapter for SpO2Adapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        let spo2_data = self.parse_spo2_data(raw)?;

        Ok(json!({
            "spo2": spo2_data.spo2,
            "pulse_rate": spo2_data.pulse_rate,
            "unit": spo2_data.unit,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let spo2 = payload["spo2"].as_u64().unwrap_or(0);

        if spo2 < 70 {
            return Err(AppError::ValidationError("血氧饱和度过低".into()));
        }
        if spo2 > 100 {
            return Err(AppError::ValidationError("血氧饱和度超出正常范围".into()));
        }

        Ok(())
    }

    fn device_type(&self) -> &'static str {
        "spo2"
    }

    fn data_type(&self) -> &'static str {
        "spo2"
    }
}
