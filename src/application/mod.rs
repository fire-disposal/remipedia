//! 应用层（Application Layer）
//!
//! 协调领域层完成用例，处理事务和DTO转换

use sqlx::PgPool;

use crate::core::domain::binding::{BindingDomainService, BindingRepository};
use crate::core::domain::device::DeviceRepository;
use crate::infrastructure::persistence::{SqlxBindingRepository, SqlxDeviceRepository};

pub mod device;

/// 应用上下文
pub struct AppContext<'a> {
    pool: &'a PgPool,
}

impl<'a> AppContext<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub fn device_repo(&self) -> SqlxDeviceRepository<'_> {
        SqlxDeviceRepository::new(self.pool)
    }

    pub fn binding_repo(&self) -> SqlxBindingRepository<'_> {
        SqlxBindingRepository::new(self.pool)
    }

    pub fn binding_service(&self) -> BindingDomainService<SqlxDeviceRepository<'_>, SqlxBindingRepository<'_>> {
        BindingDomainService::new(
            SqlxDeviceRepository::new(self.pool),
            SqlxBindingRepository::new(self.pool),
        )
    }
}
