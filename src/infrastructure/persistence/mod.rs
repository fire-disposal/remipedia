//! 持久化层实现

pub mod binding_repository;
pub mod device_repository;
pub mod patient_repository;
pub mod user_repository;

pub use binding_repository::SqlxBindingRepository;
pub use device_repository::SqlxDeviceRepository;
pub use patient_repository::SqlxPatientRepository;
pub use user_repository::SqlxUserRepository;
