use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// 创建用户请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateUserRequest {
    /// 用户名
    #[validate(length(min = 1, max = 50, message = "用户名长度1-50"))]
    pub username: String,
    /// 密码
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub password: String,
    /// 角色ID (UUID)
    #[validate(length(min = 1, message = "角色不能为空"))]
    pub role_id: String,
    /// 手机号
    pub phone: Option<String>,
    /// 邮箱
    pub email: Option<String>,
}

/// 更新用户请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    /// 手机号
    pub phone: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 头像URL
    pub avatar_url: Option<String>,
    /// 状态 (active, inactive, locked)
    pub status: Option<String>,
}

/// 用户查询参数
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserQuery {
    /// 角色ID筛选
    pub role_id: Option<String>,
    /// 状态筛选
    pub status: Option<String>,
    /// 页码
    pub page: Option<u32>,
    /// 每页数量
    pub page_size: Option<u32>,
}
