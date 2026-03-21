use serde::{Deserialize, Serialize};
use validator::Validate;

/// 登录请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "用户名不能为空"))]
    pub username: String,
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub password: String,
}

/// 修改密码请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub old_password: String,
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub new_password: String,
}

/// 刷新令牌请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// 登出请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}