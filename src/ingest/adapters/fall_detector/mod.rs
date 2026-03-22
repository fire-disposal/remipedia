//! 跌倒检测器适配器模块

mod types;
mod adapter;

pub use types::{FallDetectorData, FallDetectorEvent, FallEventType};
pub use adapter::FallDetectorAdapter;