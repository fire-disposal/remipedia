use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 模块实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Module {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// 角色-模块关联
#[derive(Debug, Clone, FromRow)]
pub struct RoleModule {
    pub role_id: Uuid,
    pub module_id: Uuid,
    pub granted_at: DateTime<Utc>,
}
