//! 血氧传感器设备模块
//!
//! 聚合血氧传感器相关的所有类型和适配器

mod adapter;
mod types;

pub use adapter::SpO2Adapter;
pub use types::*;

/// 设备模块入口 - 支持自动注册
pub struct SpO2Module;

impl crate::ingest::adapters::DeviceModule for SpO2Module {
    fn metadata() -> crate::ingest::adapters::DeviceMetadata {
        crate::ingest::adapters::DeviceMetadata {
            device_type: "spo2_sensor",
            display_name: "血氧传感器",
            supported_data_types: &["spo2"],
            protocol_version: "1.0",
        }
    }

    fn create_adapter() -> Box<dyn crate::ingest::adapters::DeviceAdapter> {
        Box::new(SpO2Adapter::new())
    }
}
