//! 跌倒检测器适配器

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::adapter_trait::{AdapterOutput, DeviceAdapter, MessagePayload};
use chrono::Utc;

use super::types::{FallDetectorData, FallDetectorMessage};

/// 跌倒检测器适配器（示例型 MQTT 输入）
///
/// 约定：
/// - 不做置信度计算或阈值推断；
/// - 仅做格式校验、时间解析、字段透传。
pub struct FallDetectorAdapter;

impl FallDetectorAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_message(&self, payload: &[u8]) -> AppResult<FallDetectorMessage> {
        serde_json::from_slice(payload)
            .map_err(|e| AppError::ValidationError(format!("跌倒检测消息解析失败: {}", e)))
    }

    pub fn to_data(&self, msg: FallDetectorMessage) -> FallDetectorData {
        let timestamp = msg
            .timestamp
            .as_ref()
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        FallDetectorData {
            event_type: msg.event_type,
            timestamp,
            details: msg.details,
        }
    }
}

impl DeviceAdapter for FallDetectorAdapter {
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        let msg = self.parse_message(raw)?;
        let data = self.to_data(msg);

        let payload = serde_json::json!({
            "event_type": data.event_type.as_str(),
            "details": data.details,
        });

        Ok(AdapterOutput::Messages(vec![MessagePayload {
            time: data.timestamp,
            data_type: self.data_type().to_string(),
            message_type: Some(data.event_type.as_str().to_string()),
            severity: None,
            payload,
        }]))
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

    fn device_type(&self) -> &'static str {
        "fall_detector"
    }

    fn data_type(&self) -> &'static str {
        "fall_event"
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
    fn test_parse_message_without_confidence() {
        let adapter = FallDetectorAdapter::new();
        let payload = br#"{"event_type":"person_fall"}"#;

        let msg = adapter.parse_message(payload).unwrap();
        assert!(msg.timestamp.is_none());
        assert!(msg.details.is_none());
    }

    #[test]
    fn test_parse_with_details() {
        let adapter = FallDetectorAdapter::new();
        let payload = br#"{"event_type":"person_enter","details":{"zone":"door"}}"#;

        let output = adapter.parse(payload).unwrap();
        assert!(adapter.validate(&output).is_ok());
    }
}
