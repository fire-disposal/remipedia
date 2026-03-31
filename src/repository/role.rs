use crate::core::entity::{NewRole, Permission, Role, UpdateRole};
use crate::core::value_object::SystemRole;
use crate::errors::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

pub struct RoleRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> RoleRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 根据 ID 查找角色
    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Option<Role>> {
        let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(role)
    }

    /// 根据名称查找角色
    pub async fn find_by_name(&self, name: &str) -> AppResult<Option<Role>> {
        let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE name = $1")
            .bind(name)
            .fetch_optional(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(role)
    }

    /// 列出所有角色
    pub async fn list_all(&self) -> AppResult<Vec<Role>> {
        let roles = sqlx::query_as::<_, Role>("SELECT * FROM roles ORDER BY created_at")
            .fetch_all(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(roles)
    }

    /// 创建角色
    pub async fn create(&self, new_role: &NewRole) -> AppResult<Role> {
        let role = sqlx::query_as::<_, Role>(
            r#"INSERT INTO roles (name, description) 
               VALUES ($1, $2) 
               RETURNING *"#,
        )
        .bind(&new_role.name)
        .bind(&new_role.description)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(role)
    }

    /// 更新角色
    pub async fn update(&self, id: &Uuid, update: &UpdateRole) -> AppResult<Role> {
        let role = sqlx::query_as::<_, Role>(
            r#"UPDATE roles 
               SET name = COALESCE($2, name),
                   description = COALESCE($3, description)
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(&update.name)
        .bind(&update.description)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(role)
    }

    /// 删除角色
    pub async fn delete(&self, id: &Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM roles WHERE id = $1 AND is_system = false")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// 获取角色的所有权限
    pub async fn get_permissions(&self, role_id: &Uuid) -> AppResult<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>(
            r#"SELECT p.* FROM permissions p
               INNER JOIN role_permissions rp ON p.id = rp.permission_id
               WHERE rp.role_id = $1
               ORDER BY p.resource, p.action"#,
        )
        .bind(role_id)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(permissions)
    }

    /// 检查角色是否拥有指定权限
    pub async fn has_permission(
        &self,
        role_id: &Uuid,
        resource: &str,
        action: &str,
    ) -> AppResult<bool> {
        // 超级管理员拥有所有权限
        if SystemRole::is_super_admin(role_id) {
            return Ok(true);
        }

        let result: Option<(i64,)> = sqlx::query_as(
            r#"SELECT 1 FROM role_permissions rp
               INNER JOIN permissions p ON rp.permission_id = p.id
               WHERE rp.role_id = $1 AND p.resource = $2 AND p.action = $3
               LIMIT 1"#,
        )
        .bind(role_id)
        .bind(resource)
        .bind(action)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.is_some())
    }

    /// 为角色分配权限
    pub async fn assign_permission(&self, role_id: &Uuid, permission_id: &Uuid) -> AppResult<()> {
        sqlx::query(
            r#"INSERT INTO role_permissions (role_id, permission_id) 
               VALUES ($1, $2)
               ON CONFLICT (role_id, permission_id) DO NOTHING"#,
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// 移除角色的权限
    pub async fn revoke_permission(
        &self,
        role_id: &Uuid,
        permission_id: &Uuid,
    ) -> AppResult<()> {
        sqlx::query(
            r#"DELETE FROM role_permissions 
               WHERE role_id = $1 AND permission_id = $2"#,
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }
}
