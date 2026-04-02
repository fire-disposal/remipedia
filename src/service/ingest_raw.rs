use crate::core::entity::{RawDataQuery as CoreRawDataQuery, RawDataRecord, RawIngestStatus};
use crate::dto::request::RawDataQuery;
use crate::dto::response::{Pagination, RawDataQueryResponse, RawDataRecordResponse};
use crate::errors::AppResult;
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
            pagination: Pagination {
                page: query.page,
                page_size: query.page_size,
                total,
            },
        })
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
