use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// 登录响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 用户信息
    pub user: UserInfo,
}

/// 令牌刷新响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponse {
    /// 新的访问令牌
    pub access_token: String,
    /// 新的刷新令牌
    pub refresh_token: String,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    /// 用户ID
    pub id: String,
    /// 用户名
    pub username: String,
    /// 角色
    pub role: String,
    /// 邮箱
    pub email: Option<String>,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后登录时间
    pub last_login_at: Option<DateTime<Utc>>,
}

/// 令牌验证响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyTokenResponse {
    /// 是否有效
    pub valid: bool,
    /// 用户信息（如果有效）
    pub user: Option<UserInfo>,
}
