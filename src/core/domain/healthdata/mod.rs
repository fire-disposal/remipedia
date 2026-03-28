//! 健康数据领域模块
//!
//! 采用统一窄表存储策略：
//! - time: 数据产生时间
//! - device_id: 设备ID
//! - data_type: 数据类型标识
//! - payload: JSONB 格式的具体数据

pub mod repository;
pub mod types;
pub mod values;

pub use repository::{HealthDataQuery, HealthDataRepository, HourlyAggregation};
pub use types::{DataQuality, DataSource, DataType, HealthData};
pub use values::*;
