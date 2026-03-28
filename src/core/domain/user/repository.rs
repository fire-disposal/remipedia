//! User 仓储接口

use async_trait::async_trait;

use crate::core::domain::shared::{DomainResult, UserId};
use crate::core::domain::user::entity::User;

/// User 仓储接口
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 根据ID查找
    async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>>;

    /// 根据用户名查找
    async fn find_by_username(&self, username: &str) -> DomainResult<Option<User>>;

    /// 检查用户名是否存在
    async fn exists_by_username(&self, username: &str) -> DomainResult<bool>;

    /// 保存用户
    async fn save(&self, user: &User) -> DomainResult<()>;

    /// 删除用户
    async fn delete(&self, id: &UserId) -> DomainResult<()>;

    /// 查询用户列表
    async fn find_all(&self, role: Option<&str>, status: Option<&str>, limit: i64, offset: i64) -> DomainResult<Vec<User>>;

    /// 检查是否存在管理员
    async fn exists_admin(&self) -> DomainResult<bool>;
}
