use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// 角色-模块关联（DB: role_modules 表）
#[derive(Debug, Clone, FromRow)]
pub struct RoleModule {
    pub role_id: Uuid,
    pub module_code: String,
    pub granted_at: DateTime<Utc>,
}
