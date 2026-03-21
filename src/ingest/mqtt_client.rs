use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info, warn};

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
                    ).await;
                }
            }
            
            warn!("MQTT 事件循环结束");
        });

        Self { client, topic_prefix }
    }

    /// 订阅主题
    pub async fn subscribe(&self) {
        let topic = format!("{}/+/data", self.topic_prefix);
        self.client.subscribe(&topic, QoS::AtLeastOnce).await.unwrap();
        info!(topic = %topic, "已订阅 MQTT 主题");
    }

    /// 处理消息
    async fn handle_message(
        pool: &PgPool,
        topic_prefix: &str,
        topic: &str,
        payload: &[u8],
    ) {
        if let Err(e) = Self::process_message(pool, topic_prefix, topic, payload).await {
            error!(error = %e, topic = %topic, "处理 MQTT 消息失败");
        }
    }

    /// 处理消息
    async fn process_message(
        pool: &PgPool,
        topic_prefix: &str,
        topic: &str,
        payload: &[u8],
    ) -> Result<(), AppError> {
        // 解析 Topic: {prefix}/{serial_number}/data
        let expected_prefix = format!("{}/", topic_prefix);
        if !topic.starts_with(&expected_prefix) {
            return Err(AppError::ValidationError("无效的 Topic 格式".into()));
        }

        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() < 3 {
            return Err(AppError::ValidationError("无效的 Topic 格式".into()));
        }

        let serial_number = parts[1];

        // 解析消息（JSON 格式）
        let msg: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| AppError::ValidationError(format!("消息解析失败: {}", e)))?;

        let device_type = msg["device_type"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少 device_type".into()))?;
        
        let timestamp = msg["timestamp"].as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);
        
        let raw_data: Vec<u8> = msg["data"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
            .unwrap_or_default();

        // 自动注册或获取设备
        let device_service = DeviceService::new(pool);
        let device = device_service.auto_register_or_get(serial_number, device_type).await?;

        // 获取适配器
        let dev_type = DeviceType::from_str(&device.device_type)
            .ok_or_else(|| AppError::ValidationError("无效设备类型".into()))?;
        
        let registry = AdapterRegistry::new();
        let adapter = registry.get(&dev_type)
            .ok_or_else(|| AppError::ValidationError("无对应适配器".into()))?;

        // 解析并验证数据
        let data_payload = adapter.parse_payload(&raw_data)?;
        adapter.validate(&data_payload)?;

        // 获取当前绑定的患者
        let binding_service = BindingService::new(pool);
        let subject_id = binding_service.get_current_binding_subject(&device.id).await?;

        // 存储数据
        let data_service = DataService::new(pool);
        data_service.ingest(crate::core::entity::IngestData {
            time: timestamp,
            device_id: device.id,
            subject_id,
            data_type: adapter.data_type().to_string(),
            payload: data_payload,
            source: "mqtt".to_string(),
        }).await?;

        info!(
            device_id = %device.id,
            subject_id = ?subject_id,
            data_type = %adapter.data_type(),
            "数据入库成功"
        );

        Ok(())
    }
}