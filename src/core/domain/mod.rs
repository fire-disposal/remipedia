//! 领域层（Domain Layer）
//!
//! 包含核心业务逻辑：实体、值对象、领域服务、仓储接口

pub mod device;
pub mod shared;

pub use shared::{BindingId, DeviceId, DomainError, DomainResult, PatientId, UserId};
