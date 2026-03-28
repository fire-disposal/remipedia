//! Device仓储接口（领域层）

use async_trait::async_trait;

use crate::core::domain::shared::{DeviceId, DomainResult};
use crate::core::domain::device::entity::Device;
use crate::core::value_object::DeviceTypeId;

/// Device仓储接口
#[async_trait]
pub trait DeviceRepository: Send + Sync {
    /// 根据ID查找
    async fn find_by_id(&self, id: &DeviceId) -> DomainResult<Option<Device>>;

    /// 根据序列号查找
    async fn find_by_serial(&self, serial: &str) -> DomainResult<Option<Device>>;

    /// 检查序列号是否存在
    async fn exists_by_serial(&self, serial: &str) -> DomainResult<bool>;

    /// 保存（新增或更新）
    async fn save(&self, device: &Device) -> DomainResult<()>;

    /// 删除
    async fn delete(&self, id: &DeviceId) -> DomainResult<()>;

    /// 根据设备类型查找
    async fn find_by_type(
        &self,
        device_type: &DeviceTypeId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Device>>;
}
