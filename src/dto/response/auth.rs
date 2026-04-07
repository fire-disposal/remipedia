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
    /// 角色ID
    pub role_id: String,
    /// 角色名称
    pub role_name: String,
    /// 是否为系统角色（拥有通配权限）
    pub is_system_role: bool,
    /// 可访问模块列表（["*"] 表示通配）
    pub accessible_modules: Vec<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后登录时间
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 验证 Token 响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyTokenResponse {
    /// 是否有效
    pub valid: bool,
    /// 用户信息（有效时返回）
    pub user: Option<UserInfo>,
}

/// 注册响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterResponse {
    /// 是否成功
    pub success: bool,
    /// 用户信息
    pub user: UserInfo,
}

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    /// 会话ID
    pub id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 状态
    pub status: String,
}

/// 会话列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionListResponse {
    /// 会话列表
    pub sessions: Vec<SessionInfo>,
    /// 总数
    pub total: i64,
}

/// 令牌撤销响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeResponse {
    /// 是否成功
    pub success: bool,
    /// 消息
    pub message: String,
}
