use std::sync::{Arc, Mutex};

use crate::errors::AppResult;

/// 简化后的 EngineService：为特定 MattressEventEngine 提供线程安全的阻塞处理
pub struct EngineServiceInner {
    // 使用 Mutex 保护引擎实例，parse/处理调用将直接在当前线程进行（适配旧接口）
    inner: Arc<Mutex<Box<dyn FnMut(Box<dyn std::any::Any>) -> AppResult<Vec<Box<dyn std::any::Any>>> + Send>>>,
}

impl EngineServiceInner {
    pub fn new<F>(f: F) -> Self
    where
        F: FnMut(Box<dyn std::any::Any>) -> AppResult<Vec<Box<dyn std::any::Any>>> + Send + 'static,
    {
        Self { inner: Arc::new(Mutex::new(Box::new(f))) }
    }

    pub fn process_blocking(&self, input: Box<dyn std::any::Any>) -> AppResult<Vec<Box<dyn std::any::Any>>> {
        let mut guard = self.inner.lock().map_err(|_| crate::errors::AppError::InternalError)?;
        (guard)(input)
    }
}
