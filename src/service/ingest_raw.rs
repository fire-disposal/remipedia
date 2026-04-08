use crate::core::entity::{RawDataQuery as CoreRawDataQuery, RawDataRecord, RawIngestStatus};
use crate::dto::request::RawDataQuery;
use crate::dto::response::{Pagination, RawDataQueryResponse, RawDataRecordResponse};
use crate::errors::{AppError, AppResult};
use crate::repository::RawDataRepository;
use sqlx::PgPool;

pub struct IngestRawService<'a> {
    raw_repo: RawDataRepository<'a>,
}

impl<'a> IngestRawService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            raw_repo: RawDataRepository::new(pool),
        }
    }

    pub async fn query(&self, query: RawDataQuery) -> AppResult<RawDataQueryResponse> {
        let core_query = CoreRawDataQuery {
            source: query.source,
            serial_number: query.serial_number,
            device_type: query.device_type,
            status: query.status.and_then(|s| s.parse::<RawIngestStatus>().ok()),
            start_time: query.start_time,
            end_time: query.end_time,
            page: query.page,
            page_size: query.page_size,
        };

        let total = self.raw_repo.count(&core_query).await?;
        let data = self.raw_repo.query(&core_query).await?;
        let records = data.into_iter().map(RawDataRecordResponse::from).collect();

        Ok(RawDataQueryResponse {
            data: records,
            pagination: Pagination::new(query.page, query.page_size, total),
        })
    }

    /// 导出为CSV格式
    pub fn export_csv(&self, records: &[RawDataRecordResponse]) -> AppResult<Vec<u8>> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        wtr.write_record([
            "id",
            "source",
            "serial_number",
            "device_type",
            "status",
            "status_message",
            "payload_size",
            "received_at",
            "processed_at",
            "created_at",
        ]).map_err(|e| AppError::ValidationError(format!("CSV写入失败: {}", e)))?;

        for record in records {
            wtr.write_record([
                record.id.to_string(),
                record.source.clone(),
                record.serial_number.clone().unwrap_or_default(),
                record.device_type.clone().unwrap_or_default(),
                record.status.clone(),
                record.status_message.clone().unwrap_or_default(),
                record.payload_size.to_string(),
                record.received_at.to_rfc3339(),
                record.processed_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                record.created_at.to_rfc3339(),
            ]).map_err(|e| AppError::ValidationError(format!("CSV写入失败: {}", e)))?;
        }

        wtr.flush().map_err(|e| AppError::ValidationError(format!("CSV刷新失败: {}", e)))?;

        Ok(wtr.into_inner().map_err(|e| AppError::ValidationError(format!("CSV获取数据失败: {}", e)))?)
    }
}

impl From<RawDataRecord> for RawDataRecordResponse {
    fn from(record: RawDataRecord) -> Self {
        let preview = record
            .raw_payload_text
            .as_ref()
            .map(|text| text.chars().take(500).collect::<String>());

        Self {
            id: record.id,
            source: record.source,
            serial_number: record.serial_number,
            device_type: record.device_type,
            status: record.status,
            status_message: record.status_message,
            payload_size: record.raw_payload.len(),
            raw_payload_preview: preview,
            received_at: record.received_at,
            processed_at: record.processed_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}
