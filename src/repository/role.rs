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
            r#"INSERT INTO roles (name, description, data_scope)
               VALUES ($1, $2, COALESCE($3, 'all'))
               RETURNING *"#,
        )
        .bind(&new_role.name)
        .bind(&new_role.description)
        .bind(&new_role.data_scope)
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
                   description = COALESCE($3, description),
                   data_scope = COALESCE($4, data_scope)
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(&update.name)
        .bind(&update.description)
        .bind(&update.data_scope)
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

    // ============================================
    // 角色-模块代码相关 (替代 ModulePermissionRepository)
    // ============================================

    /// 获取角色可访问的模块代码列表
    pub async fn get_role_module_codes(&self, role_id: &Uuid) -> AppResult<Vec<String>> {
        let modules: Vec<(String,)> = sqlx::query_as(
            r#"SELECT module_code FROM role_modules
               WHERE role_id = $1
               ORDER BY module_code"#,
        )
        .bind(role_id)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(modules.into_iter().map(|m| m.0).collect())
    }

    /// 检查角色是否为系统角色
    pub async fn is_system_role(&self, role_id: &Uuid) -> AppResult<bool> {
        let result: Option<(bool,)> = sqlx::query_as(
            "SELECT is_system FROM roles WHERE id = $1"
        )
        .bind(role_id)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.map(|r| r.0).unwrap_or(false))
    }

    /// 获取角色可访问的模块（含系统角色通配判断）及数据范围
    /// 返回 (is_system_role, modules, data_scope)
    pub async fn get_accessible_modules(&self, role_id: &Uuid) -> AppResult<(bool, Vec<String>, String)> {
        let is_system = self.is_system_role(role_id).await?;
        let data_scope = self.get_data_scope(role_id).await?;

        if is_system {
            return Ok((true, vec!["*".to_string()], data_scope));
        }

        let module_codes = self.get_role_module_codes(role_id).await?;
        Ok((false, module_codes, data_scope))
    }

    /// 获取角色的数据范围
    pub async fn get_data_scope(&self, role_id: &Uuid) -> AppResult<String> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"SELECT data_scope FROM roles WHERE id = $1"#
        )
        .bind(role_id)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.map(|r| r.0).unwrap_or_else(|| "all".to_string()))
    }

    /// 为角色分配模块代码
    pub async fn assign_module_code(&self, role_id: &Uuid, module_code: &str) -> AppResult<()> {
        sqlx::query(
            r#"INSERT INTO role_modules (role_id, module_code) 
               VALUES ($1, $2)
               ON CONFLICT (role_id, module_code) DO NOTHING"#,
        )
        .bind(role_id)
        .bind(module_code)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    /// 移除角色的模块代码
    pub async fn revoke_module_code(&self, role_id: &Uuid, module_code: &str) -> AppResult<()> {
        sqlx::query(
            r#"DELETE FROM role_modules 
               WHERE role_id = $1 AND module_code = $2"#,
        )
        .bind(role_id)
        .bind(module_code)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    /// 批量设置角色的模块代码（替换式）
    pub async fn set_role_module_codes(&self, role_id: &Uuid, module_codes: &[String]) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        // 清空现有
        sqlx::query("DELETE FROM role_modules WHERE role_id = $1")
            .bind(role_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;

        // 批量插入
        for code in module_codes {
            sqlx::query(
                r#"INSERT INTO role_modules (role_id, module_code)
                   VALUES ($1, $2)"#,
            )
            .bind(role_id)
            .bind(code)
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// 批量分配模块代码
    pub async fn batch_assign_module_codes(&self, role_id: &Uuid, module_codes: &[String]) -> AppResult<()> {
        for code in module_codes {
            self.assign_module_code(role_id, code).await?;
        }
        Ok(())
    }

    /// 批量移除模块代码
    pub async fn batch_revoke_module_codes(&self, role_id: &Uuid, module_codes: &[String]) -> AppResult<()> {
        for code in module_codes {
            self.revoke_module_code(role_id, code).await?;
        }
        Ok(())
    }
}
