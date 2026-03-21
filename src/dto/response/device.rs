use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 设备响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResponse {
    pub id: Uuid,
    pub serial_number: String,
    pub device_type: String,
    pub firmware_version: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub current_binding: Option<BindingInfo>,
}

/// 绑定信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingInfo {
    pub binding_id: Uuid,
    pub patient_id: Uuid,
    pub patient_name: Option<String>,
    pub started_at: DateTime<Utc>,
}

/// 设备列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListResponse {
    pub data: Vec<DeviceResponse>,
    pub pagination: super::Pagination,
}