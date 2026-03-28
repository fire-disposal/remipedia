//! 健康数据仓储 SQLx 实现

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use std::str::FromStr;

use crate::core::domain::healthdata::{
    DataQuality, DataType, HealthData, HealthDataQuery, HealthDataRepository, HourlyAggregation,
};
use crate::core::value_object::DataSource;
use crate::core::domain::shared::{DeviceId, DomainError, DomainResult, PatientId};
use crate::core::entity::Datasheet as DatasheetRow;

/// SQLx 实现的健康数据仓储
pub struct SqlxHealthDataRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SqlxHealthDataRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    fn to_entity(&self, row: DatasheetRow) -> DomainResult<HealthData> {
        let data_type = DataType::from_str(&row.data_type);
        let source = DataSource::from_str(&row.source).unwrap_or_default();

        Ok(HealthData::reconstruct(
            Uuid::nil(), // datasheet 表使用复合主键，这里简化
            row.time,
            DeviceId::from_uuid(row.device_id),
            row.subject_id,
            data_type,
            row.payload,
            source,
            DataQuality::Good, // 默认质量
            row.ingested_at,
        ))
    }
}

#[async_trait]
impl<'a> HealthDataRepository for SqlxHealthDataRepository<'a> {
    async fn save(&self, data: &HealthData) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO datasheet (time, device_id, subject_id, data_type, payload, source, ingested_at)
               VALUES ($1, $2, $3, $4, $5, $6, NOW())"#,
        )
        .bind(data.time())
        .bind(data.device_id().as_uuid())
        .bind(data.subject_id())
        .bind(data.data_type().as_str())
        .bind(data.payload())
        .bind(data.source().to_string())
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("保存健康数据失败: {}", e)))?;

        Ok(())
    }

    async fn save_batch(&self, data_list: &[HealthData]) -> DomainResult<()> {
        // 使用事务批量插入
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Validation(format!("开启事务失败: {}", e)))?;

        for data in data_list {
            sqlx::query(
                r#"INSERT INTO datasheet (time, device_id, subject_id, data_type, payload, source, ingested_at)
                   VALUES ($1, $2, $3, $4, $5, $6, NOW())"#,
            )
            .bind(data.time())
            .bind(data.device_id().as_uuid())
            .bind(data.subject_id())
            .bind(data.data_type().as_str())
            .bind(data.payload())
            .bind(data.source().to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Validation(format!("批量保存失败: {}", e)))?;
        }

        tx.commit().await
            .map_err(|e| DomainError::Validation(format!("提交事务失败: {}", e)))?;

        Ok(())
    }

    async fn find_by_id(&self, _id: &Uuid) -> DomainResult<Option<HealthData>> {
        // datasheet 表没有单一ID主键，此方法不适用
        Err(DomainError::Validation("健康数据不支持按ID查询".into()))
    }

    async fn query(&self, query: &HealthDataQuery) -> DomainResult<Vec<HealthData>> {
        let rows = sqlx::query_as::<_, DatasheetRow>(
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
        .bind(query.data_type.as_ref().map(|t| t.as_str()))
        .bind(query.start_time)
        .bind(query.end_time)
        .bind(query.limit)
        .bind(query.offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("查询失败: {}", e)))?;

        rows.into_iter()
            .map(|row| self.to_entity(row))
            .collect()
    }

    async fn count(&self, query: &HealthDataQuery) -> DomainResult<i64> {
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
        .bind(query.data_type.as_ref().map(|t| t.as_str()))
        .bind(query.start_time)
        .bind(query.end_time)
        .fetch_one(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("计数失败: {}", e)))?;

        Ok(result.0)
    }

    async fn find_latest_by_device(
        &self,
        device_id: &DeviceId,
        data_type: Option<&DataType>,
    ) -> DomainResult<Option<HealthData>> {
        let row = sqlx::query_as::<_, DatasheetRow>(
            r#"SELECT time, device_id, subject_id, data_type, payload, source, ingested_at
               FROM datasheet
               WHERE device_id = $1 AND ($2::text IS NULL OR data_type = $2)
               ORDER BY time DESC
               LIMIT 1"#,
        )
        .bind(device_id.as_uuid())
        .bind(data_type.map(|t| t.as_str()))
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("查询最新数据失败: {}", e)))?;

        Ok(row.map(|r| self.to_entity(r)).transpose()?)
    }

    async fn find_latest_by_subject(
        &self,
        subject_id: &PatientId,
        data_type: Option<&DataType>,
    ) -> DomainResult<Option<HealthData>> {
        let row = sqlx::query_as::<_, DatasheetRow>(
            r#"SELECT time, device_id, subject_id, data_type, payload, source, ingested_at
               FROM datasheet
               WHERE subject_id = $1 AND ($2::text IS NULL OR data_type = $2)
               ORDER BY time DESC
               LIMIT 1"#,
        )
        .bind(subject_id.as_uuid())
        .bind(data_type.map(|t| t.as_str()))
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("查询最新数据失败: {}", e)))?;

        Ok(row.map(|r| self.to_entity(r)).transpose()?)
    }

    async fn aggregate_by_hour(
        &self,
        device_id: &DeviceId,
        data_type: &DataType,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> DomainResult<Vec<HourlyAggregation>> {
        // 使用 TimescaleDB 的时间桶函数（如果可用）
        let rows = sqlx::query_as::<_, (DateTime<Utc>, i64, Option<f64>, Option<f64>, Option<f64>)>(
            r#"SELECT 
                time_bucket('1 hour', time) as hour,
                COUNT(*) as count,
                AVG((payload->>'value')::float8) as avg_value,
                MIN((payload->>'value')::float8) as min_value,
                MAX((payload->>'value')::float8) as max_value
               FROM datasheet
               WHERE device_id = $1 
                 AND data_type = $2
                 AND time >= $3 AND time <= $4
               GROUP BY hour
               ORDER BY hour"#,
        )
        .bind(device_id.as_uuid())
        .bind(data_type.as_str())
        .bind(start)
        .bind(end)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(format!("聚合查询失败: {}", e)))?;

        Ok(rows.into_iter()
            .map(|(hour, count, avg_value, min_value, max_value)| HourlyAggregation {
                hour,
                count,
                avg_value,
                min_value,
                max_value,
            })
            .collect())
    }

    async fn delete_before(&self, before: DateTime<Utc>) -> DomainResult<u64> {
        let result = sqlx::query("DELETE FROM datasheet WHERE time < $1")
            .bind(before)
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(format!("删除数据失败: {}", e)))?;

        Ok(result.rows_affected())
    }
}
