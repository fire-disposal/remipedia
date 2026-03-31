//! 简单转发适配器
//!
//! 适用于90%的简单设备：直接解析JSON并转发

use crate::core::entity::DataPoint;
use crate::errors::{AppError, AppResult};
use crate::ingest::adapter::{DeviceAdapter, DeviceMetadata, DeviceType};
use crate::ingest::ParsedData;
use async_trait::async_trait;
use serde_json::Value;

/// 简单转发适配器
pub struct ForwardAdapter {
    metadata: DeviceMetadata,
}

impl ForwardAdapter {
    /// 创建新的转发适配器
    pub fn new(device_type: DeviceType, display_name: impl Into<String>) -> Self {
        Self {
            metadata: DeviceMetadata {
                device_type,
                display_name: display_name.into(),
                description: "简单转发适配器".to_string(),
                protocol_version: "1.0".to_string(),
                supports_events: false,
            },
        }
    }

    /// 从JSON创建转发适配器
    pub fn from_json(device_type: DeviceType) -> Self {
        let display_name = format!("{:?}", device_type);
        Self::new(
            device_type,
            display_name,
        )
    }
}

#[async_trait]
impl DeviceAdapter for ForwardAdapter {
    fn metadata(&self) -> DeviceMetadata {
        self.metadata.clone()
    }

    fn parse(&self, raw: &[u8]) -> AppResult<ParsedData> {
        // 尝试JSON解析
        let value: Value = serde_json::from_slice(raw)
            .map_err(|e| AppError::ValidationError(format!("JSON解析失败: {}", e)))?;

        // 提取设备ID和类型（如果有）
        let device_id = value
            .get("device_id")
            .or_else(|| value.get("sn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let device_type = value
            .get("device_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(ParsedData::new(device_id, device_type, value))
    }

    async fn process(
        &self,
        data: ParsedData,
    ) -> AppResult<Vec<DataPoint>> {
        // 尝试提取数值
        let value_numeric = data
            .payload
            .get("value")
            .or_else(|| data.payload.get("data"))
            .and_then(|v| v.as_f64())
            .or_else(|| {
                data.payload
                    .get("value")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as f64)
            });

        let point = DataPoint {
            time: data.timestamp,
            device_id: None, // 将在pipeline中填充
            patient_id: None,
            data_type: data.device_type,
            data_category: crate::core::entity::DataCategory::Metric,
            value_numeric,
            value_text: None,
            severity: None,
            status: None,
            payload: data.payload,
            source: "ingest".to_string(),
        };

        Ok(vec![point])
    }
}

/// 预定义的简单适配器
pub mod presets {
    use super::*;

    /// 心率监测器适配器
    pub fn heart_rate_monitor() -> ForwardAdapter {
        ForwardAdapter::new(
            DeviceType::HeartRateMonitor,
            "心率监测器",
        )
    }

    /// 血压计适配器
    pub fn blood_pressure_monitor() -> ForwardAdapter {
        ForwardAdapter::new(
            DeviceType::BloodPressureMonitor,
            "血压计",
        )
    }

    /// 血糖仪适配器
    pub fn glucose_meter() -> ForwardAdapter {
        ForwardAdapter::new(
            DeviceType::GlucoseMeter,
            "血糖仪",
        )
    }

    /// 跌倒检测器适配器
    pub fn fall_detector() -> ForwardAdapter {
        let mut adapter = ForwardAdapter::new(
            DeviceType::FallDetector,
            "跌倒检测器",
        );
        adapter.metadata.supports_events = true;
        adapter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_adapter_parse() {
        let adapter = ForwardAdapter::from_json(DeviceType::HeartRateMonitor);
        let raw = br#"{"device_id": "test123", "value": 75, "device_type": "heart_rate_monitor"}"#;

        let parsed = adapter.parse(raw).unwrap();
        assert_eq!(parsed.device_id, "test123");
        assert_eq!(parsed.device_type, "heart_rate_monitor");
    }
}
