use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::core::entity::IngestData;
use crate::dto::request::{DataQuery, DataReportRequest};
use crate::dto::response::{DataQueryResponse, DataRecordResponse, DataReportResponse, Pagination};
use crate::errors::AppResult;
use crate::repository::{BindingRepository, DataRepository, DeviceRepository};

pub struct DataService<'a> {
    data_repo: DataRepository<'a>,
    device_repo: DeviceRepository<'a>,
    binding_repo: BindingRepository<'a>,
}

impl<'a> DataService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            data_repo: DataRepository::new(pool),
            device_repo: DeviceRepository::new(pool),
            binding_repo: BindingRepository::new(pool),
        }
    }

    /// 数据入库
    pub async fn ingest(&self, data: IngestData) -> AppResult<DataReportResponse> {
        let result = self.data_repo.insert(&data).await?;
        
        info!(
            device_id = %result.device_id,
            subject_id = ?result.subject_id,
            data_type = %result.data_type,
            "数据入库成功"
        );

        Ok(DataReportResponse {
            success: true,
            time: result.time,
            device_id: result.device_id,
        })
    }

    /// HTTP 数据上报
    #[instrument(skip(self))]
    pub async fn report_http(&self, req: DataReportRequest) -> AppResult<DataReportResponse> {
        // 验证设备存在
        self.device_repo.find_by_id(&req.device_id).await?;

        // 获取当前绑定的患者
        let subject_id = match req.subject_id {
            Some(id) => Some(id),
            None => self.binding_repo.find_active_by_device(&req.device_id).await?.map(|b| b.patient_id),
        };

        let data = IngestData {
            time: req.timestamp.unwrap_or_else(Utc::now),
            device_id: req.device_id,
            subject_id,
            data_type: req.data_type,
            payload: req.payload,
            source: "http".to_string(),
        };

        self.ingest(data).await
    }

    /// 查询数据
    pub async fn query(&self, query: DataQuery) -> AppResult<DataQueryResponse> {
        let total = self.data_repo.count(&query).await?;
        let data = self.data_repo.query(&query).await?;

        let records: Vec<DataRecordResponse> = data.into_iter().map(|d| d.into()).collect();

        Ok(DataQueryResponse {
            data: records,
            pagination: Pagination {
                page: query.page,
                page_size: query.page_size,
                total,
                total_pages: (total + query.page_size as i64 - 1) / query.page_size as i64,
            },
        })
    }

    /// 按设备查询最新数据
    pub async fn get_latest(&self, device_id: &Uuid, data_type: Option<&str>) -> AppResult<Option<DataRecordResponse>> {
        let data = self.data_repo.find_latest(device_id, data_type).await?;
        Ok(data.map(|d| d.into()))
    }

    /// 按患者查询最新数据
    pub async fn get_latest_by_subject(&self, subject_id: &Uuid, data_type: Option<&str>, limit: i64) -> AppResult<Vec<DataRecordResponse>> {
        let data = self.data_repo.find_latest_by_subject(subject_id, data_type, limit).await?;
        Ok(data.into_iter().map(|d| d.into()).collect())
    }
}

// 实体到响应的转换
impl From<crate::core::entity::Datasheet> for DataRecordResponse {
    fn from(data: crate::core::entity::Datasheet) -> Self {
        Self {
            time: data.time,
            device_id: data.device_id,
            subject_id: data.subject_id,
            data_type: data.data_type,
            payload: data.payload,
            source: data.source,
            ingested_at: data.ingested_at,
        }
    }
}