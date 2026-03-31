use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::dto::convert::IntoResponse;
use crate::dto::response::UserResponse;
use crate::errors::AppResult;
use crate::repository::RoleRepository;

/// 用户实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    /// 角色 ID（外键关联 roles 表）
    pub role_id: Uuid,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新用户
#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub password_hash: String,
    pub role_id: Uuid,
    pub phone: Option<String>,
    pub email: Option<String>,
}

impl IntoResponse for User {
    type Response = UserResponse;

    async fn into_response(
        self,
        role_repo: &RoleRepository<'_>,
    ) -> AppResult<UserResponse> {
        let role_name = match role_repo.find_by_id(&self.role_id).await? {
            Some(role) => role.name,
            None => "unknown".to_string(),
        };

        Ok(UserResponse {
            id: self.id,
            username: self.username,
            role_id: self.role_id,
            role_name,
            phone: self.phone,
            email: self.email,
            avatar_url: self.avatar_url,
            status: self.status,
            created_at: self.created_at,
            last_login_at: self.last_login_at,
        })
    }
}
