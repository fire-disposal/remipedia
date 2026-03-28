//! Patient 富实体

use chrono::{DateTime, NaiveDate, Utc};

use crate::core::domain::shared::{DomainError, DomainResult, PatientId};
use crate::core::value_object::{BloodType, Gender};

/// 患者实体
#[derive(Debug, Clone)]
pub struct Patient {
    id: PatientId,
    name: String,
    external_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// 患者档案（值对象）
#[derive(Debug, Clone, Default)]
pub struct PatientProfile {
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub blood_type: Option<BloodType>,
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
}

impl Patient {
    /// 创建患者
    pub fn create(name: String, external_id: Option<String>) -> DomainResult<Self> {
        if name.len() < 2 {
            return Err(DomainError::Validation("姓名至少2个字符".into()));
        }

        let now = Utc::now();
        Ok(Self {
            id: PatientId::new(),
            name,
            external_id,
            created_at: now,
            updated_at: now,
        })
    }

    /// 从持久化重建
    pub fn reconstruct(
        id: PatientId,
        name: String,
        external_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            external_id,
            created_at,
            updated_at,
        }
    }

    /// 更新姓名
    pub fn update_name(&mut self, name: String) -> DomainResult<()> {
        if name.len() < 2 {
            return Err(DomainError::Validation("姓名至少2个字符".into()));
        }
        self.name = name;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 更新外部ID
    pub fn update_external_id(&mut self, external_id: Option<String>) {
        self.external_id = external_id;
        self.updated_at = Utc::now();
    }

    // Getters
    pub fn id(&self) -> PatientId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn external_id(&self) -> Option<&str> {
        self.external_id.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl PatientProfile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_basic_info(
        mut self,
        date_of_birth: Option<NaiveDate>,
        gender: Option<Gender>,
        blood_type: Option<BloodType>,
    ) -> Self {
        self.date_of_birth = date_of_birth;
        self.gender = gender;
        self.blood_type = blood_type;
        self
    }

    pub fn with_contact(
        mut self,
        contact_phone: Option<String>,
        address: Option<String>,
    ) -> Self {
        self.contact_phone = contact_phone;
        self.address = address;
        self
    }

    pub fn with_emergency(
        mut self,
        emergency_contact: Option<String>,
        emergency_phone: Option<String>,
    ) -> Self {
        self.emergency_contact = emergency_contact;
        self.emergency_phone = emergency_phone;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patient_create() {
        let patient = Patient::create("张三".into(), Some("P001".into())).unwrap();
        assert_eq!(patient.name(), "张三");
        assert_eq!(patient.external_id(), Some("P001"));
    }

    #[test]
    fn test_patient_create_validation() {
        let result = Patient::create("张".into(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_patient_update_name() {
        let mut patient = Patient::create("张三".into(), None).unwrap();
        patient.update_name("张三丰".into()).unwrap();
        assert_eq!(patient.name(), "张三丰");
    }

    #[test]
    fn test_patient_profile_builder() {
        let profile = PatientProfile::new()
            .with_basic_info(
                Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
                Some(Gender::Male),
                Some(BloodType::APositive),
            )
            .with_contact(Some("13800138000".into()), Some("北京市".into()));

        assert_eq!(profile.gender, Some(Gender::Male));
        assert_eq!(profile.contact_phone, Some("13800138000".into()));
    }
}
