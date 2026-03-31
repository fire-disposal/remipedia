//! TCP Transport V2 - 简化版

use crate::errors::{AppError, AppResult};
use crate::ingest::{DataPacket, IngestionPipeline};
use crate::ingest::protocol::ProtocolDecoder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

pub struct TcpTransportV2 {
    bind: SocketAddr,
}

impl TcpTransportV2 {
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
            .map_err(|_| AppError::InternalError)?;

        log::info!("TCP Transport启动: {}", self.bind);

        loop {
            let (stream, addr) = listener.accept().await
                .map_err(|_| AppError::InternalError)?;

            let pipeline = pipeline.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, pipeline).await {
                    log::error!("TCP连接处理错误 {}: {}", addr, e);
                }
            });
        }
    }

    async fn handle_connection(
        mut stream: TcpStream,
        addr: SocketAddr,
        pipeline: Arc<IngestionPipeline>,
    ) -> AppResult<()> {
        let mut buffer = Vec::with_capacity(4096);
        let mut temp_buf = [0u8; 1024];

        loop {
            let n = stream.read(&mut temp_buf).await
                .map_err(|_| AppError::InternalError)?;

            if n == 0 {
                log::info!("TCP连接关闭: {}", addr);
                break;
            }

            buffer.extend_from_slice(&temp_buf[..n]);

            // 简单处理：直接提交原始数据
            // 协议解码在Pipeline中通过Adapter处理
            let data = std::mem::take(&mut buffer);
            let packet = DataPacket::new(data, "tcp");

            if let Err(e) = pipeline.submit(packet) {
                log::warn!("提交到Pipeline失败: {}", e);
            }

            buffer = Vec::with_capacity(4096);
        }

        Ok(())
    }
}
