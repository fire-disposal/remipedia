use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// 创建患者请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreatePatientRequest {
    /// 患者姓名
    #[validate(length(min = 1, max = 100, message = "姓名长度1-100"))]
    pub name: String,
    /// 外部ID
    pub external_id: Option<String>,
    /// 档案信息（可选，创建时一起创建）
    pub profile: Option<CreatePatientProfileRequest>,
}

/// 更新患者请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePatientRequest {
    /// 患者姓名
    pub name: Option<String>,
    /// 外部ID
    pub external_id: Option<String>,
}

/// 患者档案请求（用于创建和更新）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePatientProfileRequest {
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
    pub allergies: Option<serde_json::Value>,
    /// 病史
    pub medical_history: Option<serde_json::Value>,
    /// 备注
    pub notes: Option<String>,
    /// 标签
    pub tags: Option<serde_json::Value>,
    /// 元数据
    pub metadata: Option<serde_json::Value>,
}

/// 患者查询参数
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatientQuery {
    /// 姓名筛选（模糊匹配）
    pub name: Option<String>,
    /// 外部ID筛选
    pub external_id: Option<String>,
    /// 页码
    pub page: Option<u32>,
    /// 每页数量
    pub page_size: Option<u32>,
}
