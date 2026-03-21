use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::DeviceAdapter;
use serde_json::json;

/// 血氧仪适配器
pub struct SpO2Adapter;

impl DeviceAdapter for SpO2Adapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 假设数据格式：[血氧饱和度, 脉率]
        if raw.len() < 2 {
            return Err(AppError::ValidationError("数据长度不足".into()));
        }

        let spo2 = raw[0] as u32;
        let pulse_rate = raw[1] as u32;

        Ok(json!({
            "spo2": spo2,
            "pulse_rate": pulse_rate,
            "unit": "%"
        }))
    }

    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let spo2 = payload["spo2"].as_u64().unwrap_or(0);

        if spo2 < 70 {
            return Err(AppError::ValidationError("血氧饱和度过低".into()));
        }
        if spo2 > 100 {
            return Err(AppError::ValidationError("血氧饱和度无效".into()));
        }

        let pulse_rate = payload["pulse_rate"].as_u64().unwrap_or(0);
        if pulse_rate < 30 || pulse_rate > 220 {
            return Err(AppError::ValidationError("脉率异常".into()));
        }

        Ok(())
    }

    fn data_type(&self) -> &'static str {
        "spo2"
    }

    fn device_type(&self) -> &'static str {
        "spo2_sensor"
    }
}