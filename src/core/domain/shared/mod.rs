pub mod errors;
pub mod id;

pub use errors::{DomainError, DomainResult};
pub use id::{BindingId, DeviceId, PatientId, UserId};
