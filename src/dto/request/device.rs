use serde::{Deserialize, Serialize};
use validator::Validate;

/// 注册设备请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterDeviceRequest {
    #[validate(length(min = 1, message = "序列号不能为空"))]
    pub serial_number: String,
    #[validate(length(min = 1, message = "设备类型不能为空"))]
    pub device_type: String,
    pub firmware_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// 更新设备请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDeviceRequest {
    pub firmware_version: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// 设备查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceQuery {
    pub device_type: Option<String>,
    pub status: Option<String>,
    pub serial_number: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}