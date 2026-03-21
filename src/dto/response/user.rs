use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// 用户响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    /// 用户ID
    pub id: Uuid,
    /// 用户名
    pub username: String,
    /// 角色
    pub role: String,
    /// 手机号
    pub phone: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 头像URL
    pub avatar_url: Option<String>,
    /// 状态
    pub status: String,
    /// 最后登录时间
    pub last_login_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 用户列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    /// 用户列表
    pub data: Vec<UserResponse>,
    /// 分页信息
    pub pagination: Pagination,
}

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Pagination {
    /// 当前页码
    pub page: u32,
    /// 每页数量
    pub page_size: u32,
    /// 总记录数
    pub total: i64,
    /// 总页数
    pub total_pages: i64,
}