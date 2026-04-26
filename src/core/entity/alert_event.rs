use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 告警严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 告警
    Alert,
    /// 严重
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Alert => write!(f, "alert"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for AlertSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "alert" => Ok(Self::Alert),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("未知严重级别: {}", s)),
        }
    }
}

/// 告警事件状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    /// 活跃（未处理）
    Active,
    /// 已确认
    Acknowledged,
    /// 已解决
    Resolved,
}

impl std::fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Acknowledged => write!(f, "acknowledged"),
            Self::Resolved => write!(f, "resolved"),
        }
    }
}

impl std::str::FromStr for AlertStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "acknowledged" => Ok(Self::Acknowledged),
            "resolved" => Ok(Self::Resolved),
            _ => Err(format!("未知告警状态: {}", s)),
        }
    }
}

/// 告警事件：含确认/解决工作流的状态跟踪
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertEvent {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub patient_id: Uuid,
    pub severity: String, // 数据库存储字符串
    pub status: String,   // 数据库存储字符串
    pub value_numeric: Option<rust_decimal::Decimal>,
    pub value_text: Option<String>,
    pub payload: serde_json::Value,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub recorded_at: DateTime<Utc>,
}

impl AlertEvent {
    /// 获取严重级别的枚举表示
    pub fn severity_enum(&self) -> Option<AlertSeverity> {
        self.severity.parse().ok()
    }

    /// 获取状态的枚举表示
    pub fn status_enum(&self) -> Option<AlertStatus> {
        self.status.parse().ok()
    }

    /// 是否为活跃告警
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// 是否已确认
    pub fn is_acknowledged(&self) -> bool {
        self.status == "acknowledged"
    }

    /// 是否已解决
    pub fn is_resolved(&self) -> bool {
        self.status == "resolved"
    }
}

/// 创建告警事件的请求结构
#[derive(Debug, Clone)]
pub struct NewAlertEvent {
    pub stream_id: Uuid,
    pub patient_id: Uuid,
    pub severity: AlertSeverity,
    pub status: AlertStatus,
    pub value_numeric: Option<rust_decimal::Decimal>,
    pub value_text: Option<String>,
    pub payload: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}
