//! RBAC 仓储接口

use async_trait::async_trait;
use uuid::Uuid;

use crate::core::domain::rbac::{Role, RoleAssignment};
use crate::core::domain::shared::DomainResult;

/// 角色仓储接口
#[async_trait]
pub trait RoleRepository: Send + Sync {
    /// 保存角色
    async fn save_role(&self, role: &Role) -> DomainResult<()>;

    /// 根据ID查询角色
    async fn find_role_by_id(&self, id: &Uuid) -> DomainResult<Option<Role>>;

    /// 根据代码查询角色
    async fn find_role_by_code(&self, code: &str) -> DomainResult<Option<Role>>;

    /// 查询所有角色
    async fn find_all_roles(&self) -> DomainResult<Vec<Role>>;

    /// 删除角色
    async fn delete_role(&self, id: &Uuid) -> DomainResult<()>;

    /// 保存角色分配
    async fn save_assignment(&self, assignment: &RoleAssignment) -> DomainResult<()>;

    /// 查询用户的所有角色分配
    async fn find_assignments_by_user(&self, user_id: &Uuid) -> DomainResult<Vec<RoleAssignment>>;

    /// 删除角色分配
    async fn delete_assignment(&self, assignment_id: &Uuid) -> DomainResult<()>;

    /// 删除用户的所有角色分配
    async fn delete_user_assignments(&self, user_id: &Uuid) -> DomainResult<u64>;
}
