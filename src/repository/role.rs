use crate::core::entity::{NewRole, Role, UpdateRole};
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
}
