//! 设备适配器模块
//! 
//! 每个设备类型作为独立模块，支持自动注册
//! 
//! 添加新设备支持：
//! 1. 创建 `adapters/新设备名/` 目录
//! 2. 实现 `DeviceAdapter` trait
//! 3. 实现 `DeviceModule` trait (可选，用于自动注册)
//! 4. 在本文件的 `autoload_devices!()` 宏中添加模块

pub mod mattress;

// 移除旧的 adapter_trait 模块，使用新的 framework 模块
