//! 智能床垫适配器模块
//! 
//! 提供事件驱动原生架构的智能床垫数据处理功能

mod types;
mod event_engine;
mod adapter;

// 公开主要类型
pub use types::{
    AlertLevel, MattressEvent, MattressState, MattressData, SmartSamplingConfig, 
    VitalSignsConfig, TurnOverState, TurnOverEvent
};
pub use event_engine::MattressEventEngine;
pub use adapter::MattressAdapter;

// 为了向后兼容，提供类型别名
pub type SmartMattressAdapter = MattressAdapter;
pub type SmartMattressFilter = MattressEventEngine;