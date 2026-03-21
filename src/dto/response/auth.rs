use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 登录响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserInfo,
    pub expires_at: DateTime<Utc>,
}

/// 刷新令牌响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenResponse {
    pub success: bool,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
}