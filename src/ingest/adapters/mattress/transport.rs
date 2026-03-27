use anyhow::Result;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

use crate::ingest::adapters::mattress::adapter::MattressAdapter;
use crate::ingest::adapters::mattress::decoder::decode_buffer;
use crate::ingest::adapters::DeviceAdapter;
use crate::ingest::transport::{Transport, TransportContext};

/// 兼容 Transport trait 的床垫 TCP 实现
pub struct MattressTransport {
    pub bind: String,
    pub adapter: Arc<MattressAdapter>,
}

impl MattressTransport {
    pub fn new(bind: String, adapter: Arc<MattressAdapter>) -> Self {
        Self { bind, adapter }
    }
}

#[async_trait::async_trait]
impl Transport for MattressTransport {
    async fn start(&self, _ctx: TransportContext) -> Result<()> {
        let listener = TcpListener::bind(&self.bind).await?;
        log::info!("mattress transport listening on {}", &self.bind);

        loop {
            let (mut socket, _addr) = listener.accept().await?;
            let adapter = self.adapter.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_socket(&mut socket, adapter).await {
                    log::error!("socket handler error: {}", e);
                }
            });
        }
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

async fn handle_socket(socket: &mut TcpStream, adapter: Arc<MattressAdapter>) -> Result<()> {
    let mut buf = BytesMut::with_capacity(8 * 1024);

    loop {
        let mut tmp = [0u8; 2048];
        let n = socket.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);

        // 尝试按 decoder 解码多个消息
        loop {
            let slice: &[u8] = &buf;
            match decode_buffer(slice) {
                Some((consumed, _maybe)) => {
                    // 拿出这段原始帧并交给适配器解析（阻塞式解析放到 spawn_blocking）
                    let raw_frame = buf.split_to(consumed).to_vec();
                    let adapter_clone = adapter.clone();
                    tokio::task::spawn_blocking(move || {
                        match adapter_clone.parse(&raw_frame) {
                            Ok(output) => {
                                // 目前简单记录输出，后续可接入事件总线/持久化/通知
                                log::debug!(
                                    "adapter produced output: {:?}",
                                    format!("{:?}", output)
                                );
                            }
                            Err(e) => {
                                log::warn!("adapter parse error: {:?}", e);
                            }
                        }
                    });
                    // continue to attempt decode more messages
                    continue;
                }
                None => break,
            }
        }
    }

    Ok(())
}
