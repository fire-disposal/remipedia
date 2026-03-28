use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// 登录请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    /// 用户名
    #[validate(length(min = 1, message = "用户名不能为空"))]
    pub username: String,
    /// 密码
    #[validate(length(min = 1, message = "密码不能为空"))]
    pub password: String,
}

/// 刷新令牌请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// 刷新令牌
    pub refresh_token: String,
}

/// 修改密码请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    /// 旧密码
    pub old_password: String,
    /// 新密码
    #[validate(length(min = 6, message = "密码至少6个字符"))]
    pub new_password: String,
}

/// 登出请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    /// 刷新令牌（用于撤销）
    pub refresh_token: Option<String>,
}

/// 验证令牌请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyTokenRequest {
    /// 访问令牌
    pub access_token: String,
}
