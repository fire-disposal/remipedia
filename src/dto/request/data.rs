use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// 数据上报请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataReportRequest {
    /// 时间戳
    pub timestamp: Option<DateTime<Utc>>,
    /// 设备ID
    pub device_id: Uuid,
    /// 患者ID
    pub patient_id: Option<Uuid>,
    /// 数据类型
    pub data_type: String,
    /// 数据负载
    pub payload: serde_json::Value,
}

/// 创建绑定请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBindingRequest {
    /// 设备ID
    pub device_id: Uuid,
    /// 患者ID
    pub patient_id: Uuid,
    /// 备注
    pub notes: Option<String>,
}

/// 切换绑定请求（强制换绑）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwitchBindingRequest {
    /// 设备ID
    pub device_id: Uuid,
    /// 新的患者ID
    pub new_patient_id: Uuid,
    /// 备注
    pub notes: Option<String>,
}

/// 结束绑定请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EndBindingRequest {
    /// 备注
    pub notes: Option<String>,
}

/// 数据查询参数
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataQuery {
    /// 患者ID筛选
    pub patient_id: Option<Uuid>,
    /// 设备ID筛选
    pub device_id: Option<Uuid>,
    /// 数据类型筛选
    pub data_type: Option<String>,
    /// 数据分类筛选（metric/event）
    pub data_category: Option<String>,
    /// 严重级别筛选（info/warning/alert）
    pub severity: Option<String>,
    /// 状态筛选（active/acknowledged/resolved）
    pub status: Option<String>,
    /// 开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 页码
    pub page: u32,
    /// 每页数量
    pub page_size: u32,
}

impl Default for DataQuery {
    fn default() -> Self {
        Self {
            patient_id: None,
            device_id: None,
            data_type: None,
            data_category: None,
            severity: None,
            status: None,
            start_time: None,
            end_time: None,
            page: 1,
            page_size: 20,
        }
    }
}

/// 确认事件请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgeEventRequest {
    /// 备注
    pub note: Option<String>,
}

/// 解决事件请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolveEventRequest {
    /// 备注
    pub note: Option<String>,
}

/// 告警查询参数
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertQuery {
    /// 患者ID筛选
    pub patient_id: Option<Uuid>,
    /// 数据类型筛选
    pub data_type: Option<String>,
    /// 严重级别筛选
    pub severity: Option<String>,
    /// 状态筛选（默认查询 active）
    pub status: Option<String>,
    /// 开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 页码
    pub page: u32,
    /// 每页数量
    pub page_size: u32,
}

impl Default for AlertQuery {
    fn default() -> Self {
        Self {
            patient_id: None,
            data_type: None,
            severity: None,
            status: Some("active".to_string()),
            start_time: None,
            end_time: None,
            page: 1,
            page_size: 20,
        }
    }
}
