use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 角色实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建角色的数据
#[derive(Debug, Clone)]
pub struct NewRole {
    pub name: String,
    pub description: Option<String>,
}

/// 更新角色的数据
#[derive(Debug, Clone, Default)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub description: Option<String>,
}
