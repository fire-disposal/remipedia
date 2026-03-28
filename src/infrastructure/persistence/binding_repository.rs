//! Binding仓储SQLx实现

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::core::domain::binding::{Binding, BindingRepository};
use crate::core::domain::shared::{BindingId, DeviceId, DomainError, DomainResult, PatientId};
use crate::core::entity::Binding as BindingRow;

/// SQLx实现的Binding仓储
pub struct SqlxBindingRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SqlxBindingRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    fn to_entity(row: BindingRow) -> DomainResult<Binding> {
        Ok(Binding::reconstruct(
            BindingId::from_uuid(row.id),
            DeviceId::from_uuid(row.device_id),
            PatientId::from_uuid(row.patient_id),
            row.started_at,
            row.ended_at,
            row.notes,
            row.created_at,
        ))
    }
}

#[async_trait]
impl<'a> BindingRepository for SqlxBindingRepository<'a> {
    async fn find_by_id(&self, id: &BindingId) -> DomainResult<Option<Binding>> {
        let row = sqlx::query_as::<_, BindingRow>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE id = $1"#,
        )
        .bind(id.as_uuid())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn find_active_by_device(&self, device_id: &DeviceId) -> DomainResult<Option<Binding>> {
        let row = sqlx::query_as::<_, BindingRow>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE device_id = $1 AND ended_at IS NULL"#,
        )
        .bind(device_id.as_uuid())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn find_active_by_patient(
        &self,
        patient_id: &PatientId,
    ) -> DomainResult<Option<Binding>> {
        let row = sqlx::query_as::<_, BindingRow>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE patient_id = $1 AND ended_at IS NULL"#,
        )
        .bind(patient_id.as_uuid())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn find_by_device(
        &self,
        device_id: &DeviceId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Binding>> {
        let rows = sqlx::query_as::<_, BindingRow>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE device_id = $1
               ORDER BY started_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(device_id.as_uuid())
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        rows.into_iter().map(Self::to_entity).collect()
    }

    async fn find_by_patient(
        &self,
        patient_id: &PatientId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Binding>> {
        let rows = sqlx::query_as::<_, BindingRow>(
            r#"SELECT id, device_id, patient_id, started_at, ended_at, notes, created_at
               FROM binding WHERE patient_id = $1
               ORDER BY started_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(patient_id.as_uuid())
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        rows.into_iter().map(Self::to_entity).collect()
    }

    async fn save(&self, binding: &Binding) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO binding (id, device_id, patient_id, started_at, ended_at, notes, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (id) DO UPDATE SET
                   ended_at = EXCLUDED.ended_at,
                   notes = EXCLUDED.notes"#,
        )
        .bind(binding.id().as_uuid())
        .bind(binding.device_id().as_uuid())
        .bind(binding.patient_id().as_uuid())
        .bind(binding.started_at())
        .bind(binding.ended_at())
        .bind(binding.notes())
        .bind(binding.created_at())
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn end_binding(
        &self,
        id: &BindingId,
        ended_at: DateTime<Utc>,
    ) -> DomainResult<()> {
        sqlx::query("UPDATE binding SET ended_at = $1 WHERE id = $2")
            .bind(ended_at)
            .bind(id.as_uuid())
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }
}
