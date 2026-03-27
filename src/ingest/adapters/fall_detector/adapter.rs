//! 跌倒检测器适配器
//!
//! 无状态设计：仅负责解析和验证，状态由 DeviceManager 管理

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::adapter_trait::{
    AdapterOutput, DeviceAdapter, DeviceMetadata, MessagePayload,
};
use chrono::Utc;

pub struct FallDetectorAdapter;

impl FallDetectorAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceAdapter for FallDetectorAdapter {
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: "fall_detector",
            display_name: "跌倒检测器",
            supported_data_types: &["fall_detector"],
            protocol_version: "1.0",
        }
    }

    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        if raw.is_empty() {
            return Err(AppError::ValidationError("数据为空".into()));
        }

        let data: serde_json::Value = serde_json::from_slice(raw)
            .map_err(|e| AppError::ValidationError(format!("JSON 解析失败: {}", e)))?;

        let event_type = data
            .get("event_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ValidationError("缺少 event_type 字段".into()))?
            .to_string();

        let details = data.get("details").cloned();

        // 解析时间戳
        let timestamp = data
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        // 根据事件类型设置严重程度
        let severity = match event_type.as_str() {
            "person_fall" => Some("critical".to_string()),
            "person_still" => Some("warning".to_string()),
            _ => Some("info".to_string()),
        };

        let payload = serde_json::json!({
            "event_type": event_type,
            "details": details,
        });

        let msg = MessagePayload {
            time: timestamp,
            data_type: "fall_detector".to_string(),
            message_type: Some(event_type),
            severity,
            payload,
        };

        Ok(AdapterOutput::Messages(vec![msg]))
    }

    fn validate(&self, output: &AdapterOutput) -> AppResult<()> {
        match output {
            AdapterOutput::Messages(msgs) if msgs.is_empty() => {
                Err(AppError::ValidationError("空消息输出".into()))
            }
            AdapterOutput::Messages(msgs) => {
                for msg in msgs {
                    if msg.message_type.is_none() {
                        return Err(AppError::ValidationError("缺少 event_type".into()));
                    }
                }
                Ok(())
            }
        }
    }

    fn clone_box(&self) -> Box<dyn DeviceAdapter> {
        Box::new(Self::new())
    }

    fn device_type(&self) -> &'static str {
        "fall_detector"
    }

    fn data_type(&self) -> &'static str {
        "fall_detector"
    }
}

impl Default for FallDetectorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fall_event() {
        let adapter = FallDetectorAdapter::new();
        let payload = br#"{"event_type":"person_fall"}"#;

        let output = adapter.parse(payload).unwrap();
        assert!(adapter.validate(&output).is_ok());
    }

    #[test]
    fn test_parse_with_details() {
        let adapter = FallDetectorAdapter::new();
        let payload = br#"{"event_type":"person_enter","details":{"zone":"door"}}"#;

        let output = adapter.parse(payload).unwrap();
        assert!(adapter.validate(&output).is_ok());
    }
}
