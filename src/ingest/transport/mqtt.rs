//! MQTT Transport V2 - 简化版

use crate::errors::{AppError, AppResult};
use crate::ingest::{DataPacket, IngestionPipeline};
use std::sync::Arc;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions};

pub struct MqttTransportV2 {
    broker: String,
    port: u16,
    client_id: String,
}

impl MqttTransportV2 {
    pub fn new(
        broker: impl Into<String>,
        port: u16,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            broker: broker.into(),
            port,
            client_id: client_id.into(),
        }
    }

    pub async fn start(
        &self,
        pipeline: Arc<IngestionPipeline>,
    ) -> AppResult<()> {
        let mut options = MqttOptions::new(
            &self.client_id,
            &self.broker,
            self.port,
        );
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, mut eventloop) = AsyncClient::new(options, 10);

        // 订阅主题
        client.subscribe("devices/+/+", rumqttc::QoS::AtLeastOnce).await
            .map_err(|e| AppError::InternalError)?;

        log::info!("MQTT Transport启动: {}:{}", self.broker, self.port);

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let pipeline = pipeline.clone();
                    let topic = publish.topic.clone();
                    let payload = publish.payload.to_vec();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_message(
                            &topic,
                            payload,
                            pipeline,
                        ).await {
                            log::error!("MQTT消息处理错误: {}", e);
                        }
                    });
                }
                Ok(_) => {}
                Err(e) => {
                    log::error!("MQTT错误: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn handle_message(
        topic: &str,
        payload: Vec<u8>,
        pipeline: Arc<IngestionPipeline>,
    ) -> AppResult<()> {
        // 解析topic: devices/{serial}/{type}
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() < 3 {
            return Ok(());
        }

        let serial = parts[1];
        let msg_type = parts[2];

        // 推断设备类型
        let device_type = if msg_type == "event" {
            "fall_detector"
        } else {
            // 从payload解析
            Self::infer_device_type(&payload).unwrap_or("unknown")
        };

        let packet = DataPacket::new(payload, "mqtt")
            .with_serial(serial)
            .with_device_type(device_type);

        pipeline.submit(packet)?;

        Ok(())
    }

    fn infer_device_type(payload: &[u8]) -> Option<&'static str> {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(payload) {
            json.get("device_type")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "smart_mattress" => "smart_mattress",
                    "heart_rate_monitor" => "heart_rate_monitor",
                    _ => "unknown",
                })
        } else {
            None
        }
    }
}
