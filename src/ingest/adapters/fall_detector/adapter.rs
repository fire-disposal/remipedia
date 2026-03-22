//! 跌倒检测器适配器

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::DeviceAdapter;
use chrono::Utc;
use serde_json::json;

use super::types::{FallDetectorData, FallDetectorEvent, FallEventType};

/// 跌倒检测器适配器
pub struct FallDetectorAdapter;

impl FallDetectorAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 解析跌倒检测数据
    pub fn parse_fall_data(&self, raw: &[u8]) -> AppResult<FallDetectorData> {
        if raw.is_empty() {
            return Err(AppError::ValidationError("数据为空".into()));
        }

        let event_type = match raw[0] {
            0 => FallEventType::Normal,
            1 => FallEventType::FallDetected,
            2 => FallEventType::ImpactDetected,
            _ => FallEventType::Unknown,
        };

        let confidence = if raw.len() > 1 {
            raw[1] as f32 / 100.0
        } else {
            0.0
        };

        Ok(FallDetectorData {
            event_type,
            confidence,
            raw_data: raw.to_vec(),
        })
    }

    /// 检测跌倒事件
    pub fn detect_fall_events(&self, data: &FallDetectorData) -> Vec<FallDetectorEvent> {
        let mut events = Vec::new();
        let timestamp = Utc::now().to_rfc3339();

        match &data.event_type {
            FallEventType::Normal => {
                events.push(FallDetectorEvent::Normal {
                    confidence: data.confidence,
                    timestamp: timestamp.clone(),
                });
            }
            FallEventType::FallDetected => {
                events.push(FallDetectorEvent::FallDetected {
                    confidence: data.confidence,
                    timestamp: timestamp.clone(),
                    severity: "high".to_string(),
                });
            }
            FallEventType::ImpactDetected => {
                events.push(FallDetectorEvent::ImpactDetected {
                    confidence: data.confidence,
                    timestamp: timestamp.clone(),
                    intensity: "medium".to_string(),
                });
            }
            FallEventType::Unknown => {
                // 不生成事件，记录日志即可
            }
        }

        events
    }
}

impl DeviceAdapter for FallDetectorAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        let fall_data = self.parse_fall_data(raw)?;
        let events = self.detect_fall_events(&fall_data);

        Ok(json!({
            "event_type": match fall_data.event_type {
                FallEventType::Normal => "normal",
                FallEventType::FallDetected => "fall_detected",
                FallEventType::ImpactDetected => "impact_detected",
                FallEventType::Unknown => "unknown",
            },
            "confidence": fall_data.confidence,
            "events": events,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let event_type = payload["event_type"].as_str().unwrap_or("unknown");
        let confidence = payload["confidence"].as_f64().unwrap_or(0.0);

        if confidence < 0.0 || confidence > 1.0 {
            return Err(AppError::ValidationError("置信度超出范围".into()));
        }

        match event_type {
            "fall_detected" => {
                // 跌倒事件需要高置信度
                if confidence < 0.7 {
                    return Err(AppError::ValidationError("跌倒事件置信度不足".into()));
                }
            }
            "impact_detected" => {
                // 撞击事件需要中等置信度
                if confidence < 0.5 {
                    return Err(AppError::ValidationError("撞击事件置信度不足".into()));
                }
            }
            "normal" => {
                // 正常事件可以接受低置信度
            }
            _ => {
                return Err(AppError::ValidationError("未知事件类型".into()));
            }
        }

        Ok(())
    }

    fn device_type(&self) -> &'static str {
        "fall_detector"
    }

    fn data_type(&self) -> &'static str {
        "fall_detector"
    }
}
