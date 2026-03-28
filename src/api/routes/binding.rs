use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post};
use sqlx::PgPool;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::CreateBindingRequest;
use crate::dto::response::BindingListResponse;
use crate::dto::response::BindingResponse;
use crate::errors::{AppError, AppResult};
use crate::service::BindingService;

/// 获取单个绑定
#[utoipa::path(
    get,
    path = "/bindings/{id}",
    tag = "bindings",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "绑定ID")
    ),
    responses(
        (status = 200, description = "获取成功", body = BindingResponse),
        (status = 404, description = "绑定不存在"),
    )
)]
#[get("/bindings/<id>")]
pub async fn get_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<BindingResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的绑定 ID".into()))?;
    let service = BindingService::new(pool);
    let response = service.get_by_id(&id).await?;
    Ok(Json(response))
}

/// 查询绑定列表
#[utoipa::path(
    get,
    path = "/bindings",
    tag = "bindings",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("device_id" = Option<String>, Query, description = "设备ID筛选"),
        ("patient_id" = Option<String>, Query, description = "患者ID筛选"),
        ("active_only" = Option<bool>, Query, description = "仅显示有效绑定"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = BindingListResponse),
    )
)]
#[get("/bindings?<device_id>&<patient_id>&<active_only>&<page>&<page_size>")]
pub async fn list_bindings(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    device_id: Option<String>,
    patient_id: Option<String>,
    active_only: Option<bool>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<BindingListResponse>> {
    let device_id = device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    let patient_id = patient_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());

    let service = BindingService::new(pool);
    let response = service
        .query(
            device_id,
            patient_id,
            active_only.unwrap_or(false),
            page.unwrap_or(1),
            page_size.unwrap_or(20),
        )
        .await?;
    Ok(Json(response))
}

/// 创建绑定
#[utoipa::path(
    post,
    path = "/bindings",
    tag = "bindings",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateBindingRequest,
    responses(
        (status = 200, description = "绑定成功", body = BindingResponse),
        (status = 400, description = "设备已有有效绑定"),
        (status = 404, description = "设备或患者不存在"),
    )
)]
#[post("/bindings", data = "<req>")]
pub async fn create_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    req: Json<CreateBindingRequest>,
) -> AppResult<Json<BindingResponse>> {
    let service = BindingService::new(pool);
    let response = service.bind(req.into_inner()).await?;
    Ok(Json(response))
}

/// 解除绑定
#[utoipa::path(
    delete,
    path = "/bindings/{id}",
    tag = "bindings",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "绑定ID")
    ),
    responses(
        (status = 200, description = "解除成功"),
        (status = 404, description = "绑定不存在"),
    )
)]
#[delete("/bindings/<id>")]
pub async fn delete_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的绑定 ID".into()))?;
    let service = BindingService::new(pool);
    service.unbind(&id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![list_bindings, create_binding, delete_binding, get_binding]
}

#[derive(OpenApi)]
#[openapi(paths(list_bindings, create_binding, delete_binding, get_binding))]
pub struct BindingApiDoc;
