use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 登录响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    /// 是否成功
    pub success: bool,
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 用户信息
    pub user: UserInfo,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

/// 刷新令牌响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponse {
    /// 是否成功
    pub success: bool,
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
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
}