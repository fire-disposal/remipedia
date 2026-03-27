use std::sync::{Arc, Mutex};

use crate::errors::AppResult;
use super::event_engine::MattressEventEngine;
use super::types::{MattressData, MattressEvent};

/// 床垫事件引擎服务（简化）：内部持有 `MattressEventEngine` 并提供阻塞式调用以兼容旧代码
pub struct MattressEventEngineService {
    engine: Arc<Mutex<MattressEventEngine>>,
}

impl MattressEventEngineService {
    pub fn new() -> Self {
        Self { engine: Arc::new(Mutex::new(MattressEventEngine::new())) }
    }

    /// 阻塞式调用：直接在当前线程锁引擎并处理
    pub fn process_blocking(&self, data: MattressData) -> AppResult<Vec<MattressEvent>> {
        let mut eng = self.engine.lock().map_err(|_| crate::errors::AppError::InternalError)?;
        eng.process_data(&data)
    }

    pub fn shutdown(self) {
        // nothing to do for now
    }
}
