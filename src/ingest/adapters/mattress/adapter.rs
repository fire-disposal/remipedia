//! 床垫适配器V2实现

use crate::core::entity::DataPoint;
use crate::errors::{AppError, AppResult};
use crate::ingest::adapter::{DeviceAdapter, DeviceMetadata, DeviceType};
use crate::ingest::adapters::mattress::{
    decoder::MattressProtocolDecoder, state::MattressStateV2, types::MattressData,
};
use crate::ingest::protocol::ProtocolDecoder;
use crate::ingest::state::DeviceState;
use crate::ingest::ParsedData;
use async_trait::async_trait;
use chrono::Utc;

/// 床垫适配器V2
pub struct MattressAdapterV2 {
    metadata: DeviceMetadata,
    decoder: MattressProtocolDecoder,
}

impl MattressAdapterV2 {
    pub fn new() -> Self {
        Self {
            metadata: DeviceMetadata {
                device_type: DeviceType::SmartMattress,
                display_name: "智能床垫".to_string(),
                description: "智能床垫设备适配器（支持完整事件检测）".to_string(),
                protocol_version: "2.0".to_string(),
                supports_events: true,
            },
            decoder: MattressProtocolDecoder::new(),
        }
    }

    /// 从解码后的数据中提取床垫数据
    fn extract_mattress_data(&self, decoded: &[u8]) -> AppResult<MattressData> {
        if decoded.len() < 5 {
            return Err(AppError::ValidationError("数据包太短".into()));
        }

        let data = &decoded[4..];
        let value: serde_json::Value = rmp_serde::from_slice(data)
            .map_err(|e| AppError::ValidationError(format!("MessagePack解析失败: {}", e)))?;

        // 验证必要字段
        let manufacturer = value
            .get("ma")
            .or_else(|| value.get("manufacturer"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ValidationError("缺少manufacturer".into()))?;

        if manufacturer != "HT" {
            return Err(AppError::ValidationError(format!(
                "不支持的制造商: {}",
                manufacturer
            )));
        }

        MattressData::from_json(&value)
            .ok_or_else(|| AppError::ValidationError("无法解析床垫数据".into()))
    }
}

impl Default for MattressAdapterV2 {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DeviceAdapter for MattressAdapterV2 {
    fn metadata(&self) -> DeviceMetadata {
        self.metadata.clone()
    }

    fn is_stateful(&self) -> bool {
        true
    }

    fn protocol_decoder(&self) -> Option<&dyn ProtocolDecoder> {
        Some(&self.decoder)
    }

    fn create_state(&self) -> Option<Box<dyn DeviceState>> {
        Some(Box::new(MattressStateV2::new()))
    }

    fn parse(&self,
        raw: &[u8]) -> AppResult<ParsedData> {
        // 这里假设已经被protocol_decoder解码过
        // 实际数据格式: [4字节头][MessagePack数据]
        let mattress_data = self.extract_mattress_data(raw)?;

        let payload = serde_json::json!({
            "manufacturer": mattress_data.manufacturer,
            "model": mattress_data.model,
            "version": mattress_data.version,
            "serial_number": mattress_data.serial_number,
            "firmware_version": mattress_data.firmware_version,
            "status": mattress_data.status,
            "heart_rate": mattress_data.heart_rate,
            "breath_rate": mattress_data.breath_rate,
            "wet_status": mattress_data.wet_status,
            "apnea_count": mattress_data.apnea_count,
            "weight_value": mattress_data.weight_value,
            "position": mattress_data.position,
        });

        Ok(ParsedData::new(
            mattress_data.serial_number.clone(),
            "smart_mattress".to_string(),
            payload,
        ))
    }

    async fn process_with_state(
        &self,
        data: ParsedData,
        state: &mut dyn DeviceState,
    ) -> AppResult<Vec<DataPoint>> {
        let mattress_state = state
            .as_any_mut()
            .downcast_mut::<MattressStateV2>()
            .ok_or_else(|| AppError::InternalError)?;

        // 使用状态更新并生成事件
        let device_events = mattress_state.update(&data.payload)?;

        // 构造指标数据点
        let metric_point = DataPoint::metric(
            None, // device_id将在pipeline中填充
            None,
            "smart_mattress",
            data.payload.get("heart_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            data.payload.clone(),
        );

        let mut points = vec![metric_point];

        // 构造事件数据点
        for event in device_events {
            let severity = match event.severity {
                crate::ingest::state::EventSeverity::Info => {
                    crate::core::entity::Severity::Info
                }
                crate::ingest::state::EventSeverity::Warning => {
                    crate::core::entity::Severity::Warning
                }
                crate::ingest::state::EventSeverity::Critical => {
                    crate::core::entity::Severity::Alert
                }
            };

            let event_point = DataPoint::event(
                None,
                None,
                &event.event_type,
                severity,
                event.payload,
            );
            points.push(event_point);
        }

        Ok(points)
    }
}
