//! 设备适配器模块
//! 
//! 每个设备类型作为独立模块，支持自动注册
//! 
//! 添加新设备支持：
//! 1. 创建 `adapters/新设备名/` 目录
//! 2. 实现 `DeviceAdapter` trait
//! 3. 实现 `DeviceModule` trait (可选，用于自动注册)
//! 4. 在本文件的 `autoload_devices!()` 宏中添加模块

mod adapter_trait;
pub mod fall_detector;
pub mod heart_rate;
pub mod mattress;

// 公开 trait 和类型
pub use adapter_trait::{
    AdapterOutput, DeviceAdapter, DeviceMetadata, DeviceModule, MessagePayload,
};

use std::collections::HashMap;
use log::info;

use crate::core::value_object::DeviceType;

/// 适配器注册表 (非 Clone)
pub struct AdapterRegistry {
    adapters: HashMap<DeviceType, Box<dyn DeviceAdapter>>,
    metadata: HashMap<DeviceType, DeviceMetadata>,
}

impl AdapterRegistry {
    /// 创建新的注册表 (自动注册所有设备)
    pub fn new() -> Self {
        let mut registry = Self {
            adapters: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        // 自动注册所有设备模块
        registry.register_module::<mattress::MattressModule>();
        registry.register_module::<fall_detector::FallDetectorModule>();
        registry.register_module::<heart_rate::HeartRateModule>();
        
        info!("适配器注册表初始化完成，已注册 {} 种设备", registry.adapters.len());
        
        registry
    }

    /// 注册设备模块 (使用 DeviceModule trait)
    fn register_module<M: DeviceModule + 'static>(&mut self) {
        let metadata = M::metadata();
        
        // 从 metadata 获取设备类型并解析
        if let Some(device_type) = DeviceType::from_str(metadata.device_type) {
            let adapter = M::create_adapter();
            self.adapters.insert(device_type, adapter);
            self.metadata.insert(device_type, metadata);
        }
    }

    /// 手动注册适配器 (不支持 DeviceModule 时使用)
    pub fn register(&mut self, device_type: DeviceType, adapter: Box<dyn DeviceAdapter>) {
        let metadata = DeviceMetadata {
            device_type: adapter.device_type(),
            display_name: "Unknown",
            supported_data_types: &[],
            protocol_version: "unknown",
        };
        self.adapters.insert(device_type, adapter);
        self.metadata.insert(device_type, metadata);
    }

    /// 获取适配器
    pub fn get(&self, device_type: &DeviceType) -> Option<&Box<dyn DeviceAdapter>> {
        self.adapters.get(device_type)
    }

    /// 获取设备元信息
    pub fn get_metadata(&self, device_type: &DeviceType) -> Option<&DeviceMetadata> {
        self.metadata.get(device_type)
    }

    /// 检查是否支持该设备类型
    pub fn is_supported(&self, device_type: &DeviceType) -> bool {
        self.adapters.contains_key(device_type)
    }

    /// 获取所有支持的设备类型
    pub fn supported_types(&self) -> Vec<DeviceType> {
        self.adapters.keys().cloned().collect()
    }

    /// 获取所有设备元信息
    pub fn all_metadata(&self) -> Vec<&DeviceMetadata> {
        self.metadata.values().collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
