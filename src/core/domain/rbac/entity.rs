//! RBAC 实体定义

use chrono::{DateTime, Utc};
use std::collections::HashSet;
use uuid::Uuid;

/// 权限标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Permission {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

impl Permission {
    pub fn new(code: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn code(&self) -> &str {
        &self.code
    }
}

/// 角色
#[derive(Debug, Clone)]
pub struct Role {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    permissions: HashSet<String>, // 权限代码集合
    is_system: bool,              // 是否系统内置角色
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Role {
    /// 创建新角色
    pub fn create(code: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            code: code.into(),
            name: name.into(),
            description: None,
            permissions: HashSet::new(),
            is_system: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// 系统角色重建
    pub fn reconstruct(
        id: Uuid,
        code: String,
        name: String,
        description: Option<String>,
        permissions: HashSet<String>,
        is_system: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            code,
            name,
            description,
            permissions,
            is_system,
            created_at,
            updated_at,
        }
    }

    /// 添加权限
    pub fn grant_permission(&mut self, permission_code: impl Into<String>) {
        self.permissions.insert(permission_code.into());
        self.updated_at = Utc::now();
    }

    /// 移除权限
    pub fn revoke_permission(&mut self, permission_code: &str) {
        self.permissions.remove(permission_code);
        self.updated_at = Utc::now();
    }

    /// 检查是否有权限
    pub fn has_permission(&self, permission_code: &str) -> bool {
        // 如果拥有 manage 权限，则自动拥有该域的所有权限
        if let Some((domain, _)) = permission_code.split_once(':') {
            let manage_perm = format!("{}:manage", domain);
            if self.permissions.contains(&manage_perm) {
                return true;
            }
        }
        self.permissions.contains(permission_code)
    }

    /// 检查是否有任一权限
    pub fn has_any_permission(&self, permission_codes: &[&str]) -> bool {
        permission_codes.iter().any(|&p| self.has_permission(p))
    }

    /// 检查是否有所有权限
    pub fn has_all_permissions(&self, permission_codes: &[&str]) -> bool {
        permission_codes.iter().all(|&p| self.has_permission(p))
    }

    /// 是否是系统角色（不可删除）
    pub fn is_system(&self) -> bool {
        self.is_system
    }

    // Getters
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn permissions(&self) -> &HashSet<String> {
        &self.permissions
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// 角色分配（用户-角色关联）
#[derive(Debug, Clone)]
pub struct RoleAssignment {
    id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
    granted_by: Option<Uuid>, // 谁授予的
    granted_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}

impl RoleAssignment {
    pub fn new(user_id: Uuid, role_id: Uuid) -> Self {
        Self {
            id: Uuid::now_v7(),
            user_id,
            role_id,
            granted_by: None,
            granted_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn with_granted_by(mut self, granted_by: Uuid) -> Self {
        self.granted_by = Some(granted_by);
        self
    }

    pub fn with_expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }

    // Getters
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn role_id(&self) -> Uuid {
        self.role_id
    }

    pub fn granted_by(&self) -> Option<Uuid> {
        self.granted_by
    }

    pub fn granted_at(&self) -> DateTime<Utc> {
        self.granted_at
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permission_check() {
        let mut role = Role::create("test", "Test Role");
        role.grant_permission("device:manage");

        // manage 权限应该包含该域的所有权限
        assert!(role.has_permission("device:read"));
        assert!(role.has_permission("device:create"));
        assert!(role.has_permission("device:manage"));
        
        // 其他域的权限不应该有
        assert!(!role.has_permission("user:read"));
    }

    #[test]
    fn test_role_any_and_all_permissions() {
        let mut role = Role::create("test", "Test Role");
        role.grant_permission("device:read");
        role.grant_permission("device:create");

        assert!(role.has_any_permission(&["device:read", "user:read"]));
        assert!(!role.has_all_permissions(&["device:read", "user:read"]));
        assert!(role.has_all_permissions(&["device:read", "device:create"]));
    }

    #[test]
    fn test_role_assignment_expiration() {
        let assignment = RoleAssignment::new(Uuid::now_v7(), Uuid::now_v7())
            .with_expires_at(Utc::now() - chrono::Duration::hours(1));

        assert!(assignment.is_expired());
    }
}
