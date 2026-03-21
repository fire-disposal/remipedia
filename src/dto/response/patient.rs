use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::Pagination;

/// 患者响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatientResponse {
    /// 患者ID
    pub id: Uuid,
    /// 患者姓名
    pub name: String,
    /// 外部ID
    pub external_id: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 患者详情响应（含档案）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatientDetailResponse {
    /// 患者ID
    pub id: Uuid,
    /// 患者姓名
    pub name: String,
    /// 外部ID
    pub external_id: Option<String>,
    /// 患者档案
    pub profile: Option<PatientProfileResponse>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 患者档案响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatientProfileResponse {
    /// 出生日期
    pub date_of_birth: Option<chrono::NaiveDate>,
    /// 性别
    pub gender: Option<String>,
    /// 血型
    pub blood_type: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 地址
    pub address: Option<String>,
    /// 紧急联系人
    pub emergency_contact: Option<String>,
    /// 紧急联系电话
    pub emergency_phone: Option<String>,
    /// 医疗ID
    pub medical_id: Option<String>,
    /// 过敏史
    pub allergies: serde_json::Value,
    /// 病史
    pub medical_history: serde_json::Value,
    /// 备注
    pub notes: Option<String>,
    /// 标签
    pub tags: serde_json::Value,
}

/// 患者列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatientListResponse {
    /// 患者列表
    pub data: Vec<PatientResponse>,
    /// 分页信息
    pub pagination: Pagination,
}