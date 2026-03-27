pub mod adapters;
pub mod adapter_manager;

pub use adapters::*;
pub use adapter_manager::*;
pub mod transport; // Expose ingest transport module for unified transport management
