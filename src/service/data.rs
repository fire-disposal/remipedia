use chrono::Utc;
use log::info;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{DataPoint, DataQuery as CoreDataQuery};
use crate::dto::request::{AlertQuery, DataQuery, DataReportRequest};
use crate::dto::response::{AlertStatsResponse, DataQueryResponse, DataRecordResponse, DataReportResponse, Pagination};
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

    /// 数据入库（使用新的 DataPoint）
    pub async fn ingest(&self,
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
        data_type: String,
        payload: serde_json::Value,
    ) -> AppResult<DataReportResponse> {
        let datapoint = DataPoint {
            time: Utc::now(),
            device_id,
            patient_id,
            data_type: data_type.clone(),
            data_category: crate::core::entity::DataCategory::Metric,
            value_numeric: None,
            value_text: None,
            severity: None,
            status: None,
            payload,
            source: "mqtt".to_string(),
        };

        let result = self.data_repo.insert_datapoint(&datapoint).await?;

        info!(
            "数据入库成功: device_id={:?}, patient_id={:?}, data_type={}",
            result.device_id, result.patient_id, result.data_type
        );

        Ok(DataReportResponse {
            success: true,
            time: result.time,
            device_id: result.device_id,
            patient_id: result.patient_id,
        })
    }

    /// HTTP 数据上报
    pub async fn report_http(&self, req: DataReportRequest
    ) -> AppResult<DataReportResponse> {
        // 验证设备存在
        self.device_repo.find_by_id(&req.device_id).await?;

        // 获取当前绑定的患者
        let patient_id = match req.patient_id {
            Some(id) => Some(id),
            None => self
                .binding_repo
                .find_active_by_device(&req.device_id)
                .await?
                .map(|b| b.patient_id),
        };

        let datapoint = DataPoint {
            time: req.timestamp.unwrap_or_else(Utc::now),
            device_id: Some(req.device_id),
            patient_id,
            data_type: req.data_type,
            data_category: crate::core::entity::DataCategory::Metric,
            value_numeric: None,
            value_text: None,
            severity: None,
            status: None,
            payload: req.payload,
            source: "http".to_string(),
        };

        let result = self.data_repo.insert_datapoint(&datapoint).await?;

        Ok(DataReportResponse {
            success: true,
            time: result.time,
            device_id: result.device_id,
            patient_id: result.patient_id,
        })
    }

    /// 查询数据
    pub async fn query(&self, query: DataQuery) -> AppResult<DataQueryResponse> {
        let core_query = CoreDataQuery {
            patient_id: query.patient_id,
            device_id: query.device_id,
            data_type: query.data_type,
            data_category: query.data_category.and_then(|s| s.parse().ok()),
            severity: query.severity.and_then(|s| s.parse().ok()),
            status: query.status.and_then(|s| s.parse().ok()),
            start_time: query.start_time,
            end_time: query.end_time,
            page: query.page,
            page_size: query.page_size,
        };

        let total = self.data_repo.count(&core_query).await?;
        let data = self.data_repo.query(&core_query).await?;

        let records: Vec<DataRecordResponse> = data.into_iter().map(|d| d.into()).collect();

        Ok(DataQueryResponse {
            data: records,
            pagination: Pagination::new(query.page, query.page_size, total),
        })
    }

    /// 查询活跃告警
    pub async fn query_alerts(&self, query: AlertQuery
    ) -> AppResult<DataQueryResponse> {
        let core_query = CoreDataQuery {
            patient_id: query.patient_id,
            device_id: None,
            data_type: query.data_type,
            data_category: Some(crate::core::entity::DataCategory::Event),
            severity: query.severity.and_then(|s| s.parse().ok()),
            status: query.status.and_then(|s| s.parse().ok()),
            start_time: query.start_time,
            end_time: query.end_time,
            page: query.page,
            page_size: query.page_size,
        };

        let total = self.data_repo.count(&core_query).await?;
        let data = self.data_repo.query(&core_query).await?;

        let records: Vec<DataRecordResponse> = data.into_iter().map(|d| d.into()).collect();

        Ok(DataQueryResponse {
            data: records,
            pagination: Pagination::new(query.page, query.page_size, total),
        })
    }

    /// 获取告警统计
    pub async fn get_alert_stats(
        &self,
        patient_id: Option<&Uuid>,
    ) -> AppResult<AlertStatsResponse> {
        let stats = self.data_repo.get_stats(patient_id, None).await?;
        
        Ok(AlertStatsResponse {
            metric_count: stats.metric_count,
            event_count: stats.event_count,
            active_alert_count: stats.active_alert_count,
            critical_count: stats.critical_count,
        })
    }

    /// 确认事件
    pub async fn acknowledge_event(
        &self,
        patient_id: &Uuid,
        time: &chrono::DateTime<chrono::Utc>,
        device_id: Option<&Uuid>,
    ) -> AppResult<DataRecordResponse> {
        let result = self.data_repo.acknowledge_event(patient_id, time, device_id).await?;
        Ok(result.into())
    }

    /// 解决事件
    pub async fn resolve_event(
        &self,
        patient_id: &Uuid,
        time: &chrono::DateTime<chrono::Utc>,
        device_id: Option<&Uuid>,
    ) -> AppResult<DataRecordResponse> {
        let result = self.data_repo.resolve_event(patient_id, time, device_id).await?;
        Ok(result.into())
    }

    /// 按患者查询最新数据
    pub async fn get_latest_by_patient(
        &self,
        patient_id: &Uuid,
        data_type: Option<&str>,
        limit: i64,
    ) -> AppResult<Vec<DataRecordResponse>> {
        let data = self
            .data_repo
            .find_latest_by_patient(patient_id, data_type, limit)
            .await?;
        Ok(data.into_iter().map(|d| d.into()).collect())
    }
}

// 实体到响应的转换
impl From<crate::core::entity::Datasheet> for DataRecordResponse {
    fn from(data: crate::core::entity::Datasheet) -> Self {
        Self {
            time: data.time,
            device_id: data.device_id,
            patient_id: data.patient_id,
            data_type: data.data_type,
            data_category: data.data_category,
            value_numeric: data.value_numeric,
            value_text: data.value_text,
            severity: data.severity,
            status: data.status,
            payload: data.payload,
            source: data.source,
            ingested_at: data.ingested_at,
        }
    }
}
