//! Ingest 层数据载体类型
//!
//! 这些类型是 Ingest 模块内部使用的中间数据格式。
//! 从旧 `datasheet.rs` 迁移而来，剥离核心实体层后置于 Ingest 层。
//!
//! 用途：Ingest 模块的 process_* 函数生成 Vec<DataPoint>，
//! 通过 store_data_points() 桥接转换为新的 Observation/AlertEvent 格式写入新表。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 数据分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DataCategory {
    #[default]
    Metric, // 指标数据
    Event,  // 事件/告警
}

impl std::fmt::Display for DataCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Metric => write!(f, "metric"),
            Self::Event => write!(f, "event"),
        }
    }
}

impl std::str::FromStr for DataCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "metric" => Ok(Self::Metric),
            "event" => Ok(Self::Event),
            _ => Err(format!("未知数据分类: {}", s)),
        }
    }
}

/// 事件严重级别（Ingest 模块内部使用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,    // 信息
    Warning, // 警告
    Alert,   // 告警
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Alert => write!(f, "alert"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "alert" => Ok(Self::Alert),
            _ => Err(format!("未知严重级别: {}", s)),
        }
    }
}

/// 事件状态（Ingest 模块内部使用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Active,       // 活跃（未处理）
    Acknowledged, // 已确认
    Resolved,     // 已解决
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Acknowledged => write!(f, "acknowledged"),
            Self::Resolved => write!(f, "resolved"),
        }
    }
}

impl std::str::FromStr for EventStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "acknowledged" => Ok(Self::Acknowledged),
            "resolved" => Ok(Self::Resolved),
            _ => Err(format!("未知状态: {}", s)),
        }
    }
}

/// 数据点（Ingest 模块通用中间格式）
///
/// Ingest 模块内部生成此格式，通过 `store_data_points()` 桥接写入新表。
#[derive(Debug, Clone)]
pub struct DataPoint {
    pub time: DateTime<Utc>,
    pub device_id: Option<Uuid>,
    pub patient_id: Option<Uuid>,
    pub data_type: String,
    pub data_category: DataCategory,
    pub value_numeric: Option<f64>,
    pub value_text: Option<String>,
    pub severity: Option<Severity>,
    pub status: Option<EventStatus>,
    pub payload: serde_json::Value,
    pub source: String,
}

impl DataPoint {
    /// 创建指标数据点
    pub fn metric(
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
        data_type: impl Into<String>,
        value: impl Into<f64>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            time: Utc::now(),
            device_id,
            patient_id,
            data_type: data_type.into(),
            data_category: DataCategory::Metric,
            value_numeric: Some(value.into()),
            value_text: None,
            severity: None,
            status: None,
            payload,
            source: "mqtt".to_string(),
        }
    }

    /// 创建事件数据点
    pub fn event(
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
        event_type: impl Into<String>,
        severity: Severity,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            time: Utc::now(),
            device_id,
            patient_id,
            data_type: event_type.into(),
            data_category: DataCategory::Event,
            value_numeric: None,
            value_text: None,
            severity: Some(severity),
            status: Some(EventStatus::Active),
            payload,
            source: "mqtt".to_string(),
        }
    }

    /// 设置数值
    pub fn with_numeric(mut self, value: impl Into<f64>) -> Self {
        self.value_numeric = Some(value.into());
        self
    }

    /// 设置文本
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.value_text = Some(text.into());
        self
    }

    /// 设置状态
    pub fn with_status(mut self, status: EventStatus) -> Self {
        self.status = Some(status);
        self
    }
}
