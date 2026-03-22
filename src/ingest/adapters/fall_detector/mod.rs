//! 跌倒检测器适配器模块

mod adapter;
mod types;

pub use adapter::FallDetectorAdapter;
pub use types::{FallDetectorData, FallDetectorEvent, FallEventType};
