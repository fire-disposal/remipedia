use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::DataReportRequest;
use crate::dto::response::{DataQueryResponse, DataRecordResponse, DataReportResponse};
use crate::errors::{AppError, AppResult};
use crate::service::DataService;

/// 数据上报
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
        start_time: start_time.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
        end_time: end_time.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    };
    
    let response = service.query(query).await?;
    Ok(Json(response))
}

/// 获取设备最新数据
#[get("/data/latest/<device_id>?<data_type>")]
pub async fn get_latest_data(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    device_id: &str,
    data_type: Option<String>,
) -> AppResult<Json<Option<DataRecordResponse>>> {
    let device_id = Uuid::parse_str(device_id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let service = DataService::new(pool);
    let response = service.get_latest(&device_id, data_type.as_deref()).await?;
    Ok(Json(response))
}

/// 获取患者最新数据
#[get("/data/latest/subject/<subject_id>?<data_type>&<limit>")]
pub async fn get_latest_data_by_subject(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    subject_id: &str,
    data_type: Option<String>,
    limit: Option<i64>,
) -> AppResult<Json<Vec<DataRecordResponse>>> {
    let subject_id = Uuid::parse_str(subject_id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = DataService::new(pool);
    let response = service.get_latest_by_subject(&subject_id, data_type.as_deref(), limit.unwrap_or(10)).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![report_data, query_data, get_latest_data, get_latest_data_by_subject]
}