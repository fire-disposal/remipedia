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
    pub device_id: Option<Uuid>,
    /// 患者ID
    pub patient_id: Option<Uuid>,
}

/// 数据记录响应（统一数据+事件）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataRecordResponse {
    /// 数据时间
    pub time: DateTime<Utc>,
    /// 设备ID
    pub device_id: Option<Uuid>,
    /// 患者ID
    pub patient_id: Option<Uuid>,
    /// 数据类型
    pub data_type: String,
    /// 数据分类（metric/event）
    pub data_category: String,
    /// 数值（指标数据）
    pub value_numeric: Option<f64>,
    /// 文本值
    pub value_text: Option<String>,
    /// 严重级别（事件）
    pub severity: Option<String>,
    /// 状态（事件）
    pub status: Option<String>,
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

/// 告警统计响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertStatsResponse {
    /// 指标数据数量
    pub metric_count: i64,
    /// 事件数量
    pub event_count: i64,
    /// 活跃告警数量
    pub active_alert_count: i64,
    /// 严重告警数量
    pub critical_count: i64,
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
