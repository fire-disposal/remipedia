use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::DeviceAdapter;
use serde_json::json;

/// 跌倒检测器适配器
pub struct FallDetectorAdapter;

impl DeviceAdapter for FallDetectorAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 假设数据格式：[事件类型, 置信度]
        if raw.is_empty() {
            return Err(AppError::ValidationError("数据为空".into()));
        }

        let event_type = match raw[0] {
            0 => "normal",
            1 => "fall_detected",
            2 => "impact_detected",
            _ => "unknown",
        };

        let confidence = if raw.len() > 1 { raw[1] as f32 / 100.0 } else { 0.0 };

        Ok(json!({
            "event_type": event_type,
            "confidence": confidence,
            "raw_data": raw.to_vec()
        }))
    }

    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let event_type = payload["event_type"].as_str().unwrap_or("unknown");

        if event_type == "unknown" {
            return Err(AppError::ValidationError("未知事件类型".into()));
        }

        Ok(())
    }

    fn data_type(&self) -> &'static str {
        "fall_event"
    }

    fn device_type(&self) -> &'static str {
        "fall_detector"
    }
}