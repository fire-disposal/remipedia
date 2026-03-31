//! 智能床垫设备 - V2架构实现
//!
//! 将原有床垫适配器迁移到新架构

pub mod adapter;
pub mod decoder;
pub mod state;
pub mod types;

pub use adapter::MattressAdapterV2;
pub use state::MattressStateV2;
pub use types::*;
