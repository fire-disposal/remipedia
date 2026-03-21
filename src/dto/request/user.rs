use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建用户请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 50, message = "用户名长度1-50"))]
    pub username: String,
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub password: String,
    #[validate(length(min = 1, message = "角色不能为空"))]
    pub role: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// 更新用户请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub status: Option<String>,
}

/// 用户查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuery {
    pub role: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}