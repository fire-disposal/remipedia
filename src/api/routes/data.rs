use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;
use serde::Deserialize;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::DataReportRequest;
use crate::dto::response::{
    AlertEventResponse, AlertStatsResponse, DataQueryResponse, DataReportResponse,
    ObservationResponse,
};
use crate::errors::AppResult;
use crate::service::DataService;

/// 确认/解决事件请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct EventActionRequest {
    /// 告警事件ID
    pub event_id: Uuid,
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
    _user: AuthenticatedUser,
    req: Json<DataReportRequest>,
) -> AppResult<Json<DataReportResponse>> {
    let service = DataService::new(pool.inner().clone());
    let response = service.report_http(req.into_inner()).await?;
    Ok(Json(response))
}

/// 查询观测数据
#[utoipa::path(
    get,
    path = "/data",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("patient_id" = Option<String>, Query, description = "患者ID筛选"),
        ("data_type" = Option<String>, Query, description = "数据类型筛选"),
        ("start_time" = Option<String>, Query, description = "开始时间 (RFC3339)"),
        ("end_time" = Option<String>, Query, description = "结束时间 (RFC3339)"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = DataQueryResponse),
    )
)]
#[get("/data?<patient_id>&<data_type>&<start_time>&<end_time>&<page>&<page_size>")]
pub async fn query_data(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    patient_id: Option<String>,
    data_type: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<DataQueryResponse>> {
    let service = DataService::new(pool.inner().clone());

    let query = crate::dto::request::DataQuery {
        patient_id: patient_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        device_id: None,
        data_type,
        data_category: None,
        severity: None,
        status: None,
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

/// 查询活跃告警事件
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
        ("severity" = Option<String>, Query, description = "严重级别 (info/warning/alert/critical)"),
        ("limit" = Option<i64>, Query, description = "返回数量限制"),
    ),
    responses(
        (status = 200, description = "查询成功", body = Vec<AlertEventResponse>),
    )
)]
#[get("/data/alerts?<patient_id>&<data_type>&<severity>&<limit>")]
pub async fn query_alerts(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    patient_id: Option<String>,
    data_type: Option<String>,
    severity: Option<String>,
    limit: Option<i64>,
) -> AppResult<Json<Vec<AlertEventResponse>>> {
    let service = DataService::new(pool.inner().clone());

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
    Ok(Json(response))
}

/// 获取告警统计
#[utoipa::path(
    get,
    path = "/data/alerts/stats",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("patient_id" = Option<String>, Query, description = "患者ID筛选"),
    ),
    responses(
        (status = 200, description = "统计成功", body = AlertStatsResponse),
    )
)]
#[get("/data/alerts/stats?<patient_id>")]
pub async fn get_alert_stats(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    patient_id: Option<String>,
) -> AppResult<Json<AlertStatsResponse>> {
    let service = DataService::new(pool.inner().clone());
    let pid = patient_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    let response = service.get_alert_stats(pid.as_ref()).await?;
    Ok(Json(response))
}

/// 确认告警事件
#[utoipa::path(
    post,
    path = "/data/events/acknowledge",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    request_body = EventActionRequest,
    responses(
        (status = 200, description = "确认成功", body = AlertEventResponse),
        (status = 404, description = "事件不存在"),
    )
)]
#[post("/data/events/acknowledge", data = "<req>")]
pub async fn acknowledge_event(
    pool: &State<PgPool>,
    user: AuthenticatedUser,
    req: Json<EventActionRequest>,
) -> AppResult<Json<AlertEventResponse>> {
    let service = DataService::new(pool.inner().clone());
    let result = service
        .acknowledge_event(&req.event_id, &user.id)
        .await?;
    Ok(Json(result))
}

/// 解决告警事件
#[utoipa::path(
    post,
    path = "/data/events/resolve",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    request_body = EventActionRequest,
    responses(
        (status = 200, description = "解决成功", body = AlertEventResponse),
        (status = 404, description = "事件不存在"),
    )
)]
#[post("/data/events/resolve", data = "<req>")]
pub async fn resolve_event(
    pool: &State<PgPool>,
    user: AuthenticatedUser,
    req: Json<EventActionRequest>,
) -> AppResult<Json<AlertEventResponse>> {
    let service = DataService::new(pool.inner().clone());
    let result = service.resolve_event(&req.event_id, &user.id).await?;
    Ok(Json(result))
}

/// 获取患者最新观测数据
#[utoipa::path(
    get,
    path = "/data/latest",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("patient_id" = String, Query, description = "患者ID"),
        ("stream_type" = Option<String>, Query, description = "数据流类型 (metric/event)"),
        ("limit" = Option<i64>, Query, description = "返回数量限制"),
    ),
    responses(
        (status = 200, description = "查询成功", body = Vec<ObservationResponse>),
    )
)]
#[get("/data/latest?<patient_id>&<stream_type>&<limit>")]
pub async fn get_latest_data(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    patient_id: String,
    stream_type: Option<String>,
    limit: Option<i64>,
) -> AppResult<Json<Vec<ObservationResponse>>> {
    let service = DataService::new(pool.inner().clone());
    let pid = Uuid::parse_str(&patient_id).map_err(|e| {
        crate::errors::AppError::validation(format!("无效患者ID: {}", e))
    })?;
    let response = service
        .get_latest_by_patient(&pid, stream_type.as_deref(), limit.unwrap_or(10))
        .await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        report_data,
        query_data,
        query_alerts,
        get_alert_stats,
        acknowledge_event,
        resolve_event,
        get_latest_data,
    ]
}

#[derive(utoipa::OpenApi)]
#[openapi(paths(
    report_data,
    query_data,
    query_alerts,
    get_alert_stats,
    acknowledge_event,
    resolve_event,
    get_latest_data,
))]
pub struct DataApiDoc;
