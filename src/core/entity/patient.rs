use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 患者实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Patient {
    pub id: Uuid,
    pub name: String,
    pub external_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新患者
#[derive(Debug, Clone)]
pub struct NewPatient {
    pub name: String,
    pub external_id: Option<String>,
}

/// 患者档案
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PatientProfile {
    pub patient_id: Uuid,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub gender: Option<String>,
    pub blood_type: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub emergency_contact: Option<String>,
    pub emergency_phone: Option<String>,
    pub medical_id: Option<String>,
    pub allergies: serde_json::Value,
    pub medical_history: serde_json::Value,
    pub notes: Option<String>,
    pub tags: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新患者档案
#[derive(Debug, Clone)]
pub struct NewPatientProfile {
    pub patient_id: Uuid,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub gender: Option<String>,
    pub blood_type: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub emergency_contact: Option<String>,
    pub emergency_phone: Option<String>,
    pub medical_id: Option<String>,
    pub allergies: Option<serde_json::Value>,
    pub medical_history: Option<serde_json::Value>,
    pub notes: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}
