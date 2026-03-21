use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Pagination;

/// 数据上报响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReportResponse {
    pub success: bool,
    pub time: DateTime<Utc>,
    pub device_id: Uuid,
}

/// 数据记录响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecordResponse {
    pub time: DateTime<Utc>,
    pub device_id: Uuid,
    pub subject_id: Option<Uuid>,
    pub data_type: String,
    pub payload: serde_json::Value,
    pub source: String,
    pub ingested_at: DateTime<Utc>,
}

/// 数据查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQueryResponse {
    pub data: Vec<DataRecordResponse>,
    pub pagination: Pagination,
}

/// 绑定响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingResponse {
    pub id: Uuid,
    pub device_id: Uuid,
    pub patient_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

/// 绑定列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingListResponse {
    pub data: Vec<BindingResponse>,
    pub pagination: Pagination,
}