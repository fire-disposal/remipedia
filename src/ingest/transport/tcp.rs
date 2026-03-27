use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

use crate::ingest::transport::{Transport, TransportContext};
use crate::ingest::AdapterManager;

/// TCP Transport: 监听原始设备连接并把原始包交给 `AdapterManager` 的 worker
pub struct TcpTransport {
    pub bind: String,
}

impl TcpTransport {
    pub fn new(bind: String) -> Self {
        Self { bind }
    }
}

#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn start(&self, ctx: TransportContext) -> Result<()> {
        let addr: SocketAddr = self.bind.parse()?;
        let listener = TcpListener::bind(addr).await?;
        log::info!("tcp transport listening on {}", self.bind);

        loop {
            let (stream, addr) = listener.accept().await?;
            let manager = ctx.manager.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, manager).await {
                    log::error!("tcp connection error {}: {}", addr, e);
                }
            });
        }
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

async fn handle_connection(mut stream: TcpStream, manager: Arc<AdapterManager>) -> Result<()> {
    let mut buffer = vec![0u8; 4096];
    let mut remaining_data = Vec::new();

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                remaining_data.extend_from_slice(&buffer[..n]);
                // extract packets similar to previous TcpServer.extract_packet
                while let Some(packet) = extract_packet(&mut remaining_data)? {
                    // parse serial number and device_type is still needed here
                    match dispatch_packet(&manager, packet).await {
                        Ok(_) => {}
                        Err(e) => log::error!("dispatch packet error: {}", e),
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!(e));
            }
        }
    }

    Ok(())
}

fn extract_packet(buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, anyhow::Error> {
    if buffer.len() < 4 {
        return Ok(None);
    }
    if buffer[0] != 0xab || buffer[1] != 0xcd {
        for i in 1..buffer.len() - 1 {
            if buffer[i] == 0xab && buffer[i + 1] == 0xcd {
                buffer.drain(..i);
                return extract_packet(buffer);
            }
        }
        buffer.clear();
        return Ok(None);
    }
    let data_len = buffer[2] as usize;
    let total_len = data_len + 4;
    if buffer.len() < total_len {
        return Ok(None);
    }
    let packet = buffer[..total_len].to_vec();
    buffer.drain(..total_len);
    Ok(Some(packet))
}

async fn dispatch_packet(
    manager: &Arc<AdapterManager>,
    packet: Vec<u8>,
) -> Result<(), anyhow::Error> {
    let serial_number =
        crate::ingest::adapters::mattress::MattressAdapter::extract_serial_number(&packet)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    manager
        .dispatch_by_serial(&serial_number, "smart_mattress", packet, "tcp")
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}
