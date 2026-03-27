use anyhow::Result;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions};
use std::sync::Arc;

use crate::ingest::transport::{Transport, TransportContext};
use crate::ingest::AdapterManager;

pub struct MqttTransport {
    pub broker: String,
    pub port: u16,
    pub client_id: String,
    pub topic_prefix: String,
}

impl MqttTransport {
    pub fn new(broker: String, port: u16, client_id: String, topic_prefix: String) -> Self {
        Self {
            broker,
            port,
            client_id,
            topic_prefix,
        }
    }
}

#[async_trait::async_trait]
impl Transport for MqttTransport {
    async fn start(&self, ctx: TransportContext) -> Result<()> {
        let mut options = MqttOptions::new(&self.client_id, &self.broker, self.port);
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let (_client, mut eventloop) = AsyncClient::new(options, 10);
        let topic_prefix = self.topic_prefix.clone();

        tokio::spawn(async move {
            log::info!("mqtt transport event loop start");
            while let Ok(notification) = eventloop.poll().await {
                if let Event::Incoming(Incoming::Publish(publish)) = notification {
                    // handle message: parse topic and dispatch to adapter manager
                    if let Err(e) = handle_message(
                        &topic_prefix,
                        &ctx.manager,
                        &publish.topic,
                        &publish.payload,
                    )
                    .await
                    {
                        log::error!("mqtt handle message error: {}", e);
                    }
                }
            }
            log::warn!("mqtt event loop ended");
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

async fn handle_message(
    topic_prefix: &str,
    manager: &Arc<AdapterManager>,
    topic: &str,
    payload: &[u8],
) -> Result<(), anyhow::Error> {
    // validate prefix
    let expected_prefix = format!("{}/", topic_prefix);
    if !topic.starts_with(&expected_prefix) {
        return Ok(());
    }

    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() < 3 {
        return Ok(());
    }

    let serial_number = parts[1];
    let topic_type = parts[2];

    // parse message
    let msg: serde_json::Value = serde_json::from_slice(payload).map_err(|e| anyhow::anyhow!(e))?;

    let device_type = if topic_type == "event" {
        "fall_detector".to_string()
    } else {
        msg["device_type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing device_type"))?
            .to_string()
    };

    manager
        .dispatch_by_serial(serial_number, &device_type, payload.to_vec(), "mqtt")
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
