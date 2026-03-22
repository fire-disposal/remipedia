//! 心率监测器适配器模块

mod types;
mod adapter;

pub use types::{HeartRateData, HeartRateEvent};
pub use adapter::HeartRateAdapter;