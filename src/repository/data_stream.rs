use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{DataStream, DataStreamType};
use crate::errors::{AppError, AppResult};
use crate::repository::DataStreamRepository;

/// PgDataStreamRepository — `data_streams` 表的 PostgreSQL 实现
pub struct PgDataStreamRepository {
    pool: PgPool,
}

impl PgDataStreamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DataStreamRepository for PgDataStreamRepository {
    async fn find_by_patient(&self, patient_id: &Uuid) -> AppResult<Vec<DataStream>> {
        sqlx::query_as::<_, DataStream>(
            r#"
            SELECT id, name, stream_type, data_type, device_id, patient_id,
                   metadata, is_active, created_at, updated_at
            FROM data_streams
            WHERE patient_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(patient_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    async fn find_or_create(
        &self,
        device_id: &Uuid,
        data_type: &str,
        stream_type: &DataStreamType,
        patient_id: Option<Uuid>,
    ) -> AppResult<DataStream> {
        // 先尝试查找已有流
        let existing = sqlx::query_as::<_, DataStream>(
            r#"
            SELECT id, name, stream_type, data_type, device_id, patient_id,
                   metadata, is_active, created_at, updated_at
            FROM data_streams
            WHERE device_id = $1 AND data_type = $2 AND stream_type = $3
            LIMIT 1
            "#,
        )
        .bind(device_id)
        .bind(data_type)
        .bind(stream_type.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        if let Some(stream) = existing {
            return Ok(stream);
        }

        // 不存在则创建
        let name = format!("{}_{}", data_type, stream_type);
        let stream = DataStream::new(name, *stream_type, data_type.to_string(), Some(*device_id), patient_id);

        sqlx::query_as::<_, DataStream>(
            r#"
            INSERT INTO data_streams (id, name, stream_type, data_type, device_id, patient_id,
                                       metadata, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, name, stream_type, data_type, device_id, patient_id,
                      metadata, is_active, created_at, updated_at
            "#,
        )
        .bind(stream.id)
        .bind(&stream.name)
        .bind(&stream.stream_type)
        .bind(&stream.data_type)
        .bind(stream.device_id)
        .bind(stream.patient_id)
        .bind(&stream.metadata)
        .bind(stream.is_active)
        .bind(stream.created_at)
        .bind(stream.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))
    }

    async fn create(&self, stream: &DataStream) -> AppResult<DataStream> {
        sqlx::query_as::<_, DataStream>(
            r#"
            INSERT INTO data_streams (id, name, stream_type, data_type, device_id, patient_id,
                                       metadata, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, name, stream_type, data_type, device_id, patient_id,
                      metadata, is_active, created_at, updated_at
            "#,
        )
        .bind(stream.id)
        .bind(&stream.name)
        .bind(&stream.stream_type)
        .bind(&stream.data_type)
        .bind(stream.device_id)
        .bind(stream.patient_id)
        .bind(&stream.metadata)
        .bind(stream.is_active)
        .bind(stream.created_at)
        .bind(stream.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))
    }

    async fn find_by_id(&self, id: &Uuid) -> AppResult<Option<DataStream>> {
        sqlx::query_as::<_, DataStream>(
            r#"
            SELECT id, name, stream_type, data_type, device_id, patient_id,
                   metadata, is_active, created_at, updated_at
            FROM data_streams
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    async fn delete(&self, id: &Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM data_streams WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }
}
