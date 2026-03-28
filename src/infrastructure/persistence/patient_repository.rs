//! Patient 仓储 SQLx 实现

use async_trait::async_trait;
use sqlx::PgPool;

use crate::core::domain::patient::{Patient, PatientProfile, PatientRepository};
use crate::core::domain::shared::{DomainError, DomainResult, PatientId};
use crate::core::entity::{Patient as PatientRow, PatientProfile as PatientProfileRow};
use crate::core::value_object::{BloodType, Gender};

/// SQLx 实现的 Patient 仓储
pub struct SqlxPatientRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SqlxPatientRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    fn to_entity(row: PatientRow) -> DomainResult<Patient> {
        Ok(Patient::reconstruct(
            PatientId::from_uuid(row.id),
            row.name,
            row.external_id,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[async_trait]
impl<'a> PatientRepository for SqlxPatientRepository<'a> {
    async fn find_by_id(&self, id: &PatientId) -> DomainResult<Option<Patient>> {
        let row = sqlx::query_as::<_, PatientRow>(
            r#"SELECT id, name, external_id, created_at, updated_at FROM patient WHERE id = $1"#
        )
        .bind(id.as_uuid())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn find_by_external_id(&self, external_id: &str) -> DomainResult<Option<Patient>> {
        let row = sqlx::query_as::<_, PatientRow>(
            r#"SELECT id, name, external_id, created_at, updated_at FROM patient WHERE external_id = $1"#
        )
        .bind(external_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn exists_by_external_id(&self, external_id: &str) -> DomainResult<bool> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM patient WHERE external_id = $1 LIMIT 1"
        )
        .bind(external_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(result.is_some())
    }

    async fn save(&self, patient: &Patient) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO patient (id, name, external_id, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (id) DO UPDATE SET
                   name = EXCLUDED.name,
                   external_id = EXCLUDED.external_id,
                   updated_at = EXCLUDED.updated_at"#
        )
        .bind(patient.id().as_uuid())
        .bind(patient.name())
        .bind(patient.external_id())
        .bind(patient.created_at())
        .bind(patient.updated_at())
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &PatientId) -> DomainResult<()> {
        sqlx::query("DELETE FROM patient WHERE id = $1")
            .bind(id.as_uuid())
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn find_all(&self, name: Option<&str>, limit: i64, offset: i64) -> DomainResult<Vec<Patient>> {
        let rows = sqlx::query_as::<_, PatientRow>(
            r#"SELECT id, name, external_id, created_at, updated_at FROM patient
               WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#
        )
        .bind(name)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        rows.into_iter().map(Self::to_entity).collect()
    }

    async fn find_profile(&self, patient_id: &PatientId) -> DomainResult<Option<PatientProfile>> {
        let row = sqlx::query_as::<_, PatientProfileRow>(
            r#"SELECT patient_id, date_of_birth, gender, blood_type, contact_phone, address,
                      emergency_contact, emergency_phone, medical_id, allergies, medical_history,
                      notes, tags, metadata, created_at, updated_at
               FROM patient_profile WHERE patient_id = $1"#
        )
        .bind(patient_id.as_uuid())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(|r| PatientProfile {
            date_of_birth: r.date_of_birth,
            gender: r.gender,
            blood_type: r.blood_type,
            contact_phone: r.contact_phone,
            address: r.address,
            emergency_contact: r.emergency_contact,
            emergency_phone: r.emergency_phone,
            medical_id: r.medical_id,
            allergies: r.allergies,
            medical_history: r.medical_history,
            notes: r.notes,
            tags: r.tags,
            metadata: r.metadata,
        }))
    }

    async fn save_profile(&self, patient_id: &PatientId, profile: &PatientProfile) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO patient_profile (
                patient_id, date_of_birth, gender, blood_type, contact_phone, address,
                emergency_contact, emergency_phone, medical_id, allergies, medical_history,
                notes, tags, metadata, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())
            ON CONFLICT (patient_id) DO UPDATE SET
                date_of_birth = EXCLUDED.date_of_birth,
                gender = EXCLUDED.gender,
                blood_type = EXCLUDED.blood_type,
                contact_phone = EXCLUDED.contact_phone,
                address = EXCLUDED.address,
                emergency_contact = EXCLUDED.emergency_contact,
                emergency_phone = EXCLUDED.emergency_phone,
                medical_id = EXCLUDED.medical_id,
                allergies = EXCLUDED.allergies,
                medical_history = EXCLUDED.medical_history,
                notes = EXCLUDED.notes,
                tags = EXCLUDED.tags,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()"#
        )
        .bind(patient_id.as_uuid())
        .bind(profile.date_of_birth)
        .bind(profile.gender.map(|g| g.to_string()))
        .bind(profile.blood_type.map(|b| b.to_string()))
        .bind(&profile.contact_phone)
        .bind(&profile.address)
        .bind(&profile.emergency_contact)
        .bind(&profile.emergency_phone)
        .bind(&profile.medical_id)
        .bind(&profile.allergies)
        .bind(&profile.medical_history)
        .bind(&profile.notes)
        .bind(&profile.tags)
        .bind(&profile.metadata)
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn delete_profile(&self, patient_id: &PatientId) -> DomainResult<()> {
        sqlx::query("DELETE FROM patient_profile WHERE patient_id = $1")
            .bind(patient_id.as_uuid())
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }
}
