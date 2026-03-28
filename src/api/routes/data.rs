use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post};
use sqlx::PgPool;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::DataReportRequest;
use crate::dto::response::{DataQueryResponse, DataReportResponse};
use crate::errors::{AppError, AppResult};
use crate::service::DataService;

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
        ("device_id" = Option<String>, Query, description = "设备ID筛选"),
        ("subject_id" = Option<String>, Query, description = "患者ID筛选"),
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
#[get("/data?<device_id>&<subject_id>&<data_type>&<start_time>&<end_time>&<page>&<page_size>")]
pub async fn query_data(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    device_id: Option<String>,
    subject_id: Option<String>,
    data_type: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<DataQueryResponse>> {
    let service = DataService::new(pool);

    let query = crate::dto::DataQuery {
        device_id: device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        subject_id: subject_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
        data_type,
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

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![report_data, query_data, delete_data]
}

/// 删除数据
#[utoipa::path(
    delete,
    path = "/data/{id}",
    tag = "data",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "数据ID")
    ),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "数据不存在"),
    )
)]
#[delete("/data/<id>")]
pub async fn delete_data(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的数据 ID".into()))?;
    let service = DataService::new(pool);
    service.delete(&id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(OpenApi)]
#[openapi(paths(report_data, query_data, delete_data))]
pub struct DataApiDoc;
