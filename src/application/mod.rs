//! 应用层（Application Layer）
//!
//! 协调领域层完成用例，处理事务和DTO转换

use sqlx::PgPool;

use crate::config::JwtConfig;
use crate::infrastructure::persistence::{SqlxBindingRepository, SqlxDeviceRepository, SqlxHealthDataRepository, SqlxPatientRepository, SqlxUserRepository};

pub mod auth;
pub mod binding;
pub mod device;
pub mod healthdata;
pub mod patient;
pub mod user;

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

    pub fn patient_repo(&self) -> SqlxPatientRepository<'_> {
        SqlxPatientRepository::new(self.pool)
    }

    pub fn user_repo(&self) -> SqlxUserRepository<'_> {
        SqlxUserRepository::new(self.pool)
    }

    pub fn auth_service(&self, jwt_config: &'a JwtConfig) -> auth::AuthAppService<'_> {
        auth::AuthAppService::new(self.pool, jwt_config)
    }

    pub fn health_service(&self) -> healthdata::HealthDataAppService<'_> {
        healthdata::HealthDataAppService::new(self.pool)
    }
}
