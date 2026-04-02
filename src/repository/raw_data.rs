use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{RawDataQuery, RawDataRecord, RawIngestStatus};
use crate::errors::{AppError, AppResult};
use crate::ingest::DataPacket;

pub struct RawDataRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> RawDataRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn archive_received(&self, packet: &DataPacket) -> AppResult<Uuid> {
        let metadata = serde_json::json!({
            "source": packet.metadata.source,
        });

        let raw_payload_text = std::str::from_utf8(&packet.raw).ok().map(|s| s.to_string());

        let result: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO ingest_raw_data (
                source, serial_number, device_type, remote_addr,
                metadata, raw_payload, raw_payload_text, status, received_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'stored', NOW())
            RETURNING id"#,
        )
        .bind(&packet.metadata.source)
        .bind(&packet.metadata.serial_number)
        .bind(&packet.metadata.device_type)
        .bind(&packet.metadata.remote_addr)
        .bind(metadata)
        .bind(&packet.raw)
        .bind(raw_payload_text)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    pub async fn mark_status(
        &self,
        id: Uuid,
        status: RawIngestStatus,
        message: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"UPDATE ingest_raw_data
            SET status = $2,
                status_message = $3,
                processed_at = $4,
                updated_at = NOW()
            WHERE id = $1"#,
        )
        .bind(id)
        .bind(status.to_string())
        .bind(message)
        .bind(Utc::now())
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    pub async fn query(&self, query: &RawDataQuery) -> AppResult<Vec<RawDataRecord>> {
        let limit = query.page_size as i64;
        let offset = ((query.page.saturating_sub(1)) * query.page_size) as i64;

        let data = sqlx::query_as::<_, RawDataRecord>(
            r#"SELECT
                id, source, serial_number, device_type, remote_addr,
                metadata, raw_payload, raw_payload_text, status, status_message,
                received_at, processed_at, created_at, updated_at
            FROM ingest_raw_data
            WHERE ($1::text IS NULL OR source = $1)
              AND ($2::text IS NULL OR serial_number = $2)
              AND ($3::text IS NULL OR device_type = $3)
              AND ($4::text IS NULL OR status = $4)
              AND ($5::timestamptz IS NULL OR received_at >= $5)
              AND ($6::timestamptz IS NULL OR received_at <= $6)
            ORDER BY received_at DESC
            LIMIT $7 OFFSET $8"#,
        )
        .bind(&query.source)
        .bind(&query.serial_number)
        .bind(&query.device_type)
        .bind(query.status.as_ref().map(|s| s.to_string()))
        .bind(query.start_time)
        .bind(query.end_time)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(data)
    }

    pub async fn count(&self, query: &RawDataQuery) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)
            FROM ingest_raw_data
            WHERE ($1::text IS NULL OR source = $1)
              AND ($2::text IS NULL OR serial_number = $2)
              AND ($3::text IS NULL OR device_type = $3)
              AND ($4::text IS NULL OR status = $4)
              AND ($5::timestamptz IS NULL OR received_at >= $5)
              AND ($6::timestamptz IS NULL OR received_at <= $6)"#,
        )
        .bind(&query.source)
        .bind(&query.serial_number)
        .bind(&query.device_type)
        .bind(query.status.as_ref().map(|s| s.to_string()))
        .bind(query.start_time)
        .bind(query.end_time)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }
}
