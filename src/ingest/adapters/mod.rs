mod adapter_trait;
pub mod fall_detector; // 跌倒检测器模块
pub mod mattress;
pub mod spo2; // 血氧传感器模块 // 床垫适配器模块


/// 适配器接口导出：所有适配器应实现 `DeviceAdapter`，并返回统一的 `AdapterOutput`。
/// 约束：
/// - `parse(raw)` 应为同步/快速路径或在调用端使用 `spawn_blocking`。
/// - `AdapterOutput::Messages` 的 `payload` 用于入库，字段命名应与 `DataService` 入库约定一致。
pub use adapter_trait::{DeviceAdapter, AdapterOutput, MessagePayload};
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::value_object::DeviceType;

/// 适配器注册表
#[derive(Clone)]
pub struct AdapterRegistry {
    adapters: HashMap<DeviceType, Arc<dyn DeviceAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        let mut adapters: HashMap<DeviceType, Arc<dyn DeviceAdapter>> = HashMap::new();

        // 注册所有适配器 - 使用新的模块化架构
        adapters.insert(
            DeviceType::FallDetector,
            Arc::new(fall_detector::FallDetectorAdapter::new()),
        );
        adapters.insert(DeviceType::SpO2Sensor, Arc::new(spo2::SpO2Adapter::new()));
        adapters.insert(
            DeviceType::SmartMattress,
            Arc::new(mattress::MattressAdapter::new()),
        );

        Self { adapters }
    }

    /// 返回当前注册表的条目拷贝，便于遍历（DeviceType, Adapter）
    pub fn iter(&self) -> Vec<(DeviceType, Arc<dyn DeviceAdapter>)> {
        self.adapters
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// 获取适配器
    pub fn get(&self, device_type: &DeviceType) -> Option<Arc<dyn DeviceAdapter>> {
        self.adapters.get(device_type).cloned()
    }

    /// 检查是否支持该设备类型
    pub fn is_supported(&self, device_type: &DeviceType) -> bool {
        self.adapters.contains_key(device_type)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
