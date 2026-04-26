use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{AlertEvent, NewAlertEvent};
use crate::errors::{AppError, AppResult};
use crate::repository::{AlertEventRepository, AlertStats};

/// PgAlertEventRepository — `alert_events` 表的 PostgreSQL 实现
pub struct PgAlertEventRepository {
    pool: PgPool,
}

impl PgAlertEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AlertEventRepository for PgAlertEventRepository {
    async fn insert(&self, alert: &NewAlertEvent) -> AppResult<AlertEvent> {
        sqlx::query_as::<_, AlertEvent>(
            r#"
            INSERT INTO alert_events (stream_id, patient_id, severity, status,
                                       value_numeric, value_text, payload, recorded_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, stream_id, patient_id, severity, status,
                      value_numeric, value_text, payload,
                      acknowledged_at, acknowledged_by, resolved_at, resolved_by, recorded_at
            "#,
        )
        .bind(alert.stream_id)
        .bind(alert.patient_id)
        .bind(alert.severity.to_string())
        .bind(alert.status.to_string())
        .bind(alert.value_numeric)
        .bind(&alert.value_text)
        .bind(&alert.payload)
        .bind(alert.recorded_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))
    }

    async fn query_active(&self, patient_id: &Uuid) -> AppResult<Vec<AlertEvent>> {
        sqlx::query_as::<_, AlertEvent>(
            r#"
            SELECT id, stream_id, patient_id, severity, status,
                   value_numeric, value_text, payload,
                   acknowledged_at, acknowledged_by, resolved_at, resolved_by, recorded_at
            FROM alert_events
            WHERE patient_id = $1 AND status = 'active'
            ORDER BY recorded_at DESC
            "#,
        )
        .bind(patient_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    async fn acknowledge(&self, id: &Uuid, by: &Uuid) -> AppResult<AlertEvent> {
        let now = chrono::Utc::now();
        sqlx::query_as::<_, AlertEvent>(
            r#"
            UPDATE alert_events
            SET status = 'acknowledged', acknowledged_at = $2, acknowledged_by = $3
            WHERE id = $1
            RETURNING id, stream_id, patient_id, severity, status,
                      value_numeric, value_text, payload,
                      acknowledged_at, acknowledged_by, resolved_at, resolved_by, recorded_at
            "#,
        )
        .bind(id)
        .bind(now)
        .bind(by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::not_found(format!("告警事件: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    async fn resolve(&self, id: &Uuid, by: &Uuid) -> AppResult<AlertEvent> {
        let now = chrono::Utc::now();
        sqlx::query_as::<_, AlertEvent>(
            r#"
            UPDATE alert_events
            SET status = 'resolved', resolved_at = $2, resolved_by = $3
            WHERE id = $1
            RETURNING id, stream_id, patient_id, severity, status,
                      value_numeric, value_text, payload,
                      acknowledged_at, acknowledged_by, resolved_at, resolved_by, recorded_at
            "#,
        )
        .bind(id)
        .bind(now)
        .bind(by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::not_found(format!("告警事件: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    async fn get_stats(&self, patient_id: &Uuid) -> AppResult<AlertStats> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT status, COUNT(*) as count
            FROM alert_events
            WHERE patient_id = $1
            GROUP BY status
            "#,
        )
        .bind(patient_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let mut stats = AlertStats::default();
        for (status, count) in rows {
            match status.as_str() {
                "active" => stats.total_active = count,
                "acknowledged" => stats.total_acknowledged = count,
                "resolved" => stats.total_resolved = count,
                _ => {}
            }
        }

        // 按严重级别统计
        let severity_rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT severity, COUNT(*) as count
            FROM alert_events
            WHERE patient_id = $1
            GROUP BY severity
            "#,
        )
        .bind(patient_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        for (severity, count) in severity_rows {
            stats.by_severity.insert(severity, count);
        }

        Ok(stats)
    }
}
