//! TCP Transport - 简化版

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

use crate::ingest::transport::{Transport, TransportContext};
use crate::core::value_object::DeviceType;

pub struct TcpTransport {
    pub bind: String,
    pub default_device_type: &'static str,
}

impl TcpTransport {
    pub fn new(bind: String) -> Self {
        Self { 
            bind, 
            default_device_type: "smart_mattress" 
        }
    }
}

#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn start(&self, ctx: TransportContext) -> Result<()> {
        let addr: SocketAddr = self.bind.parse()?;
        let listener = TcpListener::bind(addr).await?;
        log::info!("tcp transport listening on {}", self.bind);

        let default_type = self.default_device_type;
        let device_manager = ctx.device_manager.clone();

        loop {
            let (stream, addr) = listener.accept().await?;
            let device_manager = device_manager.clone();
            let default_type = default_type;

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, device_manager, default_type).await {
                    log::error!("tcp connection error {}: {}", addr, e);
                }
            });
        }
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    device_manager: Arc<crate::ingest::DeviceManager>,
    default_type: &str,
) -> Result<()> {
    let mut buffer = vec![0u8; 4096];
    let mut remaining = Vec::new();

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                remaining.extend_from_slice(&buffer[..n]);
                while let Some(packet) = extract_packet(&mut remaining)? {
                    if let Ok(serial) = extract_serial(&packet) {
                        let device_manager = device_manager.clone();
                        let device_type = default_type.to_string();
                        
                        tokio::spawn(async move {
                            if let Err(e) = process_packet(&device_manager, &serial, &device_type, packet).await {
                                log::error!("process error: {}", e);
                            }
                        });
                    }
                }
            }
            Err(e) => return Err(anyhow::anyhow!(e)),
        }
    }
    Ok(())
}

fn extract_serial(packet: &[u8]) -> Result<String, anyhow::Error> {
    crate::ingest::adapters::mattress::MattressAdapter::extract_serial_number(packet)
        .map_err(|e| anyhow::anyhow!(e))
}

fn extract_packet(buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, anyhow::Error> {
    if buffer.len() < 4 {
        return Ok(None);
    }
    if buffer[0] != 0xab || buffer[1] != 0xcd {
        for i in 1..buffer.len().saturating_sub(1) {
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

async fn process_packet(
    device_manager: &Arc<crate::ingest::DeviceManager>,
    serial: &str,
    device_type: &str,
    packet: Vec<u8>,
) -> Result<(), anyhow::Error> {
    let device_type = DeviceType::from_str(device_type)
        .ok_or_else(|| anyhow::anyhow!("未知设备类型"))?;
    
    device_manager
        .process(serial, device_type, packet, "tcp")
        .await?;

    Ok(())
}
