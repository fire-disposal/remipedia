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
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub password: String,
}

/// 修改密码请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    /// 旧密码
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub old_password: String,
    /// 新密码
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub new_password: String,
}

/// 刷新令牌请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// 刷新令牌
    pub refresh_token: String,
}

/// 登出请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    /// 刷新令牌
    pub refresh_token: String,
}

/// 验证 Token 请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyTokenRequest {
    /// 访问令牌
    pub access_token: String,
}

/// 用户注册请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    /// 用户名
    #[validate(length(min = 3, max = 50, message = "用户名长度需在3-50个字符之间"))]
    pub username: String,
    /// 密码
    #[validate(length(min = 6, max = 100, message = "密码长度需在6-100个字符之间"))]
    pub password: String,
    /// 邮箱
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    /// 手机号
    #[validate(length(min = 11, max = 20, message = "手机号格式不正确"))]
    pub phone: Option<String>,
}

/// 令牌撤销请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeTokenRequest {
    /// 刷新令牌（可选，不提供则撤销当前用户所有令牌）
    pub refresh_token: Option<String>,
}
