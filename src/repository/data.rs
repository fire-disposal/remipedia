use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{Datasheet, IngestData};
use crate::dto::DataQuery;
use crate::errors::AppResult;

pub struct DataRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DataRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 插入数据
    pub async fn insert(&self, data: &IngestData) -> AppResult<Datasheet> {
        sqlx::query_as::<_, Datasheet>(
            r#"INSERT INTO datasheet (time, device_id, subject_id, data_type, payload, source)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING time, device_id, subject_id, data_type, payload, source, ingested_at"#,
        )
        .bind(data.time)
        .bind(data.device_id)
        .bind(data.subject_id)
        .bind(&data.data_type)
        .bind(&data.payload)
        .bind(&data.source)
        .fetch_one(self.pool)
        .await
        .map_err(crate::errors::AppError::DatabaseError)
    }

    /// 查询数据
    pub async fn query(&self, query: &DataQuery) -> AppResult<Vec<Datasheet>> {
        let limit = query.page_size as i64;
        let offset = ((query.page - 1) * query.page_size) as i64;

        let data = sqlx::query_as::<_, Datasheet>(
            r#"SELECT time, device_id, subject_id, data_type, payload, source, ingested_at
               FROM datasheet
               WHERE ($1::uuid IS NULL OR device_id = $1)
                 AND ($2::uuid IS NULL OR subject_id = $2)
                 AND ($3::text IS NULL OR data_type = $3)
                 AND ($4::timestamptz IS NULL OR time >= $4)
                 AND ($5::timestamptz IS NULL OR time <= $5)
               ORDER BY time DESC
               LIMIT $6 OFFSET $7"#,
        )
        .bind(query.device_id)
        .bind(query.subject_id)
        .bind(&query.data_type)
        .bind(query.start_time)
        .bind(query.end_time)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(crate::errors::AppError::DatabaseError)?;

        Ok(data)
    }

    /// 统计数据数量
    pub async fn count(&self, query: &DataQuery) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM datasheet
               WHERE ($1::uuid IS NULL OR device_id = $1)
                 AND ($2::uuid IS NULL OR subject_id = $2)
                 AND ($3::text IS NULL OR data_type = $3)
                 AND ($4::timestamptz IS NULL OR time >= $4)
                 AND ($5::timestamptz IS NULL OR time <= $5)"#,
        )
        .bind(query.device_id)
        .bind(query.subject_id)
        .bind(query.data_type.as_deref())
        .bind(query.start_time)
        .bind(query.end_time)
        .fetch_one(self.pool)
        .await
        .map_err(crate::errors::AppError::DatabaseError)?;

        Ok(result.0)
    }

    /// 获取设备最新数据
    pub async fn find_latest(
        &self,
        device_id: &Uuid,
        data_type: Option<&str>,
    ) -> AppResult<Option<Datasheet>> {
        let data = sqlx::query_as::<_, Datasheet>(
            r#"SELECT time, device_id, subject_id, data_type, payload, source, ingested_at
               FROM datasheet
               WHERE device_id = $1 AND ($2::text IS NULL OR data_type = $2)
               ORDER BY time DESC
               LIMIT 1"#,
        )
        .bind(device_id)
        .bind(data_type)
        .fetch_optional(self.pool)
        .await
        .map_err(crate::errors::AppError::DatabaseError)?;

        Ok(data)
    }

    /// 获取患者的最新数据
    pub async fn find_latest_by_subject(
        &self,
        subject_id: &Uuid,
        data_type: Option<&str>,
        limit: i64,
    ) -> AppResult<Vec<Datasheet>> {
        let data = sqlx::query_as::<_, Datasheet>(
            r#"SELECT time, device_id, subject_id, data_type, payload, source, ingested_at
               FROM datasheet
               WHERE subject_id = $1 AND ($2::text IS NULL OR data_type = $2)
               ORDER BY time DESC
               LIMIT $3"#,
        )
        .bind(subject_id)
        .bind(data_type)
        .bind(limit)
        .fetch_all(self.pool)
        .await
        .map_err(crate::errors::AppError::DatabaseError)?;

        Ok(data)
    }
}
