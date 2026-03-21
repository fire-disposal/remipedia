use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{Binding, NewBinding};
use crate::errors::{AppError, AppResult};

pub struct BindingRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> BindingRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Binding> {
        sqlx::query_as::<_, Binding>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("绑定: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    /// 查找设备当前有效绑定
    pub async fn find_active_by_device(&self, device_id: &Uuid) -> AppResult<Option<Binding>> {
        sqlx::query_as::<_, Binding>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE device_id = $1 AND ended_at IS NULL"#,
        )
        .bind(device_id)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    /// 查找患者当前有效绑定
    pub async fn find_active_by_patient(&self, patient_id: &Uuid) -> AppResult<Option<Binding>> {
        sqlx::query_as::<_, Binding>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE patient_id = $1 AND ended_at IS NULL"#,
        )
        .bind(patient_id)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    /// 创建绑定
    pub async fn create(&self, binding: &NewBinding) -> AppResult<Binding> {
        sqlx::query_as::<_, Binding>(
            r#"INSERT INTO binding (device_id, patient_id, notes)
               VALUES ($1, $2, $3)
               RETURNING id, device_id, patient_id, started_at, ended_at, notes, created_at"#,
        )
        .bind(binding.device_id)
        .bind(binding.patient_id)
        .bind(&binding.notes)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    /// 结束绑定
    pub async fn end_binding(&self, binding_id: &Uuid, ended_at: DateTime<Utc>) -> AppResult<()> {
        let result = sqlx::query(
            r#"UPDATE binding SET ended_at = $2 WHERE id = $1 AND ended_at IS NULL"#,
        )
        .bind(binding_id)
        .bind(ended_at)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("有效绑定: {}", binding_id)));
        }

        Ok(())
    }

    /// 获取设备的所有绑定历史
    pub async fn find_all_by_device(&self, device_id: &Uuid, limit: i64, offset: i64) -> AppResult<Vec<Binding>> {
        let bindings = sqlx::query_as::<_, Binding>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE device_id = $1
               ORDER BY started_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(device_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(bindings)
    }

    /// 获取患者的所有绑定历史
    pub async fn find_all_by_patient(&self, patient_id: &Uuid, limit: i64, offset: i64) -> AppResult<Vec<Binding>> {
        let bindings = sqlx::query_as::<_, Binding>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE patient_id = $1
               ORDER BY started_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(patient_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(bindings)
    }

    /// 统计设备的绑定数量
    pub async fn count_by_device(&self, device_id: &Uuid) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM binding WHERE device_id = $1"
        )
        .bind(device_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    /// 统计患者的绑定数量
    pub async fn count_by_patient(&self, patient_id: &Uuid) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM binding WHERE patient_id = $1"
        )
        .bind(patient_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    /// 查询绑定列表（通用）
    pub async fn query(
        &self,
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
        active_only: bool,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Binding>> {
        let bindings = sqlx::query_as::<_, Binding>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding
               WHERE ($1::uuid IS NULL OR device_id = $1)
                 AND ($2::uuid IS NULL OR patient_id = $2)
                 AND ($3::boolean IS FALSE OR ended_at IS NULL)
               ORDER BY started_at DESC
               LIMIT $4 OFFSET $5"#,
        )
        .bind(device_id)
        .bind(patient_id)
        .bind(active_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(bindings)
    }

    /// 统计绑定数量（通用）
    pub async fn count_query(
        &self,
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
        active_only: bool,
    ) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM binding
               WHERE ($1::uuid IS NULL OR device_id = $1)
                 AND ($2::uuid IS NULL OR patient_id = $2)
                 AND ($3::boolean IS FALSE OR ended_at IS NULL)"#,
        )
        .bind(device_id)
        .bind(patient_id)
        .bind(active_only)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }
}