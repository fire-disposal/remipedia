//! 智能床垫适配器模块
//!
//! 提供事件驱动原生架构的智能床垫数据处理功能

mod adapter;
mod event_engine;
mod types;

// 公开主要类型
pub use adapter::MattressAdapter;
pub use event_engine::MattressEventEngine;
pub use types::{
    AlertLevel, MattressData, MattressEvent, MattressState, SmartSamplingConfig, TurnOverEvent,
    TurnOverState, VitalSignsConfig,
};

// 为了向后兼容，提供类型别名
pub type SmartMattressAdapter = MattressAdapter;
pub type SmartMattressFilter = MattressEventEngine;
