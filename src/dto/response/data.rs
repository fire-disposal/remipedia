use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::Pagination;

/// 数据上报响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataReportResponse {
    /// 是否成功
    pub success: bool,
    /// 数据时间
    pub time: DateTime<Utc>,
    /// 设备ID
    pub device_id: Uuid,
}

/// 数据记录响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataRecordResponse {
    /// 数据时间
    pub time: DateTime<Utc>,
    /// 设备ID
    pub device_id: Uuid,
    /// 患者/受试者ID
    pub subject_id: Option<Uuid>,
    /// 数据类型
    pub data_type: String,
    /// 数据负载
    pub payload: serde_json::Value,
    /// 数据来源
    pub source: String,
    /// 入库时间
    pub ingested_at: DateTime<Utc>,
}

/// 数据查询响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataQueryResponse {
    /// 数据列表
    pub data: Vec<DataRecordResponse>,
    /// 分页信息
    pub pagination: Pagination,
}

/// 绑定响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BindingResponse {
    /// 绑定ID
    pub id: Uuid,
    /// 设备ID
    pub device_id: Uuid,
    /// 患者ID
    pub patient_id: Uuid,
    /// 绑定开始时间
    pub started_at: DateTime<Utc>,
    /// 绑定结束时间
    pub ended_at: Option<DateTime<Utc>>,
    /// 备注
    pub notes: Option<String>,
}

/// 绑定列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BindingListResponse {
    /// 绑定列表
    pub data: Vec<BindingResponse>,
    /// 分页信息
    pub pagination: Pagination,
}