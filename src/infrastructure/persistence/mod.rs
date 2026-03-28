//! 持久化层实现

pub mod binding_repository;
pub mod device_repository;
pub mod healthdata_repository;
pub mod patient_repository;
pub mod role_repository;
pub mod user_repository;

pub use binding_repository::SqlxBindingRepository;
pub use device_repository::SqlxDeviceRepository;
pub use healthdata_repository::SqlxHealthDataRepository;
pub use patient_repository::SqlxPatientRepository;
pub use role_repository::SqlxRoleRepository;
pub use user_repository::SqlxUserRepository;
