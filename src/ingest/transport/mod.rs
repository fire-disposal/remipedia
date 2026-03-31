//! V2 Transport层 - 简化版
//!
//! 只负责接收数据并提交到Pipeline

pub mod tcp;
pub mod mqtt;
pub mod websocket;

pub use tcp::TcpTransportV2;
pub use mqtt::MqttTransportV2;
pub use websocket::WebSocketTransportV2;

use crate::errors::AppResult;
use crate::ingest::IngestionPipeline;
use async_trait::async_trait;
use std::sync::Arc;

/// Transport trait
#[async_trait]
pub trait Transport: Send + Sync {
    async fn start(&self,
        pipeline: Arc<IngestionPipeline>,
    ) -> AppResult<()>;

    async fn stop(&self) -> AppResult<()>;
}
