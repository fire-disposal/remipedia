use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建患者请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreatePatientRequest {
    #[validate(length(min = 1, max = 100, message = "姓名长度1-100"))]
    pub name: String,
    pub external_id: Option<String>,
}

/// 更新患者请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePatientRequest {
    pub name: Option<String>,
    pub external_id: Option<String>,
}

/// 创建患者档案请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePatientProfileRequest {
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

/// 患者查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientQuery {
    pub name: Option<String>,
    pub external_id: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}