use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{NewObservation, Observation};
use crate::errors::{AppError, AppResult};
use crate::repository::ObservationRepository;

/// PgObservationRepository — `observations` 表的 PostgreSQL 实现
pub struct PgObservationRepository {
    pool: PgPool,
}

impl PgObservationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ObservationRepository for PgObservationRepository {
    async fn insert(&self, obs: &NewObservation) -> AppResult<Observation> {
        sqlx::query_as::<_, Observation>(
            r#"
            INSERT INTO observations (stream_id, patient_id, value_numeric, value_text, metadata, recorded_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, stream_id, patient_id, value_numeric, value_text, metadata, recorded_at
            "#,
        )
        .bind(obs.stream_id)
        .bind(obs.patient_id)
        .bind(obs.value_numeric)
        .bind(&obs.value_text)
        .bind(&obs.metadata)
        .bind(obs.recorded_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))
    }

    async fn query(
        &self,
        patient_id: &Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Observation>> {
        sqlx::query_as::<_, Observation>(
            r#"
            SELECT o.id, o.stream_id, o.patient_id, o.value_numeric, o.value_text,
                   o.metadata, o.recorded_at
            FROM observations o
            WHERE o.patient_id = $1
            ORDER BY o.recorded_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(patient_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    async fn count(&self, patient_id: &Uuid) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM observations WHERE patient_id = $1",
        )
        .bind(patient_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    async fn find_latest_by_patient(&self, patient_id: &Uuid) -> AppResult<Option<Observation>> {
        sqlx::query_as::<_, Observation>(
            r#"
            SELECT o.id, o.stream_id, o.patient_id, o.value_numeric, o.value_text,
                   o.metadata, o.recorded_at
            FROM observations o
            WHERE o.patient_id = $1
            ORDER BY o.recorded_at DESC
            LIMIT 1
            "#,
        )
        .bind(patient_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }
}
