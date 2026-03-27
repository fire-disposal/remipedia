use crate::errors::AppResult;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

/// 设备适配器 trait - 所有设备类型必须实现
///
/// 简化输出模型：所有适配器统一返回一组扁平的 `MessagePayload`，
/// 事件与时序都视为同一种“消息”，通过 `message_type` / `severity` 字段区分。
pub trait DeviceAdapter: Send + Sync {
    /// 解析原始数据为领域输出（扁平消息列表）
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput>;

    /// 验证解析后的领域输出
    fn validate(&self, output: &AdapterOutput) -> AppResult<()>;

    /// 获取设备类型标识
    fn device_type(&self) -> &'static str;
    /// 获取数据类型标识（与旧接口兼容）
    fn data_type(&self) -> &'static str;
}

/// 统一扁平消息负载
#[derive(Debug)]
pub struct MessagePayload {
    /// 事件或测量发生时间（对应 `datasheet.time`）
    pub time: DateTime<Utc>,
    /// 数据类型（对应 `datasheet.data_type`）
    pub data_type: String,
    /// 可选的更细分类型，例如事件名（如 "fall"、"alarm"），或测量子类型
    pub message_type: Option<String>,
    /// 可选的重要/紧急等级（由适配器决定是否填充）
    pub severity: Option<String>,
    /// 具体内部载荷，自由结构，写入 `datasheet.payload`
    pub payload: Value,
}

/// 适配器输出：一组扁平消息
#[derive(Debug)]
pub enum AdapterOutput {
    Messages(Vec<MessagePayload>),
}

impl AdapterOutput {
    /// 将 AdapterOutput 转换为 JSON 数组，数组中每项为扁平对象，包含 time/data_type/severity/message_type/payload
    /// 该方法仅用于在 DB 边界以 JSON 形式传输或调试，入库时应使用结构化字段插入（time 与 data_type 对应表列）
    pub fn to_json(&self) -> Value {
        match self {
            AdapterOutput::Messages(msgs) => {
                let arr: Vec<Value> = msgs
                    .iter()
                    .map(|m| {
                        let mut map = Map::new();
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
