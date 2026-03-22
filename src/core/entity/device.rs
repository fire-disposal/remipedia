use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 设备实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub serial_number: String,
    pub device_type: String,
    pub firmware_version: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新设备
#[derive(Debug, Clone, Default)]
pub struct NewDevice {
    pub serial_number: String,
    pub device_type: String,
    pub firmware_version: Option<String>,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
}

impl NewDevice {
    pub fn new(serial_number: String, device_type: String) -> Self {
        Self {
            serial_number,
            device_type,
            firmware_version: None,
            status: "active".to_string(),
            metadata: None,
        }
    }
}
