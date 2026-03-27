//! Remipedia 数据接入层
//! 
//! 简化设计：扁平化架构

pub mod adapters;
pub mod device_manager;

pub use adapters::*;
pub use device_manager::*;

// 导出 transport
pub mod transport;
pub use transport::*;
