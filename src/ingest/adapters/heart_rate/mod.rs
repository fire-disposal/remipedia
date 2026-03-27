//! 心率监测器设备模块
//!
//! 聚合心率监测器相关的所有类型和适配器

mod adapter;
mod types;

pub use adapter::HeartRateAdapter;
pub use types::*;

/// 设备模块入口 - 支持自动注册
pub struct HeartRateModule;

impl crate::ingest::adapters::DeviceModule for HeartRateModule {
    fn metadata() -> crate::ingest::adapters::DeviceMetadata {
        crate::ingest::adapters::DeviceMetadata {
            device_type: "heart_rate_monitor",
            display_name: "心率监测器",
            supported_data_types: &["heart_rate", "heart_rate_event"],
            protocol_version: "1.0",
        }
    }

    fn create_adapter() -> Box<dyn crate::ingest::adapters::DeviceAdapter> {
        Box::new(HeartRateAdapter::new())
    }
}
