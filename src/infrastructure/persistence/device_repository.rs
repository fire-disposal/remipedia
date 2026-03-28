//! Device仓储SQLx实现

use async_trait::async_trait;
use sqlx::PgPool;

use crate::core::domain::device::{Device, DeviceRepository};
use crate::core::domain::shared::{DeviceId, DomainError, DomainResult};
use crate::core::entity::Device as DeviceRow;
use crate::core::value_object::DeviceTypeId;

/// SQLx实现的Device仓储
pub struct SqlxDeviceRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SqlxDeviceRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 将数据库行转换为领域实体
    fn to_entity(row: DeviceRow) -> DomainResult<Device> {
        Ok(Device::reconstruct(
            DeviceId::from_uuid(row.id),
            row.serial_number,
            row.device_type,
            row.firmware_version,
            row.status,
            row.metadata,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[async_trait]
impl<'a> DeviceRepository for SqlxDeviceRepository<'a> {
    async fn find_by_id(&self, id: &DeviceId) -> DomainResult<Option<Device>> {
        let row = sqlx::query_as::<_, DeviceRow>(
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device WHERE id = $1"#,
        )
        .bind(id.as_uuid())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn find_by_serial(&self, serial: &str) -> DomainResult<Option<Device>> {
        let row = sqlx::query_as::<_, DeviceRow>(
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device WHERE serial_number = $1"#,
        )
        .bind(serial)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn exists_by_serial(&self, serial: &str) -> DomainResult<bool> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM device WHERE serial_number = $1 LIMIT 1"
        )
        .bind(serial)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(result.is_some())
    }

    async fn save(&self, device: &Device) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO device (id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (id) DO UPDATE SET
                   serial_number = EXCLUDED.serial_number,
                   device_type = EXCLUDED.device_type,
                   firmware_version = EXCLUDED.firmware_version,
                   status = EXCLUDED.status,
                   metadata = EXCLUDED.metadata,
                   updated_at = EXCLUDED.updated_at"#,
        )
        .bind(device.id().as_uuid())
        .bind(device.serial_number())
        .bind(device.device_type())
        .bind(device.firmware_version())
        .bind(device.status())
        .bind(device.metadata())
        .bind(device.created_at())
        .bind(device.updated_at())
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &DeviceId) -> DomainResult<()> {
        sqlx::query("DELETE FROM device WHERE id = $1")
            .bind(id.as_uuid())
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn find_by_type(
        &self,
        device_type: &DeviceTypeId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Device>> {
        let rows = sqlx::query_as::<_, DeviceRow>(
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device WHERE device_type = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(device_type.as_str())
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        rows.into_iter()
            .map(Self::to_entity)
            .collect::<Result<Vec<_>, _>>()
    }
}
