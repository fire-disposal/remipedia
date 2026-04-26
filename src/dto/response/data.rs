use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use super::Pagination;
use crate::core::entity::{AlertEvent, Binding, Observation};

/// 数据上报响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataReportResponse {
    /// 是否成功
    pub success: bool,
    /// 数据时间
    pub time: DateTime<Utc>,
    /// 数据流ID
    pub stream_id: Uuid,
    /// 观测记录ID
    pub observation_id: Uuid,
}

/// 观测数据响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ObservationResponse {
    /// 观测记录ID
    pub id: Uuid,
    /// 数据流ID
    pub stream_id: Uuid,
    /// 患者ID
    pub patient_id: Uuid,
    /// 数值
    #[schema(value_type = String)]
    pub value_numeric: Option<rust_decimal::Decimal>,
    /// 文本值
    pub value_text: Option<String>,
    /// 元数据
    pub metadata: serde_json::Value,
    /// 记录时间
    pub recorded_at: DateTime<Utc>,
}

impl From<Observation> for ObservationResponse {
    fn from(o: Observation) -> Self {
        Self {
            id: o.id,
            stream_id: o.stream_id,
            patient_id: o.patient_id,
            value_numeric: o.value_numeric,
            value_text: o.value_text,
            metadata: o.metadata,
            recorded_at: o.recorded_at,
        }
    }
}

/// 告警事件响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertEventResponse {
    /// 事件ID
    pub id: Uuid,
    /// 数据流ID
    pub stream_id: Uuid,
    /// 患者ID
    pub patient_id: Uuid,
    /// 严重级别
    pub severity: String,
    /// 状态
    pub status: String,
    /// 数值
    #[schema(value_type = String)]
    pub value_numeric: Option<rust_decimal::Decimal>,
    /// 文本值
    pub value_text: Option<String>,
    /// 载荷
    pub payload: serde_json::Value,
    /// 确认时间
    pub acknowledged_at: Option<DateTime<Utc>>,
    /// 确认人
    pub acknowledged_by: Option<Uuid>,
    /// 解决时间
    pub resolved_at: Option<DateTime<Utc>>,
    /// 解决人
    pub resolved_by: Option<Uuid>,
    /// 记录时间
    pub recorded_at: DateTime<Utc>,
}

impl From<AlertEvent> for AlertEventResponse {
    fn from(e: AlertEvent) -> Self {
        Self {
            id: e.id,
            stream_id: e.stream_id,
            patient_id: e.patient_id,
            severity: e.severity,
            status: e.status,
            value_numeric: e.value_numeric,
            value_text: e.value_text,
            payload: e.payload,
            acknowledged_at: e.acknowledged_at,
            acknowledged_by: e.acknowledged_by,
            resolved_at: e.resolved_at,
            resolved_by: e.resolved_by,
            recorded_at: e.recorded_at,
        }
    }
}

/// 数据查询响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataQueryResponse {
    /// 观测数据列表
    pub data: Vec<ObservationResponse>,
    /// 分页信息
    pub pagination: Pagination,
}

/// 告警统计响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertStatsResponse {
    /// 活跃告警数量
    pub total_active: i64,
    /// 已确认告警数量
    pub total_acknowledged: i64,
    /// 已解决告警数量
    pub total_resolved: i64,
    /// 按严重级别统计
    pub by_severity: HashMap<String, i64>,
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
