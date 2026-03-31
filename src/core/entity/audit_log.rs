use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

/// 审计日志实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub details: Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// 创建审计日志的数据
#[derive(Debug, Clone)]
pub struct NewAuditLog {
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub details: Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
}

impl NewAuditLog {
    pub fn success(
        user_id: Option<Uuid>,
        action: impl Into<String>,
        resource: impl Into<String>,
        resource_id: Option<String>,
    ) -> Self {
        Self {
            user_id,
            action: action.into(),
            resource: resource.into(),
            resource_id,
            details: serde_json::json!({}),
            ip_address: None,
            user_agent: None,
            status: "success".to_string(),
            error_message: None,
            duration_ms: None,
        }
    }

    pub fn failure(
        user_id: Option<Uuid>,
        action: impl Into<String>,
        resource: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            user_id,
            action: action.into(),
            resource: resource.into(),
            resource_id: None,
            details: serde_json::json!({}),
            ip_address: None,
            user_agent: None,
            status: "failure".to_string(),
            error_message: Some(error.into()),
            duration_ms: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    pub fn with_duration(mut self, ms: i32) -> Self {
        self.duration_ms = Some(ms);
        self
    }
}

/// 审计日志查询参数
#[derive(Debug, Clone, Default)]
pub struct AuditLogQuery {
    pub user_id: Option<Uuid>,
    pub action: Option<String>,
    pub resource: Option<String>,
    pub status: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub page: u32,
    pub page_size: u32,
}
