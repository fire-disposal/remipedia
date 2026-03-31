//! WebSocket Transport V2 - 简化版

use crate::errors::{AppError, AppResult};
use crate::ingest::{DataPacket, IngestionPipeline};
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

pub struct WebSocketTransportV2 {
    bind: SocketAddr,
}

impl WebSocketTransportV2 {
    pub fn new(bind: impl Into<SocketAddr>) -> Self {
        Self {
            bind: bind.into(),
        }
    }

    pub async fn start(
        &self,
        pipeline: Arc<IngestionPipeline>,
    ) -> AppResult<()> {
        let listener = TcpListener::bind(self.bind).await
            .map_err(|_e| AppError::InternalError)?;

        log::info!("WebSocket Transport启动: {}", self.bind);

        loop {
            let (stream, addr) = listener.accept().await
                .map_err(|_e| AppError::InternalError)?;

            let pipeline = pipeline.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, pipeline).await {
                    log::error!("WebSocket连接错误 {}: {}", addr, e);
                }
            });
        }
    }

    async fn handle_connection(
        stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        pipeline: Arc<IngestionPipeline>,
    ) -> AppResult<()> {
        let ws_stream = accept_async(stream).await
            .map_err(|_e| AppError::InternalError)?;

        log::info!("WebSocket连接建立: {}", addr);

        let (_write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let msg = msg.map_err(|_e| AppError::InternalError)?;

            if msg.is_text() {
                let payload = msg.into_data();

                // 从JSON提取设备信息
                let (serial, device_type) = if let Ok(json) =
                    serde_json::from_slice::<serde_json::Value>(&payload)
                {
                    let serial = json
                        .get("serial_number")
                        .or_else(|| json.get("sn"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let device_type = json
                        .get("device_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    (serial, device_type)
                } else {
                    ("unknown".to_string(), "unknown".to_string())
                };

                let packet = DataPacket::new(payload.to_vec(), "websocket")
                    .with_serial(serial)
                    .with_device_type(device_type);

                if let Err(e) = pipeline.submit(packet) {
                    log::warn!("提交到Pipeline失败: {}", e);
                }
            } else if msg.is_close() {
                break;
            }
        }

        log::info!("WebSocket连接关闭: {}", addr);
        Ok(())
    }
}
