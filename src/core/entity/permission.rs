use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 权限实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Permission {
    pub id: Uuid,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 权限标识（用于快速检查）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionKey {
    pub resource: String,
    pub action: String,
}

impl PermissionKey {
    pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

impl From<(String, String)> for PermissionKey {
    fn from((resource, action): (String, String)) -> Self {
        Self { resource, action }
    }
}
