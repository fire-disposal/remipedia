//! Binding仓储接口

use async_trait::async_trait;

use chrono::{DateTime, Utc};

use crate::core::domain::binding::entity::Binding;
use crate::core::domain::shared::{BindingId, DeviceId, DomainResult, PatientId};

/// Binding仓储接口
#[async_trait]
pub trait BindingRepository: Send + Sync {
    /// 根据ID查找
    async fn find_by_id(&self, id: &BindingId) -> DomainResult<Option<Binding>>;

    /// 查找设备的当前有效绑定
    async fn find_active_by_device(&self, device_id: &DeviceId) -> DomainResult<Option<Binding>>;

    /// 查找患者的当前有效绑定
    async fn find_active_by_patient(&self, patient_id: &PatientId) -> DomainResult<Option<Binding>>;

    /// 查找设备的所有绑定历史
    async fn find_by_device(
        &self,
        device_id: &DeviceId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Binding>>;

    /// 查找患者的所有绑定历史
    async fn find_by_patient(
        &self,
        patient_id: &PatientId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Binding>>;

    /// 保存绑定
    async fn save(&self, binding: &Binding) -> DomainResult<()>;

    /// 结束绑定（更新ended_at）
    async fn end_binding(&self, id: &BindingId, ended_at: DateTime<Utc>) -> DomainResult<()>;
}
