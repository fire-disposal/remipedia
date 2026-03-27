//! 智能床垫适配器模块
//!
//! 提供事件驱动原生架构的智能床垫数据处理功能

mod adapter;
mod decoder;
mod event_engine;
pub mod transport;
mod types;

// 公开主要类型
pub use adapter::MattressAdapter;
pub use event_engine::MattressEventEngine;
pub use types::{
    AlertLevel, MattressData, MattressEvent, MattressState, SmartSamplingConfig, TurnOverEvent,
    TurnOverState, VitalSignsConfig,
};

// 类型别名
pub type SmartMattressAdapter = MattressAdapter;
// 事件引擎为模块内实现，但可以直接使用 `MattressEventEngine`（线程安全由调用方负责）
