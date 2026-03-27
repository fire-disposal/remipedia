//! 事件钩子系统
//! 
//! 用于在事件产生时触发回调，支持 WebSocket 推送等扩展

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Serialize;

/// 事件类型
#[derive(Debug, Clone, Serialize)]
pub enum IngestEventType {
    /// 设备数据
    Data {
        serial_number: String,
        device_type: String,
    },
    /// 设备事件 (告警等)
    Event {
        serial_number: String,
        device_type: String,
        event_type: String,
        severity: Option<String>,
    },
    /// 设备上线
    DeviceOnline {
        serial_number: String,
        device_type: String,
    },
    /// 设备离线
    DeviceOffline {
        serial_number: String,
    },
    /// 设备错误
    DeviceError {
        serial_number: String,
        error: String,
    },
}

/// 事件钩子 trait
pub trait EventHook: Send + Sync {
    /// 处理事件
    fn on_event(&self, event: IngestEvent);
}

/// 事件包装
#[derive(Debug, Clone)]
pub struct IngestEvent {
    pub event_type: IngestEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub payload: serde_json::Value,
}

impl IngestEvent {
    pub fn new(event_type: IngestEventType, payload: serde_json::Value) -> Self {
        Self {
            event_type,
            timestamp: chrono::Utc::now(),
            payload,
        }
    }
}

/// 事件钩子注册表
#[derive(Clone)]
pub struct EventHookRegistry {
    hooks: Arc<RwLock<Vec<Box<dyn EventHook>>>>,
}

impl EventHookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册钩子
    pub async fn register<H: EventHook + 'static>(&self, hook: H) {
        let mut hooks = self.hooks.write().await;
        hooks.push(Box::new(hook));
    }

    /// 触发事件
    pub async fn emit(&self, event: IngestEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            hook.on_event(event.clone());
        }
    }

    /// 触发事件 (同步版本，用于 async 上下文外)
    pub fn emit_blocking(&self, event: IngestEvent) {
        let hooks = self.hooks.blocking_read();
        for hook in hooks.iter() {
            hook.on_event(event.clone());
        }
    }
}

impl Default for EventHookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局事件钩子注册表
static EVENT_HOOKS: std::sync::OnceLock<EventHookRegistry> = std::sync::OnceLock::new();

/// 获取全局事件钩子注册表
pub fn global_hooks() -> &'static EventHookRegistry {
    EVENT_HOOKS.get_or_init(EventHookRegistry::new)
}
