//! 跌倒检测器适配器

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::DeviceAdapter;
use chrono::Utc;
use serde_json::json;

use super::types::{FallAlertEvent, FallDetectorData, FallDetectorEventType, FallDetectorMessage};

/// 跌倒检测器适配器
pub struct FallDetectorAdapter;

impl FallDetectorAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 解析MQTT消息JSON
    pub fn parse_message(&self, payload: &[u8]) -> AppResult<FallDetectorMessage> {
        let msg: FallDetectorMessage = serde_json::from_slice(payload).map_err(|e| {
            AppError::ValidationError(format!("跌倒检测消息解析失败: {}", e))
        })?;
        Ok(msg)
    }

    /// 将消息转换为内部数据结构
    pub fn to_data(&self, msg: FallDetectorMessage) -> AppResult<FallDetectorData> {
        // 验证置信度范围
        if !(0.0..=1.0).contains(&msg.confidence) {
            return Err(AppError::ValidationError("置信度必须在0.0-1.0之间".into()));
        }

        // 解析时间戳，未提供则使用当前时间
        let timestamp = if let Some(ts) = &msg.timestamp {
            chrono::DateTime::parse_from_rfc3339(ts)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now())
        } else {
            Utc::now()
        };

        Ok(FallDetectorData {
            event_type: msg.event_type,
            confidence: msg.confidence,
            timestamp,
        })
    }

    /// 生成告警事件
    pub fn create_alert_event(&self, data: &FallDetectorData) -> Option<FallAlertEvent> {
        if !data.event_type.is_alert() {
            return None;
        }

        let severity = match data.event_type {
            FallDetectorEventType::PersonFall => {
                if data.confidence >= 0.8 {
                    "high"
                } else if data.confidence >= 0.6 {
                    "medium"
                } else {
                    "low"
                }
            }
            _ => "low",
        };

        Some(FallAlertEvent {
            event_type: data.event_type,
            confidence: data.confidence,
            timestamp: data.timestamp,
            severity: severity.to_string(),
        })
    }
}

impl DeviceAdapter for FallDetectorAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        let msg = self.parse_message(raw)?;
        let data = self.to_data(msg)?;

        Ok(json!({
            "event_type": data.event_type.as_str(),
            "confidence": data.confidence,
            "timestamp": data.timestamp.to_rfc3339(),
        }))
    }

    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let event_type_str = payload["event_type"]
            .as_str()
            .ok_or_else(|| AppError::ValidationError("缺少event_type字段".into()))?;

        let event_type = FallDetectorEventType::from_str(event_type_str)
            .ok_or_else(|| AppError::ValidationError(format!("未知事件类型: {}", event_type_str)))?;

        let confidence = payload["confidence"]
            .as_f64()
            .ok_or_else(|| AppError::ValidationError("缺少confidence字段".into()))?;

        if !(0.0..=1.0).contains(&confidence) {
            return Err(AppError::ValidationError("置信度必须在0.0-1.0之间".into()));
        }

        // 跌倒事件需要较高置信度
        if event_type == FallDetectorEventType::PersonFall && confidence < 0.5 {
            return Err(AppError::ValidationError("跌倒事件置信度不足(需>=0.5)".into()));
        }

        Ok(())
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
    fn test_parse_fall_message() {
        let adapter = FallDetectorAdapter::new();
        let payload = br#"{"event_type":"person_fall","confidence":0.85}"#;

        let msg = adapter.parse_message(payload).unwrap();
        assert_eq!(msg.event_type, FallDetectorEventType::PersonFall);
        assert_eq!(msg.confidence, 0.85);
    }

    #[test]
    fn test_parse_enter_message() {
        let adapter = FallDetectorAdapter::new();
        let payload = br#"{"event_type":"person_enter","confidence":0.9,"timestamp":"2024-01-15T10:30:00Z"}"#;

        let msg = adapter.parse_message(payload).unwrap();
        assert_eq!(msg.event_type, FallDetectorEventType::PersonEnter);
        assert_eq!(msg.confidence, 0.9);
        assert!(msg.timestamp.is_some());
    }

    #[test]
    fn test_validate_fall_event() {
        let adapter = FallDetectorAdapter::new();
        let payload = json!({
            "event_type": "person_fall",
            "confidence": 0.8
        });

        assert!(adapter.validate(&payload).is_ok());
    }

    #[test]
    fn test_validate_low_confidence_fall() {
        let adapter = FallDetectorAdapter::new();
        let payload = json!({
            "event_type": "person_fall",
            "confidence": 0.3
        });

        assert!(adapter.validate(&payload).is_err());
    }

    #[test]
    fn test_create_alert_event() {
        let adapter = FallDetectorAdapter::new();
        let data = FallDetectorData {
            event_type: FallDetectorEventType::PersonFall,
            confidence: 0.9,
            timestamp: Utc::now(),
        };

        let alert = adapter.create_alert_event(&data).unwrap();
        assert_eq!(alert.severity, "high");
    }
}