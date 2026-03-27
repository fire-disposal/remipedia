/// AdapterManager: 负责把外部 Transport 层发送过来的原始帧按 device_type 分发到
/// 对应适配器的 worker。设计要点：
/// - 每种 device_type 一个有界 channel + worker（避免单个慢适配器导致全局阻塞）
/// - 在 worker 内使用 `spawn_blocking` 调用适配器的 `parse`，将 CPU/阻塞操作移出 async runtime
/// - Worker 负责把 `AdapterOutput` 转换为 `IngestData` 并调用 `DataService::ingest`
///
/// 约束与扩展点：
/// - `AdapterRegistry` 应保持单一来源（建议在应用启动时创建并注入，而不是多次构建）
/// - `device_type` 字符串必须与 `AdapterRegistry` 的 key 保持一致（优先使用 `DeviceType::as_str()`）
use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc::{self, Sender};

use crate::core::entity::IngestData;
use crate::errors::AppError;
use crate::ingest::adapters::{AdapterRegistry, DeviceAdapter, MessagePayload};
use crate::service::DataService;

/// 入站消息（从 MQTT/TCP 分发到适配器 worker）
pub struct InboundMessage {
    pub time: DateTime<Utc>,
    pub device_id: uuid::Uuid,
    pub subject_id: Option<uuid::Uuid>,
    pub device_type: String,
    pub raw_payload: Vec<u8>,
    pub source: String,
}

/// 简单的适配器管理器：为每种设备类型启动一个 worker，接收原始消息并执行解析/入库。
pub struct AdapterManager {
    senders: HashMap<String, Sender<InboundMessage>>,
}

impl AdapterManager {
    /// 创建并启动管理器（为注册表中的每个适配器启动 worker）。
    pub fn new(pool: Arc<sqlx::PgPool>) -> Arc<Self> {
        let registry = AdapterRegistry::new();
        let mut senders: HashMap<String, Sender<InboundMessage>> = HashMap::new();

        for (device_type, adapter) in registry.iter().into_iter() {
            let dt = device_type.as_str().to_string();
            // 有界通道，避免无界堆积
            let (tx, mut rx) = mpsc::channel::<InboundMessage>(500);
            let pool_clone = pool.clone();
            let adapter_arc = adapter.clone();

            // spawn worker
            tokio::spawn(async move {
                let data_service = DataService::new(&pool_clone);
                while let Some(msg) = rx.recv().await {
                    // 把解析/复杂处理移出 async runtime，使用 blocking 线程池执行 parse
                    let adapter_clone = adapter_arc.clone();
                    let raw_payload = msg.raw_payload.clone();

                    let parse_result = tokio::task::spawn_blocking(move || adapter_clone.parse(&raw_payload))
                        .await;

                    let output = match parse_result {
                        Ok(Ok(output)) => output,
                        Ok(Err(e)) => {
                            log::error!("适配器解析失败: {}", e);
                            continue;
                        }
                        Err(e) => {
                            log::error!("spawn_blocking join error: {}", e);
                            continue;
                        }
                    };

                    if let Err(e) = adapter_arc.validate(&output) {
                        log::error!("适配器验证失败: {}", e);
                        continue;
                    }

                    match output {
                        crate::ingest::adapters::AdapterOutput::Messages(msgs) => {
                            for m in msgs {
                                let mut payload = if m.payload.is_object() {
                                    let mut map = m.payload.as_object().cloned().unwrap_or_default();
                                    if let Some(mt) = &m.message_type {
                                        map.insert("message_type".to_string(), serde_json::Value::String(mt.clone()));
                                    }
                                    if let Some(sev) = &m.severity {
                                        map.insert("severity".to_string(), serde_json::Value::String(sev.clone()));
                                    }
                                    serde_json::Value::Object(map)
                                } else {
                                    let mut map = serde_json::Map::new();
                                    map.insert("value".to_string(), m.payload.clone());
                                    if let Some(mt) = &m.message_type {
                                        map.insert("message_type".to_string(), serde_json::Value::String(mt.clone()));
                                    }
                                    if let Some(sev) = &m.severity {
                                        map.insert("severity".to_string(), serde_json::Value::String(sev.clone()));
                                    }
                                    serde_json::Value::Object(map)
                                };

                                let ingest_data = IngestData {
                                    time: m.time,
                                    device_id: msg.device_id,
                                    subject_id: msg.subject_id,
                                    data_type: m.data_type.clone(),
                                    payload,
                                    source: msg.source.clone(),
                                };

                                if let Err(e) = data_service.ingest(ingest_data).await {
                                    log::error!("入库失败: {}", e);
                                }
                            }
                        }
                    }
                }
            });

            senders.insert(dt, tx);
        }

        Arc::new(Self { senders })
    }

    /// Dispatch 一条消息到对应设备类型的 worker。
    pub async fn dispatch(&self, device_type: &str, msg: InboundMessage) -> Result<(), AppError> {
        if let Some(tx) = self.senders.get(device_type) {
            // try_send 不适用于 async mpsc::Sender; 使用 try_send via try_send if needed, but这里使用 send().await
            tx.send(msg).await.map_err(|_| AppError::InternalError)
        } else {
            Err(AppError::ValidationError(format!("无对应适配器: {}", device_type)))
        }
    }
}
