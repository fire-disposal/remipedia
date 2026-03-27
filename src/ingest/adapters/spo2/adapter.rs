//! 血氧传感器适配器

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::adapter_trait::{AdapterOutput, DeviceAdapter, MessagePayload};
use chrono::Utc;

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
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        let spo2_data = self.parse_spo2_data(raw)?;

        let payload = serde_json::json!({
            "spo2": spo2_data.spo2,
            "pulse_rate": spo2_data.pulse_rate,
            "unit": spo2_data.unit,
        });

        let msg = MessagePayload {
            time: Utc::now(),
            data_type: "spo2".to_string(),
            message_type: None,
            severity: None,
            payload,
        };

        Ok(AdapterOutput::Messages(vec![msg]))
    }

    fn validate(&self, output: &AdapterOutput) -> AppResult<()> {
        // 结构性校验：遍历消息查找 spo2 值并校验范围
        match output {
            AdapterOutput::Messages(msgs) => {
                for m in msgs {
                    if m.data_type == "spo2" {
                        let spo2 = m.payload.get("spo2").and_then(|v| v.as_u64()).unwrap_or(0);
                        if spo2 == 0 {
                            return Err(AppError::ValidationError("缺少 spo2 字段".into()));
                        }
                        if spo2 < 50 {
                            return Err(AppError::ValidationError("血氧饱和度过低".into()));
                        }
                        if spo2 > 100 {
                            return Err(AppError::ValidationError("血氧饱和度超出正常范围".into()));
                        }
                    }
                }
            }
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
