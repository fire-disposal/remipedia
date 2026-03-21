use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 患者响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientResponse {
    pub id: Uuid,
    pub name: String,
    pub external_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 患者详情响应（含档案）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub external_id: Option<String>,
    pub profile: Option<PatientProfileResponse>,
    pub created_at: DateTime<Utc>,
}

/// 患者档案响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientProfileResponse {
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
}

/// 患者列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientListResponse {
    pub data: Vec<PatientResponse>,
    pub pagination: super::Pagination,
}