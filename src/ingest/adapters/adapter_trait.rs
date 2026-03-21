use crate::errors::AppResult;

/// 设备适配器 trait - 所有设备类型必须实现
pub trait DeviceAdapter: Send + Sync {
    /// 解析原始数据为标准 JSON 格式
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value>;

    /// 验证数据有效性
    fn validate(&self, payload: &serde_json::Value) -> AppResult<()>;

    /// 获取数据类型标识
    fn data_type(&self) -> &'static str;

    /// 获取设备类型标识
    fn device_type(&self) -> &'static str;
}