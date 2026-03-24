//! MQTT 数据接入客户端
//!
//! ## 支持的消息格式
//!
//! ### 通用格式 (Topic: `{prefix}/{serial_number}/data`)
//! ```json
//! {
//!     "device_type": "heart_rate_monitor",
//!     "timestamp": "2024-01-15T10:30:00Z",
//!     "data": [1, 2, 3, ...]
//! }
//! ```
//!
//! ### 跌倒检测器格式 (Topic: `{prefix}/{serial_number}/event`)
//! ```json
//! {
//!     "event_type": "person_fall",
//!     "confidence": 0.85,
//!     "timestamp": "2024-01-15T10:30:00Z"
//! }
//! ```

use log::{error, info, warn};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::MqttConfig;
use crate::core::value_object::DeviceType;
use crate::errors::AppError;
use crate::ingest::adapters::AdapterRegistry;
use crate::service::{BindingService, DataService, DeviceService};

/// MQTT 数据接入客户端
pub struct MqttIngest {
    client: AsyncClient,
    topic_prefix: String,
}

impl MqttIngest {
    /// 创建 MQTT 客户端
    pub async fn new(pool: Arc<PgPool>, config: &MqttConfig) -> Self {
        let mut options = MqttOptions::new(&config.client_id, &config.broker, config.port);
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, mut eventloop) = AsyncClient::new(options, 10);
        let topic_prefix = config.topic_prefix.clone();

        // 启动事件循环
        let pool_clone = pool.clone();
        let topic_prefix_clone = topic_prefix.clone();

        tokio::spawn(async move {
            info!("MQTT 事件循环启动");

            while let Ok(notification) = eventloop.poll().await {
                if let Event::Incoming(Incoming::Publish(publish)) = notification {
                    Self::handle_message(
                        &pool_clone,
                        &topic_prefix_clone,
                        &publish.topic,
                        &publish.payload,
                    )
                    .await;
                }
            }

            warn!("MQTT 事件循环结束");
        });

        Self {
            client,
            topic_prefix,
        }
    }

    /// 订阅主题
    pub async fn subscribe(&self) {
        // 订阅通用数据主题
        let data_topic = format!("{}/+/data", self.topic_prefix);
        self.client
            .subscribe(&data_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        info!("已订阅 MQTT 主题: {}", data_topic);

        // 订阅事件主题（用于跌倒检测器等事件驱动设备）
        let event_topic = format!("{}/+/event", self.topic_prefix);
        self.client
            .subscribe(&event_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        info!("已订阅 MQTT 主题: {}", event_topic);
    }

    /// 处理消息
    async fn handle_message(pool: &PgPool, topic_prefix: &str, topic: &str, payload: &[u8]) {
        if let Err(e) = Self::process_message(pool, topic_prefix, topic, payload).await {
            error!("处理 MQTT 消息失败: {}, topic: {}", e, topic);
        }
    }

    /// 处理消息
    async fn process_message(
        pool: &PgPool,
        topic_prefix: &str,
        topic: &str,
        payload: &[u8],
    ) -> Result<(), AppError> {
        // 解析 Topic: {prefix}/{serial_number}/{topic_type}
        let expected_prefix = format!("{}/", topic_prefix);
        if !topic.starts_with(&expected_prefix) {
            return Err(AppError::ValidationError("无效的 Topic 格式".into()));
        }

        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() < 3 {
            return Err(AppError::ValidationError("无效的 Topic 格式".into()));
        }

        let serial_number = parts[1];
        let topic_type = parts[2]; // "data" 或 "event"

        // 解析消息（JSON 格式）
        let msg: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| AppError::ValidationError(format!("消息解析失败: {}", e)))?;

        // 根据主题类型和消息内容确定设备类型
        let (device_type, timestamp, raw_data) = match topic_type {
            "event" => {
                // 事件主题：用于跌倒检测器等事件驱动设备
                // 消息格式: {"event_type": "person_fall", "confidence": 0.85, "timestamp": "..."}
                Self::parse_event_message(&msg)?
            }
            "data" | _ => {
                // 数据主题：通用格式
                // 消息格式: {"device_type": "...", "timestamp": "...", "data": [...]}
                Self::parse_data_message(&msg)?
            }
        };

        // 自动注册或获取设备
        let device_service = DeviceService::new(pool);
        let device = device_service
            .auto_register_or_get(serial_number, &device_type)
            .await?;

        // 获取适配器
        let dev_type = DeviceType::from_str(&device.device_type)
            .ok_or_else(|| AppError::ValidationError("无效设备类型".into()))?;

        let registry = AdapterRegistry::new();
        let adapter = registry
            .get(&dev_type)
            .ok_or_else(|| AppError::ValidationError("无对应适配器".into()))?;

        // 解析并验证数据
        let data_payload = adapter.parse_payload(&raw_data)?;
        adapter.validate(&data_payload)?;

        // 获取当前绑定的患者
        let binding_service = BindingService::new(pool);
        let subject_id = binding_service
            .get_current_binding_subject(&device.id)
            .await?;

        // 存储数据
        let data_service = DataService::new(pool);
        data_service
            .ingest(crate::core::entity::IngestData {
                time: timestamp,
                device_id: device.id,
                subject_id,
                data_type: adapter.data_type().to_string(),
                payload: data_payload,
                source: "mqtt".to_string(),
            })
            .await?;

        info!(
            "数据入库成功: device_id={}, subject_id={:?}, data_type={}",
            device.id,
            subject_id,
            adapter.data_type()
        );

        Ok(())
    }

    /// 解析事件主题消息（跌倒检测器等）
    fn parse_event_message(
        msg: &serde_json::Value,
    ) -> Result<(String, chrono::DateTime<chrono::Utc>, Vec<u8>), AppError> {
        // 事件主题的消息本身就是事件数据，设备类型固定为 fall_detector
        let device_type = "fall_detector".to_string();

        let timestamp = msg["timestamp"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        // 整个消息作为原始数据传递给适配器
        let raw_data = serde_json::to_vec(msg)
            .map_err(|e| AppError::ValidationError(format!("序列化消息失败: {}", e)))?;

        Ok((device_type, timestamp, raw_data))
    }

    /// 解析数据主题消息（通用格式）
    fn parse_data_message(
        msg: &serde_json::Value,
    ) -> Result<(String, chrono::DateTime<chrono::Utc>, Vec<u8>), AppError> {
        let device_type = msg["device_type"]
            .as_str()
            .ok_or_else(|| AppError::ValidationError("缺少 device_type".into()))?
            .to_string();

        let timestamp = msg["timestamp"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let raw_data: Vec<u8> = msg["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect()
            })
            .unwrap_or_default();

        Ok((device_type, timestamp, raw_data))
    }
}