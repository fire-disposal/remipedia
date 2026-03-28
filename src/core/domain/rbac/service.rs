//! RBAC 领域服务

use std::collections::HashSet;
use std::sync::Arc;

use crate::core::domain::rbac::{Role, RoleAssignment, RoleRepository, SystemPermissions};
use crate::core::domain::shared::{DomainError, DomainResult, UserId};

/// RBAC 服务
pub struct RbacService<R: RoleRepository> {
    role_repo: Arc<R>,
}

impl<R: RoleRepository> RbacService<R> {
    pub fn new(role_repo: Arc<R>) -> Self {
        Self { role_repo }
    }

    /// 获取用户的所有权限
    pub async fn get_user_permissions(&self, user_id: &UserId) -> DomainResult<HashSet<String>> {
        let assignments = self.role_repo.find_assignments_by_user(&user_id.as_uuid()).await?;
        
        let mut all_permissions = HashSet::new();
        
        for assignment in assignments {
            if assignment.is_expired() {
                continue;
            }
            
            if let Some(role) = self.role_repo.find_role_by_id(&assignment.role_id()).await? {
                all_permissions.extend(role.permissions().clone());
            }
        }
        
        Ok(all_permissions)
    }

    /// 检查用户是否有指定权限
    pub async fn has_permission(
        &self,
        user_id: &UserId,
        permission: &str,
    ) -> DomainResult<bool> {
        let assignments = self.role_repo.find_assignments_by_user(&user_id.as_uuid()).await?;
        
        for assignment in assignments {
            if assignment.is_expired() {
                continue;
            }
            
            if let Some(role) = self.role_repo.find_role_by_id(&assignment.role_id()).await? {
                if role.has_permission(permission) {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }

    /// 检查用户是否有任一权限
    pub async fn has_any_permission(
        &self,
        user_id: &UserId,
        permissions: &[&str],
    ) -> DomainResult<bool> {
        for &permission in permissions {
            if self.has_permission(user_id, permission).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 为用户分配角色
    pub async fn assign_role(
        &self,
        user_id: &UserId,
        role_id: &uuid::Uuid,
        granted_by: Option<&UserId>,
    ) -> DomainResult<RoleAssignment> {
        // 验证角色存在
        if self.role_repo.find_role_by_id(role_id).await?.is_none() {
            return Err(DomainError::NotFound(format!("角色不存在: {}", role_id)));
        }

        let mut assignment = RoleAssignment::new(user_id.as_uuid(), *role_id);
        if let Some(granter) = granted_by {
            assignment = assignment.with_granted_by(granter.as_uuid());
        }

        self.role_repo.save_assignment(&assignment).await?;
        Ok(assignment)
    }

    /// 创建系统内置角色
    pub async fn create_builtin_roles(&self) -> DomainResult<()> {
        // Admin 角色
        let admin_role = Role::reconstruct(
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            "admin".to_string(),
            "管理员".to_string(),
            Some("系统管理员，拥有所有权限".to_string()),
            SystemPermissions::admin_permissions(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        
        // User 角色
        let mut user_role = Role::reconstruct(
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            "user".to_string(),
            "普通用户".to_string(),
            Some("普通用户权限".to_string()),
            SystemPermissions::user_permissions(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        
        self.role_repo.save_role(&admin_role).await?;
        self.role_repo.save_role(&user_role).await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试需要 mock repository，这里仅测试逻辑
    #[test]
    fn test_permission_matching() {
        let mut role = Role::create("test", "Test");
        role.grant_permission("device:manage");

        assert!(role.has_permission("device:read"));
        assert!(role.has_permission("device:create"));
        assert!(!role.has_permission("user:read"));
    }
}
