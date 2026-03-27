//! 智能床垫设备模块
//! 
//! 聚合床垫设备相关的所有类型、状态和适配器

mod adapter;
mod decoder;
mod event_engine;
pub mod transport;
pub mod types;

pub use adapter::MattressAdapter;
pub use types::*;
pub use event_engine::MattressEventEngine;

/// 设备模块入口 - 支持自动注册
pub struct MattressModule;

impl crate::ingest::adapters::DeviceModule for MattressModule {
    fn metadata() -> crate::ingest::adapters::DeviceMetadata {
        crate::ingest::adapters::DeviceMetadata {
            device_type: "smart_mattress",
            display_name: "智能床垫",
            supported_data_types: &["smart_mattress", "mattress_event"],
            protocol_version: "1.0",
        }
    }

    fn create_adapter() -> Box<dyn crate::ingest::adapters::DeviceAdapter> {
        Box::new(MattressAdapter::new())
    }
}
