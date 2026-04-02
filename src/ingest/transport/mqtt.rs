//! MQTT Transport V2 - 简化版

use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::MqttAdapter;
use crate::ingest::IngestionPipeline;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions};
use std::sync::Arc;

pub struct MqttTransportV2 {
    broker: String,
    port: u16,
    client_id: String,
    mqtt_adapter: MqttAdapter,
}

impl MqttTransportV2 {
    pub fn new(
        broker: impl Into<String>,
        port: u16,
        client_id: impl Into<String>,
        topic_prefix: impl Into<String>,
    ) -> Self {
        Self {
            broker: broker.into(),
            port,
            client_id: client_id.into(),
            mqtt_adapter: MqttAdapter::new(topic_prefix),
        }
    }

    pub async fn start(&self, pipeline: Arc<IngestionPipeline>) -> AppResult<()> {
        let mut options = MqttOptions::new(&self.client_id, &self.broker, self.port);
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, mut eventloop) = AsyncClient::new(options, 10);

        // 订阅主题
        let topic = self.mqtt_adapter.subscribe_topic();
        client
            .subscribe(&topic, rumqttc::QoS::AtLeastOnce)
            .await
            .map_err(|_e| AppError::InternalError)?;

        log::info!(
            "MQTT Transport启动: {}:{}, topic={} ",
            self.broker,
            self.port,
            topic
        );

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let pipeline = pipeline.clone();
                    let topic = publish.topic.clone();
                    let payload = publish.payload.to_vec();
                    let mqtt_adapter = self.mqtt_adapter.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_message(mqtt_adapter, &topic, payload, pipeline).await
                        {
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
        mqtt_adapter: MqttAdapter,
        topic: &str,
        payload: Vec<u8>,
        pipeline: Arc<IngestionPipeline>,
    ) -> AppResult<()> {
        let Some(packet) = mqtt_adapter.to_packet(topic, payload) else {
            log::warn!("忽略无法解析 topic 的 MQTT 消息: {}", topic);
            return Ok(());
        };

        pipeline.submit(packet)?;

        Ok(())
    }
}
