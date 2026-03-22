use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// 注册设备请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterDeviceRequest {
    /// 设备序列号
    #[validate(length(min = 1, message = "序列号不能为空"))]
    pub serial_number: String,
    /// 设备类型 (heart_rate_monitor, fall_detector, spo2_sensor)
    #[validate(length(min = 1, message = "设备类型不能为空"))]
    pub device_type: String,
    /// 固件版本
    pub firmware_version: Option<String>,
    /// 元数据
    pub metadata: Option<serde_json::Value>,
}

/// 更新设备请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateDeviceRequest {
    /// 固件版本
    pub firmware_version: Option<String>,
    /// 状态 (active, inactive, maintenance)
    pub status: Option<String>,
    /// 元数据
    pub metadata: Option<serde_json::Value>,
}

/// 设备查询参数
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceQuery {
    /// 设备类型筛选
    pub device_type: Option<String>,
    /// 状态筛选
    pub status: Option<String>,
    /// 序列号筛选
    pub serial_number: Option<String>,
    /// 页码
    pub page: Option<u32>,
    /// 每页数量
    pub page_size: Option<u32>,
}
