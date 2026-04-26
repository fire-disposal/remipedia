use chrono::Utc;
use log::info;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{AlertEvent, DataStreamType, NewAlertEvent, NewObservation, Observation};
use crate::dto::request::{AlertQuery, DataQuery, DataReportRequest};
use crate::dto::response::{
    AlertEventResponse, AlertStatsResponse, DataQueryResponse, DataReportResponse, ObservationResponse,
    Pagination,
};
use crate::errors::AppResult;
use crate::repository::{
    AlertEventRepository, DataStreamRepository, ObservationRepository,
    PgAlertEventRepository, PgDataStreamRepository, PgObservationRepository,
};

/// DataService — 数据服务
///
/// 使用 Trait 对象注入 Repository，支持单元测试时传入 Mock 实现。
pub struct DataService {
    pool: PgPool,
    stream_repo: Box<dyn DataStreamRepository>,
    obs_repo: Box<dyn ObservationRepository>,
    alert_repo: Box<dyn AlertEventRepository>,
}

impl DataService {
    /// 使用具体 PostgreSQL 实现构造
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            stream_repo: Box::new(PgDataStreamRepository::new(pool.clone())),
            obs_repo: Box::new(PgObservationRepository::new(pool.clone())),
            alert_repo: Box::new(PgAlertEventRepository::new(pool)),
        }
    }

    /// 使用自定义 Repository 实现构造（主要用于测试注入 Mock）
    pub fn with_repos(
        pool: PgPool,
        stream_repo: Box<dyn DataStreamRepository>,
        obs_repo: Box<dyn ObservationRepository>,
        alert_repo: Box<dyn AlertEventRepository>,
    ) -> Self {
        Self {
            pool,
            stream_repo,
            obs_repo,
            alert_repo,
        }
    }

    // ============================================
    // 内部辅助
    // ============================================

    /// 通过设备 ID 查找当前绑定的患者
    pub(crate) async fn resolve_patient_from_device(&self, device_id: &Uuid) -> AppResult<Option<Uuid>> {
        let binding = sqlx::query_scalar::<_, Uuid>(
            r#"SELECT patient_id FROM binding WHERE device_id = $1 AND ended_at IS NULL LIMIT 1"#,
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::errors::AppError::DatabaseError)?;
        Ok(binding)
    }

    // ============================================
    // 数据写入
    // ============================================

    /// 数据入库（Ingest 内部调用）
    pub async fn ingest(
        &self,
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
        data_type: String,
        stream_type: DataStreamType,
        value_numeric: Option<rust_decimal::Decimal>,
        value_text: Option<String>,
        payload: serde_json::Value,
    ) -> AppResult<DataReportResponse> {
        // 自动从设备绑定填充 patient_id
        let patient_id = match patient_id {
            Some(id) => id,
            None => {
                if let Some(did) = device_id {
                    self.resolve_patient_from_device(&did)
                        .await?
                        .unwrap_or(Uuid::nil())
                } else {
                    Uuid::nil()
                }
            }
        };

        // 查找或创建 DataStream
        let stream = match device_id {
            Some(did) => {
                self.stream_repo
                    .find_or_create(&did, &data_type, &stream_type, Some(patient_id))
                    .await?
            }
            None => {
                // 无设备 ID 时，创建一个独立的数据流
                let name = format!("standalone_{}_{}", data_type, stream_type);
                let stream = crate::core::entity::DataStream::new(
                    name,
                    stream_type,
                    data_type.clone(),
                    None,
                    Some(patient_id),
                );
                self.stream_repo.create(&stream).await?
            }
        };

        // 写入观测数据
        let obs = NewObservation {
            stream_id: stream.id,
            patient_id,
            value_numeric,
            value_text,
            metadata: payload,
            recorded_at: Utc::now(),
        };
        let result = self.obs_repo.insert(&obs).await?;

        info!(
            "数据入库成功: stream_id={}, patient_id={}, data_type={}",
            result.stream_id, result.patient_id, data_type
        );

        Ok(DataReportResponse {
            success: true,
            time: result.recorded_at,
            stream_id: result.stream_id,
            observation_id: result.id,
        })
    }

    /// HTTP 数据上报
    pub async fn report_http(&self, req: DataReportRequest) -> AppResult<DataReportResponse> {
        // 获取当前绑定的患者
        let patient_id = match req.patient_id {
            Some(id) => id,
            None => self
                .resolve_patient_from_device(&req.device_id)
                .await?
                .unwrap_or(Uuid::nil()),
        };

        // 查找或创建 DataStream（HTTP 上报默认是 metric 类型）
        let stream = self
            .stream_repo
            .find_or_create(
                &req.device_id,
                &req.data_type,
                &DataStreamType::Metric,
                Some(patient_id),
            )
            .await?;

        let obs = NewObservation {
            stream_id: stream.id,
            patient_id,
            value_numeric: None,
            value_text: None,
            metadata: req.payload,
            recorded_at: req.timestamp.unwrap_or_else(Utc::now),
        };
        let result = self.obs_repo.insert(&obs).await?;

        Ok(DataReportResponse {
            success: true,
            time: result.recorded_at,
            stream_id: result.stream_id,
            observation_id: result.id,
        })
    }

    // ============================================
    // 观测数据查询
    // ============================================

    /// 查询观测数据
    pub async fn query(&self, query: DataQuery) -> AppResult<DataQueryResponse> {
        let patient_id = query.patient_id.unwrap_or(Uuid::nil());
        let limit = query.page_size as i64;
        let offset = ((query.page.max(1) - 1) * query.page_size) as i64;

        let total = self.obs_repo.count(&patient_id).await?;
        let data = self.obs_repo.query(&patient_id, limit, offset).await?;

        let records: Vec<ObservationResponse> = data.into_iter().map(Into::into).collect();

        Ok(DataQueryResponse {
            data: records,
            pagination: Pagination::new(query.page, query.page_size, total),
        })
    }

    /// 获取患者最新的观测数据
    pub async fn get_latest_by_patient(
        &self,
        patient_id: &Uuid,
        _stream_type: Option<&str>,
        _limit: i64,
    ) -> AppResult<Vec<ObservationResponse>> {
        let result = self.obs_repo.find_latest_by_patient(patient_id).await?;
        Ok(result.into_iter().map(Into::into).collect())
    }

    // ============================================
    // 告警事件管理
    // ============================================

    /// 查询告警事件
    pub async fn query_alerts(&self, query: AlertQuery) -> AppResult<Vec<AlertEventResponse>> {
        let patient_id = query.patient_id.unwrap_or(Uuid::nil());
        let events = self.alert_repo.query_active(&patient_id).await?;
        Ok(events.into_iter().map(Into::into).collect())
    }

    /// 获取告警统计
    pub async fn get_alert_stats(
        &self,
        patient_id: Option<&Uuid>,
    ) -> AppResult<AlertStatsResponse> {
        let nil_uuid = Uuid::nil();
        let pid = patient_id.unwrap_or(&nil_uuid);
        let stats = self.alert_repo.get_stats(pid).await?;

        Ok(AlertStatsResponse {
            total_active: stats.total_active,
            total_acknowledged: stats.total_acknowledged,
            total_resolved: stats.total_resolved,
            by_severity: stats.by_severity,
        })
    }

    /// 确认告警事件（按 event_id）
    pub async fn acknowledge_event(
        &self,
        event_id: &Uuid,
        user_id: &Uuid,
    ) -> AppResult<AlertEventResponse> {
        let result = self.alert_repo.acknowledge(event_id, user_id).await?;
        Ok(result.into())
    }

    /// 解决告警事件（按 event_id）
    pub async fn resolve_event(
        &self,
        event_id: &Uuid,
        user_id: &Uuid,
    ) -> AppResult<AlertEventResponse> {
        let result = self.alert_repo.resolve(event_id, user_id).await?;
        Ok(result.into())
    }

    // ============================================
    // Ingest 辅助 — 批量写入观测 + 告警
    // ============================================

    /// 插入观测数据（Ingest 模块直接调用）
    pub async fn insert_observation(&self, obs: &NewObservation) -> AppResult<Observation> {
        self.obs_repo.insert(obs).await
    }

    /// 插入告警事件（Ingest 模块直接调用）
    pub async fn insert_alert(&self, alert: &NewAlertEvent) -> AppResult<AlertEvent> {
        self.alert_repo.insert(alert).await
    }

    /// 查找或创建数据流（Ingest 模块直接调用）
    pub async fn find_or_create_stream(
        &self,
        device_id: &Uuid,
        data_type: &str,
        stream_type: &DataStreamType,
        patient_id: Option<Uuid>,
    ) -> AppResult<crate::core::entity::DataStream> {
        self.stream_repo
            .find_or_create(device_id, data_type, stream_type, patient_id)
            .await
    }
}
