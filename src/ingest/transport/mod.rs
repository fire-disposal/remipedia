use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub mod mqtt;
pub mod tcp;

/// 统一 Transport trait：负责接收外部连接并把原始帧交给适配器/管道
#[async_trait]
pub trait Transport: Send + Sync {
    async fn start(&self, ctx: TransportContext) -> Result<()>;
    async fn stop(&self) -> Result<()> { Ok(()) }
}

/// 传输运行时上下文，提供访问适配器注册表等资源
#[derive(Clone)]
pub struct TransportContext {
    pub adapters: Arc<crate::ingest::AdapterRegistry>,
    pub manager: Arc<crate::ingest::AdapterManager>,
}

/// 简单管理器：按次序启动所有 transport（以非阻塞方式 spawn）
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
}
