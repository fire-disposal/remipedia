//! 设备接入层 V4 - 解耦架构
//!
//! 设计目标：
//! - 完全解耦：每个设备类型独立模块
//! - 自包含：传输 + 协议 + 业务 在一个模块内
//! - 简化依赖：模块之间无直接依赖
//! - 易于扩展：新增设备只需添加新模块
//!
//! 模块列表：
//! - mattress: 智能床垫TCP模块 (Msgpack协议)
//! - vision: 视觉识别MQTT模块 (JSON)
//! - imu: IMU传感器MQTT模块 (JSON，跌倒检测)

pub mod modules;

pub use modules::*;
