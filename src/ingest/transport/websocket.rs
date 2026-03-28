//! WebSocket Transport

use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::ingest::transport::{Transport, TransportContext};
use crate::core::value_object::DeviceTypeId;

pub struct WebSocketTransport {
    pub bind: String,
    pub default_device_type: &'static str,
}

impl WebSocketTransport {
    pub fn new(bind: String) -> Self {
        Self {
            bind,
            default_device_type: "smart_mattress",
        }
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn start(&self, ctx: TransportContext) -> Result<()> {
        let addr: SocketAddr = self.bind.parse()?;
        let listener = TcpListener::bind(addr).await?;
        log::info!("websocket transport listening on {}", self.bind);

        let (broadcast_tx, _) = broadcast::channel(1000);
        let broadcast_tx_clone = broadcast_tx.clone();

        let device_manager = ctx.device_manager.clone();
        let default_type = self.default_device_type.to_string();

        tokio::spawn(async move {
            loop {
                if let Ok((stream, addr)) = listener.accept().await {
                    let tx = broadcast_tx_clone.clone();
                    let dm = device_manager.clone();
                    let dt = default_type.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, dm, dt, tx).await {
                            log::error!("ws connection error {}: {}", addr, e);
                        }
                    });
                }
            }
        });

        let dm = ctx.device_manager;
        let mut rx = broadcast_tx.subscribe();

        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if let Ok(text) = msg.into_text() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let serial = json.get("serial_number")
                            .or_else(|| json.get("sn"))
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        if let Some(serial) = serial {
                            if let Ok(packet) = serde_json::to_vec(&json) {
                                let _ = dm.process(
                                    &serial,
                                    DeviceTypeId::new(DeviceTypeId::SMART_MATTRESS),
                                    packet,
                                    "websocket"
                                ).await;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    device_manager: Arc<crate::ingest::DeviceManager>,
    default_type: String,
    broadcast_tx: broadcast::Sender<Message>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (_write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    let serial = json.get("serial_number")
                        .or_else(|| json.get("sn"))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    if let Some(serial) = serial {
                        if let Ok(packet) = serde_json::to_vec(&json) {
                            let _ = device_manager.process(
                                &serial,
                                DeviceTypeId::new(default_type.clone()),
                                packet,
                                "websocket"
                            ).await;
                        }
                    }

                    let _ = broadcast_tx.send(Message::Text(text));
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                log::error!("ws read error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
