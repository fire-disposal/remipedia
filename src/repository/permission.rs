use crate::core::entity::Permission;
use crate::errors::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PermissionRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PermissionRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 根据 ID 查找权限
    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Option<Permission>> {
        let permission = sqlx::query_as::<_, Permission>("SELECT * FROM permissions WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(permission)
    }

    /// 根据资源名和操作查找权限
    pub async fn find_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> AppResult<Option<Permission>> {
        let permission =
            sqlx::query_as::<_, Permission>(
                "SELECT * FROM permissions WHERE resource = $1 AND action = $2",
            )
            .bind(resource)
            .bind(action)
            .fetch_optional(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(permission)
    }

    /// 列出所有权限
    pub async fn list_all(&self) -> AppResult<Vec<Permission>> {
        let permissions =
            sqlx::query_as::<_, Permission>("SELECT * FROM permissions ORDER BY resource, action")
                .fetch_all(self.pool)
                .await
                .map_err(AppError::DatabaseError)?;
        Ok(permissions)
    }

    /// 按资源分组列出权限
    pub async fn list_by_resource(&self, resource: &str) -> AppResult<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>(
            "SELECT * FROM permissions WHERE resource = $1 ORDER BY action",
        )
        .bind(resource)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(permissions)
    }
}
