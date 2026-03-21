use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{NewPatient, NewPatientProfile, Patient, PatientProfile};
use crate::errors::{AppError, AppResult};

pub struct PatientRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PatientRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Patient> {
        sqlx::query_as::<_, Patient>(
            r#"SELECT id, name, external_id, created_at, updated_at
               FROM patient WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("患者: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn find_by_external_id(&self, external_id: &str) -> AppResult<Option<Patient>> {
        sqlx::query_as::<_, Patient>(
            r#"SELECT id, name, external_id, created_at, updated_at
               FROM patient WHERE external_id = $1"#,
        )
        .bind(external_id)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn insert(&self, patient: &NewPatient) -> AppResult<Patient> {
        sqlx::query_as::<_, Patient>(
            r#"INSERT INTO patient (name, external_id)
               VALUES ($1, $2)
               RETURNING id, name, external_id, created_at, updated_at"#,
        )
        .bind(&patient.name)
        .bind(&patient.external_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn update(&self, id: &Uuid, name: Option<&str>, external_id: Option<&str>) -> AppResult<Patient> {
        sqlx::query_as::<_, Patient>(
            r#"UPDATE patient SET name = COALESCE($2, name), external_id = COALESCE($3, external_id)
               WHERE id = $1
               RETURNING id, name, external_id, created_at, updated_at"#,
        )
        .bind(id)
        .bind(name)
        .bind(external_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("患者: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn find_all(&self, name: Option<&str>, external_id: Option<&str>, limit: i64, offset: i64) -> AppResult<Vec<Patient>> {
        let patients = sqlx::query_as::<_, Patient>(
            r#"SELECT id, name, external_id, created_at, updated_at
               FROM patient
               WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')
                 AND ($2::text IS NULL OR external_id = $2)
               ORDER BY created_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(name)
        .bind(external_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(patients)
    }

    pub async fn count(&self, name: Option<&str>, external_id: Option<&str>) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM patient
               WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')
                 AND ($2::text IS NULL OR external_id = $2)"#,
        )
        .bind(name)
        .bind(external_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    pub async fn delete(&self, id: &Uuid) -> AppResult<()> {
        let result = sqlx::query(r#"DELETE FROM patient WHERE id = $1"#)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("患者: {}", id)));
        }

        Ok(())
    }

    // ========== 患者档案 ==========

    pub async fn find_profile(&self, patient_id: &Uuid) -> AppResult<Option<PatientProfile>> {
        sqlx::query_as::<_, PatientProfile>(
            r#"SELECT patient_id, date_of_birth, gender, blood_type, contact_phone, address,
                      emergency_contact, emergency_phone, medical_id, allergies, medical_history,
                      notes, tags, metadata, created_at, updated_at
               FROM patient_profile WHERE patient_id = $1"#,
        )
        .bind(patient_id)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn insert_profile(&self, profile: &NewPatientProfile) -> AppResult<PatientProfile> {
        sqlx::query_as::<_, PatientProfile>(
            r#"INSERT INTO patient_profile (patient_id, date_of_birth, gender, blood_type, contact_phone,
                      address, emergency_contact, emergency_phone, medical_id, allergies, medical_history,
                      notes, tags, metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
               RETURNING patient_id, date_of_birth, gender, blood_type, contact_phone, address,
                      emergency_contact, emergency_phone, medical_id, allergies, medical_history,
                      notes, tags, metadata, created_at, updated_at"#,
        )
        .bind(profile.patient_id)
        .bind(profile.date_of_birth)
        .bind(&profile.gender)
        .bind(&profile.blood_type)
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
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn delete_profile(&self, patient_id: &Uuid) -> AppResult<()> {
        sqlx::query(r#"DELETE FROM patient_profile WHERE patient_id = $1"#)
            .bind(patient_id)
            .execute(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }
}