//! 跌倒检测器设备模块
//!
//! 聚合跌倒检测器相关的所有类型和适配器

mod adapter;
mod types;

pub use adapter::FallDetectorAdapter;
pub use types::*;

/// 设备模块入口 - 支持自动注册
pub struct FallDetectorModule;

impl crate::ingest::adapters::DeviceModule for FallDetectorModule {
    fn metadata() -> crate::ingest::adapters::DeviceMetadata {
        crate::ingest::adapters::DeviceMetadata {
            device_type: "fall_detector",
            display_name: "跌倒检测器",
            supported_data_types: &["fall_detector", "fall_event"],
            protocol_version: "1.0",
        }
    }

    fn create_adapter() -> Box<dyn crate::ingest::adapters::DeviceAdapter> {
        Box::new(FallDetectorAdapter::new())
    }
}
