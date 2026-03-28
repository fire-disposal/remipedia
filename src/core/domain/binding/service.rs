//! Binding领域服务
//!
//! 处理跨聚合的业务逻辑

use chrono::Utc;

use crate::core::domain::binding::entity::Binding;
use crate::core::domain::binding::repository::BindingRepository;
use crate::core::domain::device::repository::DeviceRepository;
use crate::core::domain::shared::{DeviceId, DomainError, DomainResult, PatientId};

/// Binding领域服务
pub struct BindingDomainService<DR: DeviceRepository, BR: BindingRepository> {
    device_repo: DR,
    binding_repo: BR,
}

impl<DR: DeviceRepository, BR: BindingRepository> BindingDomainService<DR, BR> {
    pub fn new(device_repo: DR, binding_repo: BR) -> Self {
        Self {
            device_repo,
            binding_repo,
        }
    }

    /// 绑定设备到患者
    ///
    /// 业务规则：
    /// 1. 设备必须存在且已激活
    /// 2. 如果设备已有有效绑定，先结束它
    /// 3. 创建新绑定
    pub async fn bind_device_to_patient(
        &self,
        device_id: DeviceId,
        patient_id: PatientId,
        notes: Option<String>,
    ) -> DomainResult<Binding> {
        // 1. 检查设备存在且激活
        let device = self
            .device_repo
            .find_by_id(&device_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("设备: {}", device_id)))?;

        if !device.is_active() {
            return Err(DomainError::Validation("设备未激活，无法绑定".into()));
        }

        // 2. 检查并结束现有绑定
        if let Some(mut existing) = self.binding_repo.find_active_by_device(&device_id).await? {
            existing.end(Utc::now())?;
            self.binding_repo.save(&existing).await?;
        }

        // 3. 创建新绑定
        let binding = Binding::create(device_id, patient_id, notes)?;
        self.binding_repo.save(&binding).await?;

        Ok(binding)
    }

    /// 解绑设备
    ///
    /// 结束设备的当前有效绑定
    pub async fn unbind_device(&self, device_id: &DeviceId) -> DomainResult<()> {
        if let Some(mut binding) = self.binding_repo.find_active_by_device(device_id).await? {
            binding.end(Utc::now())?;
            self.binding_repo.save(&binding).await?;
        }
        Ok(())
    }
}
