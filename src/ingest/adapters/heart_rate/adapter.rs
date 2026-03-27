//! 心率监测器适配器

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::adapter_trait::{
    AdapterOutput, DeviceAdapter, DeviceMetadata, MessagePayload,
};
use chrono::Utc;

use super::types::HeartRateData;

pub struct HeartRateAdapter;

impl HeartRateAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 解析心率数据
    pub fn parse_heart_rate_data(&self, raw: &[u8]) -> AppResult<HeartRateData> {
        // 假设格式: [心率值, 状态码, 电池电量?]
        // 具体格式根据设备协议定义
        if raw.is_empty() {
            return Err(AppError::ValidationError("数据为空".into()));
        }

        // 简化实现：假设是 JSON
        let data: HeartRateData = serde_json::from_slice(raw)
            .map_err(|e| AppError::ValidationError(format!("JSON 解析失败: {}", e)))?;

        // 验证心率范围
        if data.heart_rate > 0 && (data.heart_rate < 30 || data.heart_rate > 250) {
            return Err(AppError::ValidationError("心率值超出合理范围".into()));
        }

        Ok(data)
    }
}

impl DeviceAdapter for HeartRateAdapter {
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: "heart_rate_monitor",
            display_name: "心率监测器",
            supported_data_types: &["heart_rate", "heart_rate_event"],
            protocol_version: "1.0",
        }
    }

    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        let data = self.parse_heart_rate_data(raw)?;

        // 构建主负载
        let payload = serde_json::json!({
            "heart_rate": data.heart_rate,
            "status": data.status,
            "battery": data.battery,
        });

        let msg = MessagePayload {
            time: Utc::now(),
            data_type: "heart_rate".to_string(),
            message_type: None,
            severity: None,
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
                            if hr > 0 && (hr < 30 || hr > 250) {
                                return Err(AppError::ValidationError("心率值异常".into()));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn device_type(&self) -> &'static str {
        "heart_rate_monitor"
    }

    fn data_type(&self) -> &'static str {
        "heart_rate"
    }

    fn clone_box(&self) -> Box<dyn crate::ingest::DeviceAdapter> {
        Box::new(Self::new())
    }
}
