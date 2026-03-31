use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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

/// 事件严重级别
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

/// 事件状态
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

/// 统一数据实体（指标 + 事件）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Datasheet {
    pub time: DateTime<Utc>,
    pub device_id: Option<Uuid>,  // 改为可选
    pub patient_id: Option<Uuid>, // 患者ID（从绑定自动填充）
    pub data_type: String,
    pub data_category: String,                        // metric/event
    pub value_numeric: Option<f64>, // 数值
    pub value_text: Option<String>,                   // 文本值
    pub severity: Option<String>,                     // info/warning/alert
    pub status: Option<String>,                       // active/acknowledged/resolved
    pub payload: serde_json::Value,                   // 原始数据
    pub source: String,
    pub ingested_at: DateTime<Utc>,
}

impl Datasheet {
    /// 检查是否为事件
    pub fn is_event(&self) -> bool {
        self.data_category == "event"
    }

    /// 检查是否为活跃告警
    pub fn is_active_alert(&self) -> bool {
        self.is_event() && self.status.as_deref() == Some("active")
    }

    /// 获取严重级别
    pub fn severity(&self) -> Option<Severity> {
        self.severity.as_ref()?.parse().ok()
    }

    /// 获取事件状态
    pub fn status(&self) -> Option<EventStatus> {
        self.status.as_ref()?.parse().ok()
    }
}

/// 数据点（统一入口）
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

/// 数据查询参数
#[derive(Debug, Clone, Default)]
pub struct DataQuery {
    pub patient_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub data_type: Option<String>,
    pub data_category: Option<DataCategory>,
    pub severity: Option<Severity>,
    pub status: Option<EventStatus>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub page: u32,
    pub page_size: u32,
}

/// 事件确认请求
#[derive(Debug, Clone)]
pub struct AcknowledgeEventRequest {
    pub user_id: Uuid,
    pub note: Option<String>,
}

/// 事件解决请求
#[derive(Debug, Clone)]
pub struct ResolveEventRequest {
    pub user_id: Uuid,
    pub note: Option<String>,
}
