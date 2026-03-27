//! 心率监测器适配器
//!
//! 无状态设计：仅负责解析和验证，状态由 DeviceManager 管理

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::adapter_trait::{
    AdapterOutput, DeviceAdapter, DeviceMetadata, MessagePayload,
};
use chrono::Utc;

pub struct HeartRateAdapter;

impl HeartRateAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceAdapter for HeartRateAdapter {
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: "heart_rate_monitor",
            display_name: "心率监测器",
            supported_data_types: &["heart_rate"],
            protocol_version: "1.0",
        }
    }

    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        if raw.is_empty() {
            return Err(AppError::ValidationError("数据为空".into()));
        }

        // 支持 JSON 格式
        let data: serde_json::Value = serde_json::from_slice(raw)
            .map_err(|e| AppError::ValidationError(format!("JSON 解析失败: {}", e)))?;

        let heart_rate = data
            .get("heart_rate")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::ValidationError("缺少 heart_rate 字段".into()))?;

        let status = data
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("normal")
            .to_string();

        let battery = data.get("battery").and_then(|v| v.as_i64());

        // 检测异常
        let (message_type, severity) = if heart_rate > 0 && heart_rate < 40 {
            ("low_heart_rate".to_string(), Some("critical".to_string()))
        } else if heart_rate > 120 {
            ("high_heart_rate".to_string(), Some("warning".to_string()))
        } else {
            (String::new(), None)
        };

        let payload = serde_json::json!({
            "heart_rate": heart_rate,
            "status": status,
            "battery": battery,
        });

        let msg = MessagePayload {
            time: Utc::now(),
            data_type: "heart_rate".to_string(),
            message_type: if message_type.is_empty() {
                None
            } else {
                Some(message_type)
            },
            severity,
            payload,
        };

        Ok(AdapterOutput::Messages(vec![msg]))
    }

    fn validate(&self, output: &AdapterOutput) -> AppResult<()> {
        match output {
            AdapterOutput::Messages(msgs) => {
                for m in msgs {
                    if m.data_type == "heart_rate" {
                        if let Some(hr) = m.payload.get("heart_rate").and_then(|v| v.as_i64()) {
                            if hr == 0 || hr > 250 {
                                return Err(AppError::ValidationError("心率值异常".into()));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn DeviceAdapter> {
        Box::new(Self::new())
    }

    fn device_type(&self) -> &'static str {
        "heart_rate_monitor"
    }

    fn data_type(&self) -> &'static str {
        "heart_rate"
    }
}

impl Default for HeartRateAdapter {
    fn default() -> Self {
        Self::new()
    }
}
