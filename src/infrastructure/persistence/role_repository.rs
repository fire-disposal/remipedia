//! RBAC 仓储 SQLx 实现

use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::core::domain::rbac::{Role, RoleAssignment, RoleRepository};
use crate::core::domain::shared::{DomainError, DomainResult};

/// SQLx 实现的角色仓储
pub struct SqlxRoleRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SqlxRoleRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl<'a> RoleRepository for SqlxRoleRepository<'a> {
    async fn save_role(&self, role: &Role) -> DomainResult<()> {
        let permissions: Vec<String> = role.permissions().iter().cloned().collect();

        sqlx::query(
            r#"INSERT INTO roles (id, code, name, description, permissions, is_system, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (id) DO UPDATE SET
                   code = EXCLUDED.code,
                   name = EXCLUDED.name,
                   description = EXCLUDED.description,
                   permissions = EXCLUDED.permissions,
                   updated_at = EXCLUDED.updated_at"#,
        )
        .bind(role.id())
        .bind(role.code())
        .bind(role.name())
        .bind(role.permissions().is_empty().then_some(None::<String>).unwrap_or_else(|| Some(permissions.join(","))))
        .bind(role.is_system())
        .bind(role.created_at())
        .bind(role.updated_at())
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("保存角色失败: {}", e)))?;

        Ok(())
    }

    async fn find_role_by_id(&self, id: &Uuid) -> DomainResult<Option<Role>> {
        let row = sqlx::query_as::<_, RoleRow>(
            r#"SELECT id, code, name, description, permissions, is_system, created_at, updated_at
               FROM roles WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("查询角色失败: {}", e)))?;

        Ok(row.map(|r| r.to_entity()))
    }

    async fn find_role_by_code(&self, code: &str) -> DomainResult<Option<Role>> {
        let row = sqlx::query_as::<_, RoleRow>(
            r#"SELECT id, code, name, description, permissions, is_system, created_at, updated_at
               FROM roles WHERE code = $1"#,
        )
        .bind(code)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("查询角色失败: {}", e)))?;

        Ok(row.map(|r| r.to_entity()))
    }

    async fn find_all_roles(&self) -> DomainResult<Vec<Role>> {
        let rows = sqlx::query_as::<_, RoleRow>(
            r#"SELECT id, code, name, description, permissions, is_system, created_at, updated_at
               FROM roles ORDER BY created_at DESC"#,
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("查询角色列表失败: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.to_entity()).collect())
    }

    async fn delete_role(&self, id: &Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM roles WHERE id = $1 AND is_system = false")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(format!("删除角色失败: {}", e)))?;

        Ok(())
    }

    async fn save_assignment(&self, assignment: &RoleAssignment) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO user_roles (id, user_id, role_id, granted_by, granted_at, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (user_id, role_id) DO UPDATE SET
                   granted_by = EXCLUDED.granted_by,
                   granted_at = EXCLUDED.granted_at,
                   expires_at = EXCLUDED.expires_at"#,
        )
        .bind(assignment.id())
        .bind(assignment.user_id())
        .bind(assignment.role_id())
        .bind(assignment.granted_by())
        .bind(assignment.granted_at())
        .bind(assignment.expires_at())
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("保存角色分配失败: {}", e)))?;

        Ok(())
    }

    async fn find_assignments_by_user(&self, user_id: &Uuid) -> DomainResult<Vec<RoleAssignment>> {
        let rows = sqlx::query_as::<_, AssignmentRow>(
            r#"SELECT id, user_id, role_id, granted_by, granted_at, expires_at
               FROM user_roles WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("查询角色分配失败: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.to_entity()).collect())
    }

    async fn delete_assignment(&self, assignment_id: &Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM user_roles WHERE id = $1")
            .bind(assignment_id)
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(format!("删除角色分配失败: {}", e)))?;

        Ok(())
    }

    async fn delete_user_assignments(&self, user_id: &Uuid) -> DomainResult<u64> {
        let result = sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(format!("删除用户角色分配失败: {}", e)))?;

        Ok(result.rows_affected())
    }
}

/// 角色数据库行
#[derive(sqlx::FromRow)]
struct RoleRow {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    permissions: Option<String>,
    is_system: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl RoleRow {
    fn to_entity(self) -> Role {
        let permissions: HashSet<String> = self
            .permissions
            .map(|p| p.split(',').map(|s| s.to_string()).collect())
            .unwrap_or_default();

        Role::reconstruct(
            self.id,
            self.code,
            self.name,
            self.description,
            permissions,
            self.is_system,
            self.created_at,
            self.updated_at,
        )
    }
}

/// 角色分配数据库行
#[derive(sqlx::FromRow)]
struct AssignmentRow {
    id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
    granted_by: Option<Uuid>,
    granted_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl AssignmentRow {
    fn to_entity(self) -> RoleAssignment {
        RoleAssignment::new(self.user_id, self.role_id)
            .with_granted_by(self.granted_by.unwrap_or_else(Uuid::nil))
    }
}
