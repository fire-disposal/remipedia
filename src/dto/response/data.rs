use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::Pagination;
use crate::core::entity::{Binding, Datasheet};

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

/// 数据查询响应（直接复用 Datasheet 实体）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataQueryResponse {
    /// 数据列表
    pub data: Vec<Datasheet>,
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

/// 绑定列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BindingListResponse {
    /// 绑定列表
    pub data: Vec<Binding>,
    /// 分页信息
    pub pagination: Pagination,
}

/// Ingest 原始数据记录
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RawDataRecordResponse {
    /// 归档记录ID
    pub id: Uuid,
    /// 数据来源
    pub source: String,
    /// 设备序列号
    pub serial_number: Option<String>,
    /// 设备类型
    pub device_type: Option<String>,
    /// 状态
    pub status: String,
    /// 状态说明
    pub status_message: Option<String>,
    /// 原始载荷大小（字节）
    pub payload_size: usize,
    /// 原始文本预览（最多 500 字符）
    pub raw_payload_preview: Option<String>,
    /// 接收时间
    pub received_at: DateTime<Utc>,
    /// 处理完成时间
    pub processed_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// Ingest 原始数据详情（包含完整原始字节）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RawDataDetailResponse {
    /// 归档记录ID
    pub id: Uuid,
    /// 数据来源
    pub source: String,
    /// 设备序列号
    pub serial_number: Option<String>,
    /// 设备类型
    pub device_type: Option<String>,
    /// 远程地址
    pub remote_addr: Option<String>,
    /// 元数据
    pub metadata: serde_json::Value,
    /// 状态
    pub status: String,
    /// 状态说明
    pub status_message: Option<String>,
    /// 原始载荷大小（字节）
    pub payload_size: usize,
    /// 原始载荷（Base64编码）
    pub raw_payload_base64: String,
    /// 原始载荷（UTF-8文本，如果可解码）
    pub raw_payload_text: Option<String>,
    /// 原始载荷（十六进制表示，用于二进制诊断）
    pub raw_payload_hex: String,
    /// 接收时间
    pub received_at: DateTime<Utc>,
    /// 处理完成时间
    pub processed_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// Ingest 原始数据查询响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RawDataQueryResponse {
    /// 数据列表
    pub data: Vec<RawDataRecordResponse>,
    /// 分页信息
    pub pagination: Pagination,
}
