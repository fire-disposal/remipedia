use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{Device, NewDevice};
use crate::errors::{AppError, AppResult};

pub struct DeviceRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DeviceRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Device> {
        sqlx::query_as::<_, Device>(
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("设备: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn find_by_serial(&self, serial_number: &str) -> AppResult<Option<Device>> {
        sqlx::query_as::<_, Device>(
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device WHERE serial_number = $1"#,
        )
        .bind(serial_number)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn exists_by_serial(&self, serial_number: &str) -> AppResult<bool> {
        let result: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM device WHERE serial_number = $1 LIMIT 1")
                .bind(serial_number)
                .fetch_optional(self.pool)
                .await
                .map_err(AppError::DatabaseError)?;

        Ok(result.is_some())
    }

    pub async fn insert(&self, device: &NewDevice) -> AppResult<Device> {
        sqlx::query_as::<_, Device>(
            r#"INSERT INTO device (serial_number, device_type, firmware_version, status, metadata)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at"#,
        )
        .bind(&device.serial_number)
        .bind(&device.device_type)
        .bind(&device.firmware_version)
        .bind(&device.status)
        .bind(&device.metadata)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn update_status(&self, id: &Uuid, status: &str) -> AppResult<Device> {
        sqlx::query_as::<_, Device>(
            r#"UPDATE device SET status = $2 WHERE id = $1
               RETURNING id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at"#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("设备: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn update(
        &self,
        id: &Uuid,
        firmware_version: Option<&str>,
        status: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> AppResult<Device> {
        sqlx::query_as::<_, Device>(
            r#"UPDATE device SET
               firmware_version = COALESCE($2, firmware_version),
               status = COALESCE($3, status),
               metadata = COALESCE($4, metadata)
               WHERE id = $1
               RETURNING id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at"#,
        )
        .bind(id)
        .bind(firmware_version)
        .bind(status)
        .bind(metadata)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("设备: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn find_all(
        &self,
        device_type: Option<&str>,
        status: Option<&str>,
        serial_number: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Device>> {
        let devices = sqlx::query_as::<_, Device>(
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device
               WHERE ($1::text IS NULL OR device_type = $1)
                 AND ($2::text IS NULL OR status = $2)
                 AND ($3::text IS NULL OR serial_number ILIKE '%' || $3 || '%')
               ORDER BY created_at DESC
               LIMIT $4 OFFSET $5"#,
        )
        .bind(device_type)
        .bind(status)
        .bind(serial_number)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(devices)
    }

    pub async fn count(
        &self,
        device_type: Option<&str>,
        status: Option<&str>,
        serial_number: Option<&str>,
    ) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM device
               WHERE ($1::text IS NULL OR device_type = $1)
                 AND ($2::text IS NULL OR status = $2)
                 AND ($3::text IS NULL OR serial_number ILIKE '%' || $3 || '%')"#,
        )
        .bind(device_type)
        .bind(status)
        .bind(serial_number)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    pub async fn delete(&self, id: &Uuid) -> AppResult<()> {
        let result = sqlx::query(r#"DELETE FROM device WHERE id = $1"#)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("设备: {}", id)));
        }

        Ok(())
    }
}
