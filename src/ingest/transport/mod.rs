//! Transport 模块 - 简化版
//! 
//! 直接使用 DeviceManager，无需复杂抽象

pub mod mqtt;
pub mod tcp;

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use sqlx::PgPool;

use crate::ingest::DeviceManager;

/// Transport trait
#[async_trait]
pub trait Transport: Send + Sync {
    async fn start(&self, ctx: TransportContext) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}

/// Transport 上下文
#[derive(Clone)]
pub struct TransportContext {
    pub device_manager: Arc<DeviceManager>,
    pub pool: Arc<PgPool>,
}

impl TransportContext {
    pub fn new(device_manager: Arc<DeviceManager>, pool: Arc<PgPool>) -> Self {
        Self { device_manager, pool }
    }
}

/// Transport 管理器
pub struct TransportManager {
    transports: Vec<Arc<dyn Transport>>,
}

impl TransportManager {
    pub fn new() -> Self {
        Self { transports: Vec::new() }
    }

    pub fn register<T: Transport + 'static>(&mut self, t: Arc<T>) {
        self.transports.push(t);
    }

    pub async fn start_all(&self, ctx: TransportContext) -> Result<()> {
        for t in &self.transports {
            let t = t.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move {
                if let Err(e) = t.start(ctx).await {
                    log::error!("transport error: {:?}", e);
                }
            });
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        for t in &self.transports {
            t.stop().await?;
        }
        Ok(())
    }
}

impl Default for TransportManager {
    fn default() -> Self {
        Self::new()
    }
}
