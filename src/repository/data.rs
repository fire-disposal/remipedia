use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{DataPoint, DataQuery, Datasheet};
use crate::errors::{AppError, AppResult};

pub struct DataRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DataRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 插入数据点（统一接口）
    pub async fn insert_datapoint(&self, data: &DataPoint
    ) -> AppResult<Datasheet> {
        let result = sqlx::query_as::<_, Datasheet>(
            r#"INSERT INTO datasheet (
                time, device_id, patient_id, data_type, data_category,
                value_numeric, value_text, severity, status, payload, source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING 
                time, device_id, patient_id, data_type, data_category,
                value_numeric, value_text, severity, status, payload, source, ingested_at"#,
        )
        .bind(data.time)
        .bind(data.device_id)
        .bind(data.patient_id)
        .bind(&data.data_type)
        .bind(data.data_category.to_string())
        .bind(data.value_numeric)
        .bind(&data.value_text)
        .bind(data.severity.as_ref().map(|s| s.to_string()))
        .bind(data.status.as_ref().map(|s| s.to_string()))
        .bind(&data.payload)
        .bind(&data.source)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result)
    }

    /// 批量插入数据点（单事务批量插入，性能优化）
    pub async fn insert_datapoints(
        &self,
        data_points: &[DataPoint],
    ) -> AppResult<Vec<Datasheet>> {
        if data_points.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        let mut results = Vec::with_capacity(data_points.len());

        for dp in data_points {
            let result = sqlx::query_as::<_, Datasheet>(
                r#"INSERT INTO datasheet (
                    time, device_id, patient_id, data_type, data_category,
                    value_numeric, value_text, severity, status, payload, source
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                RETURNING 
                    time, device_id, patient_id, data_type, data_category,
                    value_numeric, value_text, severity, status, payload, source, ingested_at"#,
            )
            .bind(dp.time)
            .bind(dp.device_id)
            .bind(dp.patient_id)
            .bind(&dp.data_type)
            .bind(dp.data_category.to_string())
            .bind(dp.value_numeric)
            .bind(&dp.value_text)
            .bind(dp.severity.as_ref().map(|s| s.to_string()))
            .bind(dp.status.as_ref().map(|s| s.to_string()))
            .bind(&dp.payload)
            .bind(&dp.source)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;

            results.push(result);
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;

        Ok(results)
    }

    /// 统一查询接口（支持指标和事件）
    pub async fn query(&self, query: &DataQuery
    ) -> AppResult<Vec<Datasheet>> {
        let limit = query.page_size as i64;
        let offset = ((query.page.saturating_sub(1)) * query.page_size) as i64;

        let data = sqlx::query_as::<_, Datasheet>(
            r#"SELECT 
                time, device_id, patient_id, data_type, data_category,
                value_numeric, value_text, severity, status, payload, source, ingested_at
            FROM datasheet
            WHERE ($1::uuid IS NULL OR patient_id = $1)
              AND ($2::uuid IS NULL OR device_id = $2)
              AND ($3::text IS NULL OR data_type = $3)
              AND ($4::text IS NULL OR data_category = $4)
              AND ($5::text IS NULL OR severity = $5)
              AND ($6::text IS NULL OR status = $6)
              AND ($7::timestamptz IS NULL OR time >= $7)
              AND ($8::timestamptz IS NULL OR time <= $8)
            ORDER BY time DESC
            LIMIT $9 OFFSET $10"#,
        )
        .bind(query.patient_id)
        .bind(query.device_id)
        .bind(&query.data_type)
        .bind(query.data_category.as_ref().map(|c| c.to_string()))
        .bind(query.severity.as_ref().map(|s| s.to_string()))
        .bind(query.status.as_ref().map(|s| s.to_string()))
        .bind(query.start_time)
        .bind(query.end_time)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(data)
    }

    /// 统计数据数量
    pub async fn count(&self, query: &DataQuery
    ) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM datasheet
            WHERE ($1::uuid IS NULL OR patient_id = $1)
              AND ($2::uuid IS NULL OR device_id = $2)
              AND ($3::text IS NULL OR data_type = $3)
              AND ($4::text IS NULL OR data_category = $4)
              AND ($5::text IS NULL OR severity = $5)
              AND ($6::text IS NULL OR status = $6)
              AND ($7::timestamptz IS NULL OR time >= $7)
              AND ($8::timestamptz IS NULL OR time <= $8)"#,
        )
        .bind(query.patient_id)
        .bind(query.device_id)
        .bind(&query.data_type)
        .bind(query.data_category.as_ref().map(|c| c.to_string()))
        .bind(query.severity.as_ref().map(|s| s.to_string()))
        .bind(query.status.as_ref().map(|s| s.to_string()))
        .bind(query.start_time)
        .bind(query.end_time)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    /// 查询活跃告警（未处理的事件）
    pub async fn query_active_alerts(
        &self,
        patient_id: Option<&Uuid>,
        limit: i64,
    ) -> AppResult<Vec<Datasheet>> {
        let data = sqlx::query_as::<_, Datasheet>(
            r#"SELECT 
                time, device_id, patient_id, data_type, data_category,
                value_numeric, value_text, severity, status, payload, source, ingested_at
            FROM datasheet
            WHERE data_category = 'event'
              AND status = 'active'
              AND ($1::uuid IS NULL OR patient_id = $1)
            ORDER BY 
                CASE severity
                    WHEN 'alert' THEN 1
                    WHEN 'warning' THEN 2
                    WHEN 'info' THEN 3
                    ELSE 4
                END,
                time DESC
            LIMIT $2"#,
        )
        .bind(patient_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(data)
    }

    /// 获取患者最新数据
    pub async fn find_latest_by_patient(
        &self,
        patient_id: &Uuid,
        data_type: Option<&str>,
        limit: i64,
    ) -> AppResult<Vec<Datasheet>> {
        let data = sqlx::query_as::<_, Datasheet>(
            r#"SELECT 
                time, device_id, patient_id, data_type, data_category,
                value_numeric, value_text, severity, status, payload, source, ingested_at
            FROM datasheet
            WHERE patient_id = $1 
              AND ($2::text IS NULL OR data_type = $2)
            ORDER BY time DESC
            LIMIT $3"#,
        )
        .bind(patient_id)
        .bind(data_type)
        .bind(limit)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(data)
    }

    /// 确认事件（更新状态）
    pub async fn acknowledge_event(
        &self,
        patient_id: &Uuid,
        time: &chrono::DateTime<chrono::Utc>,
        device_id: Option<&Uuid>,
    ) -> AppResult<Datasheet> {
        let result = sqlx::query_as::<_, Datasheet>(
            r#"UPDATE datasheet 
            SET status = 'acknowledged'
            WHERE patient_id = $1 
              AND time = $2
              AND ($3::uuid IS NULL OR device_id = $3)
              AND data_category = 'event'
            RETURNING 
                time, device_id, patient_id, data_type, data_category,
                value_numeric, value_text, severity, status, payload, source, ingested_at"#,
        )
        .bind(patient_id)
        .bind(time)
        .bind(device_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result)
    }

    /// 解决事件
    pub async fn resolve_event(
        &self,
        patient_id: &Uuid,
        time: &chrono::DateTime<chrono::Utc>,
        device_id: Option<&Uuid>,
    ) -> AppResult<Datasheet> {
        let result = sqlx::query_as::<_, Datasheet>(
            r#"UPDATE datasheet 
            SET status = 'resolved'
            WHERE patient_id = $1 
              AND time = $2
              AND ($3::uuid IS NULL OR device_id = $3)
              AND data_category = 'event'
            RETURNING 
                time, device_id, patient_id, data_type, data_category,
                value_numeric, value_text, severity, status, payload, source, ingested_at"#,
        )
        .bind(patient_id)
        .bind(time)
        .bind(device_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result)
    }

    /// 获取统计信息（仪表盘用）
    pub async fn get_stats(
        &self,
        patient_id: Option<&Uuid>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<DataStats> {
        let result: DataStats = sqlx::query_as(
            r#"SELECT 
                COUNT(*) FILTER (WHERE data_category = 'metric') as metric_count,
                COUNT(*) FILTER (WHERE data_category = 'event') as event_count,
                COUNT(*) FILTER (WHERE data_category = 'event' AND status = 'active') as active_alert_count,
                COUNT(*) FILTER (WHERE data_category = 'event' AND severity = 'alert') as critical_count
            FROM datasheet
            WHERE ($1::uuid IS NULL OR patient_id = $1)
              AND ($2::timestamptz IS NULL OR time >= $2)"#,
        )
        .bind(patient_id)
        .bind(start_time)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result)
    }
}

/// 数据统计
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DataStats {
    pub metric_count: i64,
    pub event_count: i64,
    pub active_alert_count: i64,
    pub critical_count: i64,
}
