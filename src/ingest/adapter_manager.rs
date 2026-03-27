use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc::{self, Sender};

use crate::core::entity::IngestData;
use crate::core::value_object::DeviceType;
use crate::errors::AppError;
use crate::ingest::adapters::{AdapterOutput, AdapterRegistry, DeviceAdapter, MessagePayload};
use crate::service::{BindingService, DataService, DeviceService};

const WORKER_QUEUE_SIZE: usize = 1024;

/// 入站消息（从 MQTT/TCP 分发到适配器 worker）
pub struct InboundMessage {
    pub time: DateTime<Utc>,
    pub device_id: uuid::Uuid,
    pub subject_id: Option<uuid::Uuid>,
    pub device_type: String,
    pub raw_payload: Vec<u8>,
    pub source: String,
}

/// 每个设备类型对应独立 worker 的 ingest 管理器。
///
/// 设计目标：
/// 1. transport 只做接入，不做业务解析；
/// 2. adapter 只做 parse + validate；
/// 3. manager 负责流水化处理（parse -> validate -> persist）。
pub struct AdapterManager {
    senders: HashMap<String, Sender<InboundMessage>>,
    pool: Arc<sqlx::PgPool>,
}

impl AdapterManager {
    pub fn new(pool: Arc<sqlx::PgPool>, registry: Arc<AdapterRegistry>) -> Arc<Self> {
        let senders = registry
            .iter()
            .into_iter()
            .map(|(device_type, adapter)| {
                let worker_device_type = device_type.as_str().to_string();
                let (tx, rx) = mpsc::channel::<InboundMessage>(WORKER_QUEUE_SIZE);
                let pool_clone = pool.clone();
                tokio::spawn(run_device_worker(
                    worker_device_type.clone(),
                    adapter,
                    pool_clone,
                    rx,
                ));
                (worker_device_type, tx)
            })
            .collect();

        Arc::new(Self { senders, pool })
    }

    /// 通过 serial_number 自动注册/获取设备并分发原始消息。
    pub async fn dispatch_by_serial(
        &self,
        serial_number: &str,
        device_type: &str,
        raw_payload: Vec<u8>,
        source: &str,
    ) -> Result<(), AppError> {
        let normalized_type = DeviceType::from_str(device_type)
            .map(|dt| dt.as_str().to_string())
            .ok_or_else(|| AppError::ValidationError(format!("未知设备类型: {}", device_type)))?;

        if !self.senders.contains_key(&normalized_type) {
            return Err(AppError::ValidationError(format!(
                "无对应适配器: {}",
                normalized_type
            )));
        }

        let device_service = DeviceService::new(self.pool.as_ref());
        let binding_service = BindingService::new(self.pool.as_ref());
        let device = device_service
            .auto_register_or_get(serial_number, &normalized_type)
            .await?;
        let subject_id = binding_service
            .get_current_binding_subject(&device.id)
            .await?;

        let inbound = InboundMessage {
            time: Utc::now(),
            device_id: device.id,
            subject_id,
            device_type: device.device_type,
            raw_payload,
            source: source.to_string(),
        };

        let device_type = inbound.device_type.clone();
        self.dispatch(&device_type, inbound).await
    }

    /// dispatch 一条消息到对应设备 worker。
    pub async fn dispatch(&self, device_type: &str, msg: InboundMessage) -> Result<(), AppError> {
        if let Some(tx) = self.senders.get(device_type) {
            tx.send(msg).await.map_err(|_| AppError::InternalError)
        } else {
            Err(AppError::ValidationError(format!(
                "无对应适配器: {}",
                device_type
            )))
        }
    }
}

async fn run_device_worker(
    device_type: String,
    adapter: Arc<dyn DeviceAdapter>,
    pool: Arc<sqlx::PgPool>,
    mut rx: tokio::sync::mpsc::Receiver<InboundMessage>,
) {
    let data_service = DataService::new(&pool);

    while let Some(msg) = rx.recv().await {
        if let Err(err) = process_message(&data_service, adapter.clone(), msg).await {
            log::error!(
                "ingest pipeline failed: device_type={}, err={}",
                device_type,
                err
            );
        }
    }

    log::warn!("ingest worker stopped: {}", device_type);
}

async fn process_message(
    data_service: &DataService<'_>,
    adapter: Arc<dyn DeviceAdapter>,
    msg: InboundMessage,
) -> Result<(), AppError> {
    let output = parse_stage(adapter.clone(), &msg.raw_payload).await?;
    validate_stage(adapter, &output)?;
    persist_stage(data_service, &msg, output).await
}

async fn parse_stage(
    adapter: Arc<dyn DeviceAdapter>,
    raw: &[u8],
) -> Result<AdapterOutput, AppError> {
    let raw_owned = raw.to_vec();
    tokio::task::spawn_blocking(move || adapter.parse(&raw_owned))
        .await
        .map_err(|e| AppError::ValidationError(format!("parse join error: {}", e)))?
}

fn validate_stage(adapter: Arc<dyn DeviceAdapter>, output: &AdapterOutput) -> Result<(), AppError> {
    adapter.validate(output)
}

async fn persist_stage(
    data_service: &DataService<'_>,
    msg: &InboundMessage,
    output: AdapterOutput,
) -> Result<(), AppError> {
    match output {
        AdapterOutput::Messages(messages) => {
            for payload in messages {
                let ingest_data = map_payload_to_ingest(msg, payload);
                if let Err(err) = data_service.ingest(ingest_data).await {
                    log::error!("ingest persist failed: {}", err);
                }
            }
        }
    }

    Ok(())
}

fn map_payload_to_ingest(msg: &InboundMessage, payload: MessagePayload) -> IngestData {
    let mut normalized_payload = payload.payload;

    if normalized_payload.is_object() {
        if let Some(map) = normalized_payload.as_object_mut() {
            if let Some(message_type) = payload.message_type {
                map.insert(
                    "message_type".to_string(),
                    serde_json::Value::String(message_type),
                );
            }
            if let Some(severity) = payload.severity {
                map.insert("severity".to_string(), serde_json::Value::String(severity));
            }
        }
    } else {
        normalized_payload = serde_json::json!({
            "value": normalized_payload,
            "message_type": payload.message_type,
            "severity": payload.severity,
        });
    }

    IngestData {
        time: payload.time,
        device_id: msg.device_id,
        subject_id: msg.subject_id,
        data_type: payload.data_type,
        payload: normalized_payload,
        source: msg.source.clone(),
    }
}
