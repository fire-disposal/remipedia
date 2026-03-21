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
    /// 患者/受试者ID
    pub subject_id: Option<Uuid>,
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

/// 数据查询参数
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataQuery {
    /// 设备ID筛选
    pub device_id: Option<Uuid>,
    /// 患者/受试者ID筛选
    pub subject_id: Option<Uuid>,
    /// 数据类型筛选
    pub data_type: Option<String>,
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
            device_id: None,
            subject_id: None,
            data_type: None,
            start_time: None,
            end_time: None,
            page: 1,
            page_size: 20,
        }
    }
}