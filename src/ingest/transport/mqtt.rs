//! MQTT Transport - 简化版

use anyhow::Result;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions};
use std::sync::Arc;

use crate::ingest::transport::{Transport, TransportContext};
use crate::core::value_object::DeviceType;

pub struct MqttTransport {
    pub broker: String,
    pub port: u16,
    pub client_id: String,
    pub topic_prefix: String,
}

impl MqttTransport {
    pub fn new(broker: String, port: u16, client_id: String, topic_prefix: String) -> Self {
        Self { broker, port, client_id, topic_prefix }
    }
}

#[async_trait::async_trait]
impl Transport for MqttTransport {
    async fn start(&self, ctx: TransportContext) -> Result<()> {
        let mut options = MqttOptions::new(&self.client_id, &self.broker, self.port);
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let (_client, mut eventloop) = AsyncClient::new(options, 10);
        let topic_prefix = self.topic_prefix.clone();
        let device_manager = ctx.device_manager.clone();

        tokio::spawn(async move {
            log::info!("mqtt transport start");
            while let Ok(notification) = eventloop.poll().await {
                if let Event::Incoming(Incoming::Publish(publish)) = notification {
                    let device_manager = device_manager.clone();
                    let topic_prefix = topic_prefix.clone();
                    let payload = publish.payload.to_vec();
                    let topic = publish.topic.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_message(topic_prefix, device_manager, topic, payload).await {
                            log::error!("mqtt error: {}", e);
                        }
                    });
                }
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

fn get_device_type_from_payload(payload: &[u8]) -> String {
    // 单独函数，避免生命周期问题
    if let Ok(msg) = serde_json::from_slice::<serde_json::Value>(payload) {
        if let Some(v) = msg.get("device_type") {
            if let Some(s) = v.as_str() {
                return s.to_string();
            }
        }
    }
    "unknown".to_string()
}

async fn handle_message(
    topic_prefix: String,
    device_manager: Arc<crate::ingest::DeviceManager>,
    topic: String,
    payload: Vec<u8>,
) -> Result<(), anyhow::Error> {
    let expected_prefix = format!("{}/", topic_prefix);
    if !topic.starts_with(&expected_prefix) {
        return Ok(());
    }

    // 解析 topic
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() < 3 {
        return Ok(());
    }
    
    let serial_number = parts[1];
    let topic_type = parts[2];

    // 解析设备类型
    let device_type_str: String = if topic_type == "event" {
        "fall_detector".to_string()
    } else {
        get_device_type_from_payload(&payload)
    };

    let device_type = DeviceType::from_str(&device_type_str)
        .ok_or_else(|| anyhow::anyhow!("未知设备类型: {}", device_type_str))?;

    device_manager
        .process(serial_number, device_type, payload, "mqtt")
        .await?;

    Ok(())
}
