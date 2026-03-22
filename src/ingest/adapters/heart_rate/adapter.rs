//! 心率监测器适配器

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::DeviceAdapter;
use chrono::Utc;
use serde_json::json;

use super::types::{HeartRateData, HeartRateEvent};

/// 心率监测器适配器
pub struct HeartRateAdapter;

impl HeartRateAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 解析心率数据
    pub fn parse_heart_rate_data(&self, raw: &[u8]) -> AppResult<HeartRateData> {
        // 假设数据格式：[心率高字节, 心率低字节]
        if raw.len() < 2 {
            return Err(AppError::ValidationError("数据长度不足".into()));
        }

        let heart_rate = u16::from_be_bytes([raw[0], raw[1]]) as u32;

        Ok(HeartRateData {
            heart_rate,
            unit: "bpm".to_string(),
        })
    }

    /// 检测心率事件
    pub fn detect_heart_rate_events(&self, data: &HeartRateData) -> Vec<HeartRateEvent> {
        let mut events = Vec::new();
        let timestamp = Utc::now().to_rfc3339();

        // 基础心率事件检测
        if data.heart_rate < 30 {
            events.push(HeartRateEvent::Abnormal {
                heart_rate: data.heart_rate,
                timestamp: timestamp.clone(),
                reason: "心率过低".to_string(),
            });
        } else if data.heart_rate > 220 {
            events.push(HeartRateEvent::Abnormal {
                heart_rate: data.heart_rate,
                timestamp: timestamp.clone(),
                reason: "心率过高".to_string(),
            });
        } else {
            events.push(HeartRateEvent::Normal {
                heart_rate: data.heart_rate,
                timestamp,
            });
        }

        events
    }
}

impl DeviceAdapter for HeartRateAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        let heart_rate_data = self.parse_heart_rate_data(raw)?;
        let events = self.detect_heart_rate_events(&heart_rate_data);

        Ok(json!({
            "heart_rate": heart_rate_data.heart_rate,
            "unit": heart_rate_data.unit,
            "events": events,
            "timestamp": Utc::now().to_rfc3339()
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

    fn device_type(&self) -> &'static str {
        "heart_rate"
    }

    fn data_type(&self) -> &'static str {
        "heart_rate"
    }
}
