//! 设备适配器接口
//!
//! 定义所有设备适配器必须实现的 trait

use crate::errors::AppResult;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// 设备元信息
#[derive(Debug, Clone)]
pub struct DeviceMetadata {
    /// 设备类型标识
    pub device_type: &'static str,
    /// 设备类型显示名称
    pub display_name: &'static str,
    /// 支持的数据类型
    pub supported_data_types: &'static [&'static str],
    /// 协议版本
    pub protocol_version: &'static str,
}

/// 统一扁平消息负载
#[derive(Debug, Clone)]
pub struct MessagePayload {
    /// 事件或测量发生时间
    pub time: DateTime<Utc>,
    /// 数据类型
    pub data_type: String,
    /// 更细分类型
    pub message_type: Option<String>,
    /// 重要等级
    pub severity: Option<String>,
    /// 具体负载
    pub payload: Value,
}

/// 适配器输出
#[derive(Debug)]
pub enum AdapterOutput {
    Messages(Vec<MessagePayload>),
}

impl AdapterOutput {
    pub fn into_messages(self) -> Vec<MessagePayload> {
        match self {
            AdapterOutput::Messages(msgs) => msgs,
        }
    }

    /// 转换为 JSON 数组
    pub fn to_json(&self) -> Value {
        match self {
            AdapterOutput::Messages(msgs) => {
                let arr: Vec<Value> = msgs
                    .iter()
                    .map(|m| {
                        let mut map = serde_json::Map::new();
                        map.insert("time".to_string(), Value::String(m.time.to_rfc3339()));
                        map.insert("data_type".to_string(), Value::String(m.data_type.clone()));
                        if let Some(mt) = &m.message_type {
                            map.insert("message_type".to_string(), Value::String(mt.clone()));
                        }
                        if let Some(sev) = &m.severity {
                            map.insert("severity".to_string(), Value::String(sev.clone()));
                        }
                        map.insert("payload".to_string(), m.payload.clone());
                        Value::Object(map)
                    })
                    .collect();
                Value::Array(arr)
            }
        }
    }
}

/// 设备适配器 trait - 所有设备类型必须实现
///
/// 设计原则：
/// - 每个设备模块实现此 trait
/// - 提供静态元信息
/// - 支持独立状态
pub trait DeviceAdapter: Send + Sync + 'static {
    /// 获取设备元信息
    fn metadata(&self) -> DeviceMetadata;

    /// 解析原始数据为领域输出
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput>;

    /// 验证解析后的输出
    fn validate(&self, output: &AdapterOutput) -> AppResult<()>;

    /// 克隆适配器
    fn clone_box(&self) -> Box<dyn DeviceAdapter>;

    /// 获取设备类型
    fn device_type(&self) -> &'static str {
        self.metadata().device_type
    }

    /// 获取数据类型
    fn data_type(&self) -> &'static str {
        self.metadata().device_type
    }
}

/// 设备模块入口 trait
///
/// 每个设备模块实现此 trait 以支持自动注册
pub trait DeviceModule: Send + Sync + 'static {
    /// 设备元信息
    fn metadata() -> DeviceMetadata;

    /// 创建适配器实例
    fn create_adapter() -> Box<dyn DeviceAdapter>;
}
