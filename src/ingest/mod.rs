//! 设备接入层 V2 - 全新架构
//!
//! 设计目标：
//! - 简化层级：Transport → Pipeline → Storage
//! - 明确分工：收数据 → 解析转换 → 存储
//! - 两种模式：有状态（事件分析）/ 无状态（简单转发）
//! - 有界队列：长度50，满了丢弃

pub mod adapter;
pub mod pipeline;
pub mod protocol;
pub mod resolver;
pub mod state;
pub mod transport;

pub use adapter::*;
pub use pipeline::*;
pub use protocol::*;
pub use resolver::*;
pub use state::*;

// 重导出子模块
pub mod adapters;

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

/// 解析后的数据
#[derive(Debug, Clone)]
pub struct ParsedData {
    pub device_id: String,
    pub device_type: String,
    pub timestamp: DateTime<Utc>,
    pub payload: Value,
    pub metadata: HashMap<String, String>,
}

impl ParsedData {
    pub fn new(device_id: String, device_type: String, payload: Value) -> Self {
        Self {
            device_id,
            device_type,
            timestamp: Utc::now(),
            payload,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// 数据包元信息
#[derive(Debug, Clone)]
pub struct PacketMetadata {
    pub serial_number: Option<String>,
    pub device_type: Option<String>,
    pub remote_addr: Option<String>,
    pub source: String,
}

impl PacketMetadata {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            serial_number: None,
            device_type: None,
            remote_addr: None,
            source: source.into(),
        }
    }
}

/// 数据包
#[derive(Debug, Clone)]
pub struct DataPacket {
    pub raw: Vec<u8>,
    pub metadata: PacketMetadata,
}

impl DataPacket {
    pub fn new(raw: Vec<u8>, source: impl Into<String>) -> Self {
        Self {
            raw,
            metadata: PacketMetadata::new(source),
        }
    }

    pub fn with_serial(mut self, serial: impl Into<String>) -> Self {
        self.metadata.serial_number = Some(serial.into());
        self
    }

    pub fn with_device_type(mut self, device_type: impl Into<String>) -> Self {
        self.metadata.device_type = Some(device_type.into());
        self
    }
}
