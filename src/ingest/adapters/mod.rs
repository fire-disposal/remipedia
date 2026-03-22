mod adapter_trait;
mod heart_rate;
mod fall_detector;
mod spo2;
mod smart_mattress;
mod smart_mattress_filter;

#[cfg(test)]
mod smart_mattress_test;

pub use adapter_trait::DeviceAdapter;
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

        // 注册所有适配器
        adapters.insert(DeviceType::HeartRateMonitor, Arc::new(heart_rate::HeartRateAdapter));
        adapters.insert(DeviceType::FallDetector, Arc::new(fall_detector::FallDetectorAdapter));
        adapters.insert(DeviceType::SpO2Sensor, Arc::new(spo2::SpO2Adapter));
        adapters.insert(DeviceType::SmartMattress, Arc::new(smart_mattress::SmartMattressAdapter::new()));

        Self { adapters }
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