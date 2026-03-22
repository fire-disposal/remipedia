//! 心率监测器适配器模块

mod adapter;
mod types;

pub use adapter::HeartRateAdapter;
pub use types::{HeartRateData, HeartRateEvent};
