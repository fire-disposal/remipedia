use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::Pagination;

/// 设备响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceResponse {
    /// 设备ID
    pub id: Uuid,
    /// 序列号
    pub serial_number: String,
    /// 设备类型
    pub device_type: String,
    /// 固件版本
    pub firmware_version: Option<String>,
    /// 状态
    pub status: String,
    /// 元数据
    pub metadata: serde_json::Value,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 当前绑定信息
    pub current_binding: Option<BindingInfo>,
}

/// 绑定信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BindingInfo {
    /// 绑定ID
    pub binding_id: Uuid,
    /// 患者ID
    pub patient_id: Uuid,
    /// 患者姓名
    pub patient_name: Option<String>,
    /// 绑定开始时间
    pub started_at: DateTime<Utc>,
}

/// 设备列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceListResponse {
    /// 设备列表
    pub data: Vec<DeviceResponse>,
    /// 分页信息
    pub pagination: Pagination,
}
