//! Patient 仓储接口

use async_trait::async_trait;

use crate::core::domain::patient::entity::{Patient, PatientProfile};
use crate::core::domain::shared::{DomainResult, PatientId};

/// Patient 仓储接口
#[async_trait]
pub trait PatientRepository: Send + Sync {
    /// 根据ID查找
    async fn find_by_id(&self, id: &PatientId) -> DomainResult<Option<Patient>>;

    /// 根据外部ID查找
    async fn find_by_external_id(&self, external_id: &str) -> DomainResult<Option<Patient>>;

    /// 检查外部ID是否存在
    async fn exists_by_external_id(&self, external_id: &str) -> DomainResult<bool>;

    /// 保存患者
    async fn save(&self, patient: &Patient) -> DomainResult<()>;

    /// 删除患者
    async fn delete(&self, id: &PatientId) -> DomainResult<()>;

    /// 查询患者列表
    async fn find_all(&self, name: Option<&str>, limit: i64, offset: i64) -> DomainResult<Vec<Patient>>;

    /// 获取档案
    async fn find_profile(&self, patient_id: &PatientId) -> DomainResult<Option<PatientProfile>>;

    /// 保存档案
    async fn save_profile(&self, patient_id: &PatientId, profile: &PatientProfile) -> DomainResult<()>;

    /// 删除档案
    async fn delete_profile(&self, patient_id: &PatientId) -> DomainResult<()>;
}
