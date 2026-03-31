//! 有状态适配器模板
//!
//! 适用于需要状态管理和事件分析的设备（如床垫）

use crate::core::entity::DataPoint;
use crate::errors::AppResult;
use crate::ingest::adapter::{DeviceAdapter, DeviceMetadata, DeviceType};
use crate::ingest::state::{DeviceEvent, DeviceState, EventSeverity};
use crate::ingest::ParsedData;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;

/// 有状态适配器
///
/// S: 状态类型
/// E: 事件引擎
pub struct StatefulAdapter<S, E>
where
    S: DeviceState + Clone + 'static,
    E: EventEngine<S>,
{
    metadata: DeviceMetadata,
    event_engine: E,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, E> StatefulAdapter<S, E>
where
    S: DeviceState + Clone + 'static,
    E: EventEngine<S>,
{
    pub fn new(
        device_type: DeviceType,
        display_name: impl Into<String>,
        event_engine: E,
    ) -> Self {
        Self {
            metadata: DeviceMetadata {
                device_type,
                display_name: display_name.into(),
                description: "有状态适配器".to_string(),
                protocol_version: "1.0".to_string(),
                supports_events: true,
            },
            event_engine,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// 事件引擎 trait
#[async_trait]
pub trait EventEngine<S: DeviceState>: Send + Sync {
    /// 处理数据并生成事件
    fn process(
        &self,
        state: &mut S,
        data: &serde_json::Value,
    ) -> AppResult<Vec<DeviceEvent>>;
}

/// 数据转换器 trait
pub trait DataTransformer: Send + Sync {
    /// 将原始数据转换为结构化JSON
    fn transform(&self, raw: &[u8]) -> AppResult<serde_json::Value>;
}

/// JSON转换器
pub struct JsonTransformer;

impl DataTransformer for JsonTransformer {
    fn transform(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        serde_json::from_slice(raw)
            .map_err(|e| crate::errors::AppError::ValidationError(format!("JSON解析失败: {}", e)))
    }
}

/// MessagePack转换器
pub struct MessagePackTransformer;

impl DataTransformer for MessagePackTransformer {
    fn transform(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        rmp_serde::from_slice(raw)
            .map_err(|e| crate::errors::AppError::ValidationError(format!("MessagePack解析失败: {}", e)))
    }
}

/// 构建有状态适配器的Builder
pub struct StatefulAdapterBuilder<S, E>
where
    S: DeviceState + Clone + 'static,
    E: EventEngine<S>,
{
    device_type: DeviceType,
    display_name: String,
    event_engine: E,
    transformer: Option<Box<dyn DataTransformer>>,
    protocol_decoder: Option<Box<dyn crate::ingest::protocol::ProtocolDecoder>>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, E> StatefulAdapterBuilder<S, E>
where
    S: DeviceState + Clone + 'static,
    E: EventEngine<S>,
{
    pub fn new(
        device_type: DeviceType,
        display_name: impl Into<String>,
        event_engine: E,
    ) -> Self {
        Self {
            device_type,
            display_name: display_name.into(),
            event_engine,
            transformer: None,
            protocol_decoder: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_transformer(
        mut self,
        transformer: impl DataTransformer + 'static,
    ) -> Self {
        self.transformer = Some(Box::new(transformer));
        self
    }

    pub fn with_protocol_decoder(
        mut self,
        decoder: impl crate::ingest::protocol::ProtocolDecoder + 'static,
    ) -> Self {
        self.protocol_decoder = Some(Box::new(decoder));
        self
    }

    pub fn build(self) -> StatefulAdapter<S, E> {
        StatefulAdapter::new(
            self.device_type,
            self.display_name,
            self.event_engine,
        )
    }
}

/// 通用有状态适配器实现（使用泛型）
pub struct GenericStatefulAdapter<S>
where
    S: DeviceState + Clone + 'static,
{
    device_type: DeviceType,
    display_name: String,
    transformer: Box<dyn DataTransformer>,
    event_processor: Box<dyn Fn(&mut S,
&serde_json::Value) -> AppResult<Vec<DeviceEvent>> + Send + Sync>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S> GenericStatefulAdapter<S>
where
    S: DeviceState + Clone + 'static,
{
    pub fn new<F>(
        device_type: DeviceType,
        display_name: impl Into<String>,
        transformer: impl DataTransformer + 'static,
        event_processor: F,
    ) -> Self
    where
        F: Fn(&mut S, &serde_json::Value) -> AppResult<Vec<DeviceEvent>> + Send + Sync + 'static,
    {
        Self {
            device_type,
            display_name: display_name.into(),
            transformer: Box::new(transformer),
            event_processor: Box::new(event_processor),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<S> DeviceAdapter for GenericStatefulAdapter<S>
where
    S: DeviceState + Clone + Default + Send + Sync + 'static,
{
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: self.device_type.clone(),
            display_name: self.display_name.clone(),
            description: "泛型有状态适配器".to_string(),
            protocol_version: "1.0".to_string(),
            supports_events: true,
        }
    }

    fn is_stateful(&self) -> bool {
        true
    }

    fn create_state(&self) -> Option<Box<dyn DeviceState>> {
        Some(Box::new(S::default()))
    }

    fn parse(&self, raw: &[u8]) -> AppResult<ParsedData> {
        let value = self.transformer.transform(raw)?;

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

    async fn process_with_state(
        &self,
        data: ParsedData,
        state: &mut dyn DeviceState,
    ) -> AppResult<Vec<DataPoint>> {
        let typed_state = state
            .as_any_mut()
            .downcast_mut::<S>()
            .ok_or_else(|| crate::errors::AppError::InternalError)?;

        // 生成事件
        let events = (self.event_processor)(typed_state, &data.payload)?;

        // 构造数据点
        let mut points = vec![DataPoint {
            time: Utc::now(),
            device_id: None,
            patient_id: None,
            data_type: data.device_type.clone(),
            data_category: crate::core::entity::DataCategory::Metric,
            value_numeric: data.payload.get("value").and_then(|v| v.as_f64()),
            value_text: None,
            severity: None,
            status: None,
            payload: data.payload,
            source: "ingest".to_string(),
        }];

        // 添加事件数据点
        for event in events {
            let event_point = DataPoint {
                time: event.timestamp,
                device_id: None,
                patient_id: None,
                data_type: event.event_type,
                data_category: crate::core::entity::DataCategory::Event,
                value_numeric: None,
                value_text: None,
                severity: Some(match event.severity {
                    EventSeverity::Info => crate::core::entity::Severity::Info,
                    EventSeverity::Warning => crate::core::entity::Severity::Warning,
                    EventSeverity::Critical => crate::core::entity::Severity::Alert,
                }),
                status: Some(crate::core::entity::EventStatus::Active),
                payload: event.payload,
                source: "ingest".to_string(),
            };
            points.push(event_point);
        }

        Ok(points)
    }
}
