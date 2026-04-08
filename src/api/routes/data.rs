use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post};
use sqlx::PgPool;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;
use serde::Deserialize;

use crate::api::guards::ModuleGuard;
use crate::dto::request::DataReportRequest;
use crate::dto::response::{DataQueryResponse, DataRecordResponse, DataReportResponse};
use crate::errors::AppResult;
use crate::service::DataService;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AcknowledgeRequest {
    pub patient_id: Uuid,
    pub time: chrono::DateTime<chrono::Utc>,
    pub device_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolveRequest {
    pub patient_id: Uuid,
    pub time: chrono::DateTime<chrono::Utc>,
    pub device_id: Option<Uuid>,
}

/// 数据上报
#[utoipa::path(
    post,
    path = "/data",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    request_body = DataReportRequest,
    responses(
        (status = 200, description = "上报成功", body = DataReportResponse),
        (status = 404, description = "设备不存在"),
    )
)]
#[post("/data", data = "<req>")]
pub async fn report_data(
    pool: &State<PgPool>,
    _guard: ModuleGuard,
    req: Json<DataReportRequest>,
) -> AppResult<Json<DataReportResponse>> {
    let service = DataService::new(pool);
    let response = service.report_http(req.into_inner()).await?;
    Ok(Json(response))
}

/// 查询数据
#[utoipa::path(
    get,
    path = "/data",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("patient_id" = Option<String>, Query, description = "患者ID筛选"),
        ("device_id" = Option<String>, Query, description = "设备ID筛选"),
        ("data_type" = Option<String>, Query, description = "数据类型筛选"),
        ("data_category" = Option<String>, Query, description = "数据分类 (metric/event)"),
        ("severity" = Option<String>, Query, description = "严重级别 (info/warning/alert)"),
        ("status" = Option<String>, Query, description = "状态 (active/acknowledged/resolved)"),
        ("start_time" = Option<String>, Query, description = "开始时间 (RFC3339)"),
        ("end_time" = Option<String>, Query, description = "结束时间 (RFC3339)"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = DataQueryResponse),
    )
)]
#[get("/data?<patient_id>&<device_id>&<data_type>&<data_category>&<severity>&<status>&<start_time>&<end_time>&<page>&<page_size>")]
pub async fn query_data(
    pool: &State<PgPool>,
    _guard: ModuleGuard,
    patient_id: Option<String>,
    device_id: Option<String>,
    data_type: Option<String>,
    data_category: Option<String>,
    severity: Option<String>,
    status: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<DataQueryResponse>> {
    let service = DataService::new(pool);

    let query = crate::dto::request::DataQuery {
        patient_id: patient_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        device_id: device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        data_type,
        data_category,
        severity,
        status,
        start_time: start_time.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        end_time: end_time.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    };

    let response = service.query(query).await?;
    Ok(Json(response))
}

/// 查询活跃告警
#[utoipa::path(
    get,
    path = "/data/alerts",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("patient_id" = Option<String>, Query, description = "患者ID筛选"),
        ("data_type" = Option<String>, Query, description = "数据类型筛选"),
        ("severity" = Option<String>, Query, description = "严重级别 (info/warning/alert)"),
        ("limit" = Option<i64>, Query, description = "返回数量限制"),
    ),
    responses(
        (status = 200, description = "查询成功", body = DataQueryResponse),
    )
)]
#[get("/data/alerts?<patient_id>&<data_type>&<severity>&<limit>")]
pub async fn query_alerts(
    pool: &State<PgPool>,
    _guard: ModuleGuard,
    patient_id: Option<String>,
    data_type: Option<String>,
    severity: Option<String>,
    limit: Option<i64>,
) -> AppResult<Json<Vec<DataRecordResponse>>> {
    let service = DataService::new(pool);
    
    let query = crate::dto::request::AlertQuery {
        patient_id: patient_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        data_type,
        severity,
        status: Some("active".to_string()),
        start_time: None,
        end_time: None,
        page: 1,
        page_size: (limit.unwrap_or(50) as u32),
    };

    let response = service.query_alerts(query).await?;
    Ok(Json(response.data))
}

/// 确认事件
#[utoipa::path(
    post,
    path = "/data/events/acknowledge",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    request_body = AcknowledgeRequest,
    responses(
        (status = 200, description = "确认成功", body = DataRecordResponse),
        (status = 404, description = "事件不存在"),
    )
)]
#[post("/data/events/acknowledge", data = "<req>")]
pub async fn acknowledge_event(
    pool: &State<PgPool>,
    _guard: ModuleGuard,
    req: Json<AcknowledgeRequest>,
) -> AppResult<Json<DataRecordResponse>> {
    let service = DataService::new(pool);
    let result = service.acknowledge_event(&req.patient_id, &req.time, req.device_id.as_ref()).await?;
    Ok(Json(result.into()))
}

/// 解决事件
#[utoipa::path(
    post,
    path = "/data/events/resolve",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    request_body = ResolveRequest,
    responses(
        (status = 200, description = "解决成功", body = DataRecordResponse),
        (status = 404, description = "事件不存在"),
    )
)]
#[post("/data/events/resolve", data = "<req>")]
pub async fn resolve_event(
    pool: &State<PgPool>,
    _guard: ModuleGuard,
    req: Json<ResolveRequest>,
) -> AppResult<Json<DataRecordResponse>> {
    let service = DataService::new(pool);
    let result = service.resolve_event(&req.patient_id, &req.time, req.device_id.as_ref()).await?;
    Ok(Json(result.into()))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![report_data, query_data, query_alerts, acknowledge_event, resolve_event]
}

#[derive(OpenApi)]
#[openapi(paths(report_data, query_data, query_alerts, acknowledge_event, resolve_event))]
pub struct DataApiDoc;
