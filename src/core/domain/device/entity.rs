//! Device富实体

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::core::domain::shared::{DeviceId, DomainError, DomainResult};
use crate::core::value_object::{DeviceStatus, DeviceTypeId};

/// 设备富实体
#[derive(Debug, Clone)]
pub struct Device {
    id: DeviceId,
    serial_number: String,
    device_type: DeviceTypeId,
    firmware_version: Option<String>,
    status: DeviceStatus,
    metadata: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Device {
    /// 创建设备（工厂方法）
    pub fn create(serial_number: String, device_type: DeviceTypeId) -> DomainResult<Self> {
        if serial_number.len() < 4 {
            return Err(DomainError::Validation(
                "序列号至少4个字符".into(),
            ));
        }

        let now = Utc::now();
        Ok(Self {
            id: DeviceId::new(),
            serial_number,
            device_type,
            firmware_version: None,
            status: DeviceStatus::Inactive,
            metadata: Value::Object(serde_json::Map::new()),
            created_at: now,
            updated_at: now,
        })
    }

    /// 从持久化重建
    pub fn reconstruct(
        id: DeviceId,
        serial_number: String,
        device_type: DeviceTypeId,
        firmware_version: Option<String>,
        status: DeviceStatus,
        metadata: Value,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            serial_number,
            device_type,
            firmware_version,
            status,
            metadata,
            created_at,
            updated_at,
        }
    }

    // ========== 领域行为 ==========

    /// 激活设备
    pub fn activate(&mut self) -> DomainResult<()> {
        self.status = DeviceStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 停用设备
    pub fn deactivate(&mut self) -> DomainResult<()> {
        if !self.status.can_transition_to(DeviceStatus::Inactive) {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: "inactive".into(),
            });
        }
        self.status = DeviceStatus::Inactive;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 进入维护模式
    pub fn start_maintenance(&mut self) -> DomainResult<()> {
        self.status = DeviceStatus::Maintenance;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 更新固件版本
    pub fn update_firmware(&mut self, version: String) {
        self.firmware_version = Some(version);
        self.updated_at = Utc::now();
    }

    /// 更新元数据
    pub fn update_metadata(&mut self, metadata: Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    // ========== 查询方法 ==========

    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    pub fn can_bind(&self) -> bool {
        self.is_active()
    }

    pub fn id(&self) -> DeviceId {
        self.id
    }

    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    pub fn device_type(&self) -> &DeviceTypeId {
        &self.device_type
    }

    pub fn status(&self) -> DeviceStatus {
        self.status
    }

    pub fn firmware_version(&self) -> Option<&str> {
        self.firmware_version.as_deref()
    }

    pub fn metadata(&self) -> &Value {
        &self.metadata
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_create() {
        let device = Device::create(
            "TEST001".into(),
            DeviceTypeId::new(DeviceTypeId::HEART_RATE_MONITOR),
        )
        .unwrap();

        assert_eq!(device.serial_number(), "TEST001");
        assert!(!device.is_active());
    }

    #[test]
    fn test_device_create_validation() {
        let result = Device::create("ABC".into(), DeviceTypeId::new("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_device_activate() {
        let mut device = Device::create("TEST001".into(), DeviceTypeId::new("test")).unwrap();
        assert!(!device.is_active());

        device.activate().unwrap();
        assert!(device.is_active());
    }

    #[test]
    fn test_device_status_transition() {
        let mut device = Device::create("TEST001".into(), DeviceTypeId::new("test")).unwrap();

        // Inactive -> Active -> Maintenance -> Active
        device.activate().unwrap();
        assert!(device.is_active());

        device.start_maintenance().unwrap();
        assert!(!device.is_active());

        // Maintenance不能直接到Inactive
        let result = device.deactivate();
        assert!(result.is_err());

        // 必须先回到Active
        device.activate().unwrap();
        device.deactivate().unwrap();
        assert!(!device.is_active());
    }
}
