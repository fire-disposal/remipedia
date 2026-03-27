//! Remipedia 数据接入层
//! 
//! 简化设计：扁平化架构

pub mod adapters;
pub mod framework;

// 只导出新的框架类型
pub use framework::*;

// 导出 transport
pub mod transport;
pub use transport::*;
