//! 状态管理模块
//!
//! 提供内存中的设备状态管理，支持自动清理

use crate::errors::AppResult;
use chrono::{DateTime, Utc};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;

/// 设备状态 trait
pub trait DeviceState: Send + Sync + Any {
    /// 更新状态（由适配器调用）
    fn update(&mut self, data: &serde_json::Value) -> AppResult<Vec<DeviceEvent>>;

    /// 获取状态快照
    fn snapshot(&self) -> serde_json::Value;

    /// 重置状态
    fn reset(&mut self);

    /// 获取最后访问时间
    fn last_accessed(&self) -> Instant;

    /// 更新访问时间
    fn touch(&mut self);

    /// 转换为 Any（用于downcast）
    fn as_any(&self) -> &dyn Any;

    /// 转换为可变 Any
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// 设备事件
#[derive(Debug, Clone)]
pub struct DeviceEvent {
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub severity: EventSeverity,
    pub payload: serde_json::Value,
}

/// 事件严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Info,
    Warning,
    Critical,
}

impl EventSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventSeverity::Info => "info",
            EventSeverity::Warning => "warning",
            EventSeverity::Critical => "critical",
        }
    }
}

/// 状态管理器
#[allow(dead_code)]
pub struct StateManager {
    states: Arc<RwLock<HashMap<String, Box<dyn DeviceState>>>>,
    max_states: usize,
    idle_timeout: Duration,
    cleanup_interval: Duration,
}

impl StateManager {
    /// 创建新的状态管理器
    ///
    /// # Arguments
    /// * `max_states` - 最大状态数（内存限制）
    /// * `idle_timeout` - 空闲超时时间
    pub fn new(max_states: usize, idle_timeout: Duration) -> Self {
        let states = Arc::new(RwLock::new(HashMap::new()));
        let states_clone = states.clone();
        let cleanup_interval = Duration::from_secs(60);

        // 启动清理任务
        tokio::spawn(async move {
            let mut ticker = interval(cleanup_interval);
            loop {
                ticker.tick().await;
                Self::cleanup_idle_states(&states_clone,
                idle_timeout).await;
            }
        });

        Self {
            states,
            max_states,
            idle_timeout,
            cleanup_interval,
        }
    }

    /// 加载设备状态（取出并移除）
    pub async fn load(&self,
        device_id: &str,
    ) -> Option<Box<dyn DeviceState>> {
        let mut states = self.states.write().await;
        states.remove(device_id)
    }

    /// 保存设备状态
    pub async fn save(
        &self,
        device_id: &str,
        mut state: Box<dyn DeviceState>,
    ) {
        state.touch();

        let mut states = self.states.write().await;

        // LRU清理：如果满了且是新设备，移除最旧的
        if states.len() >= self.max_states && !states.contains_key(device_id) {
            if let Some((oldest_key, _)) = states
                .iter()
                .min_by_key(|(_, s)| s.last_accessed())
            {
                let key = oldest_key.clone();
                log::info!("状态管理器LRU清理: device_id={}", key);
                states.remove(&key);
            }
        }

        states.insert(device_id.to_string(), state);
    }

    /// 移除设备状态
    pub async fn remove(&self,
        device_id: &str,
    ) -> Option<Box<dyn DeviceState>> {
        let mut states = self.states.write().await;
        states.remove(device_id)
    }

    /// 获取状态数量
    pub async fn count(&self) -> usize {
        let states = self.states.read().await;
        states.len()
    }

    /// 清理空闲状态
    async fn cleanup_idle_states(
        states: &Arc<RwLock<HashMap<String, Box<dyn DeviceState>>>>,
        idle_timeout: Duration,
    ) {
        let now = Instant::now();
        let mut states_guard = states.write().await;

        let before_count = states_guard.len();
        states_guard.retain(|device_id, state| {
            let elapsed = now.duration_since(state.last_accessed());
            let keep = elapsed < idle_timeout;
            if !keep {
                log::info!(
                    "清理空闲状态: device_id={}, 空闲时间={:?}",
                    device_id,
                    elapsed
                );
            }
            keep
        });

        let after_count = states_guard.len();
        if before_count != after_count {
            log::info!(
                "状态清理完成: {} -> {} 个状态",
                before_count,
                after_count
            );
        }
    }

    /// 获取所有设备ID列表
    pub async fn list_devices(&self) -> Vec<String> {
        let states = self.states.read().await;
        states.keys().cloned().collect()
    }

    /// 清空所有状态
    pub async fn clear(&self) {
        let mut states = self.states.write().await;
        states.clear();
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new(
            10000,                           // 默认最多10000个设备状态
            Duration::from_secs(30 * 60),   // 默认30分钟超时
        )
    }
}

/// 状态包装器 - 为具体状态类型提供通用实现
pub struct StateWrapper<S: DeviceStateImpl> {
    inner: S,
    last_accessed: Instant,
}

impl<S: DeviceStateImpl> StateWrapper<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            last_accessed: Instant::now(),
        }
    }
}

/// 设备状态实现 trait（供具体状态类型实现）
pub trait DeviceStateImpl: Send + Sync + Clone + 'static {
    /// 更新状态并返回事件
    fn update_impl(
        &mut self,
        data: &serde_json::Value,
    ) -> AppResult<Vec<DeviceEvent>>;

    /// 获取状态快照
    fn snapshot_impl(&self) -> serde_json::Value;

    /// 重置状态
    fn reset_impl(&mut self);
}

impl<S: DeviceStateImpl> DeviceState for StateWrapper<S> {
    fn update(
        &mut self,
        data: &serde_json::Value,
    ) -> AppResult<Vec<DeviceEvent>> {
        self.touch();
        self.inner.update_impl(data)
    }

    fn snapshot(&self) -> serde_json::Value {
        self.inner.snapshot_impl()
    }

    fn reset(&mut self) {
        self.touch();
        self.inner.reset_impl();
    }

    fn last_accessed(&self) -> Instant {
        self.last_accessed
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
