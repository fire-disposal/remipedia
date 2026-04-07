use crate::core::entity::Module;
use crate::core::value_object::Module as ModuleEnum;
use crate::errors::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

pub struct ModulePermissionRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> ModulePermissionRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 获取角色的模块权限列表
    pub async fn get_role_modules(&self, role_id: &Uuid) -> AppResult<Vec<Module>> {
        let modules = sqlx::query_as::<_, Module>(
            r#"SELECT m.* FROM modules m
               INNER JOIN role_modules rm ON m.id = rm.module_id
               WHERE rm.role_id = $1
               ORDER BY m.category, m.code"#,
        )
        .bind(role_id)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(modules)
    }

    /// 检查是否为系统角色
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

    /// 获取角色可访问的模块代码列表
    pub async fn get_accessible_modules(&self, role_id: &Uuid) -> AppResult<(bool, Vec<String>)> {
        // 检查是否为系统角色
        let is_system = self.is_system_role(role_id).await?;
        
        if is_system {
            // 系统角色返回通配
            return Ok((true, vec!["*".to_string()]));
        }
        
        // 普通角色查询模块列表
        let modules = sqlx::query_as::<_, (String,)>(
            r#"SELECT m.code FROM modules m
               INNER JOIN role_modules rm ON m.id = rm.module_id
               WHERE rm.role_id = $1
               ORDER BY m.code"#,
        )
        .bind(role_id)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        let module_codes: Vec<String> = modules.into_iter().map(|m| m.0).collect();
        Ok((false, module_codes))
    }

    /// 为角色分配模块权限
    pub async fn assign_module(&self, role_id: &Uuid, module_id: &Uuid) -> AppResult<()> {
        sqlx::query(
            r#"INSERT INTO role_modules (role_id, module_id) 
               VALUES ($1, $2)
               ON CONFLICT (role_id, module_id) DO NOTHING"#,
        )
        .bind(role_id)
        .bind(module_id)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(())
    }

    /// 移除角色的模块权限
    pub async fn revoke_module(&self, role_id: &Uuid, module_id: &Uuid) -> AppResult<()> {
        sqlx::query(
            r#"DELETE FROM role_modules 
               WHERE role_id = $1 AND module_id = $2"#,
        )
        .bind(role_id)
        .bind(module_id)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(())
    }
}
