use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, routes, Route};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::application::binding::BindingAppService;
use crate::application::AppContext;
use crate::dto::request::CreateBindingRequest;
use crate::dto::response::{BindingListResponse, BindingResponse};
use crate::errors::{AppError, AppResult};

/// 创建绑定
#[utoipa::path(
    post,
    path = "/bindings",
    tag = "bindings",
    security(("bearer_auth" = [])),
    request_body = CreateBindingRequest,
    responses(
        (status = 200, description = "绑定成功", body = BindingResponse),
        (status = 400, description = "设备未激活或已绑定"),
    )
)]
#[post("/bindings", data = "<req>")]
pub async fn create_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    req: Json<CreateBindingRequest>,
) -> AppResult<Json<BindingResponse>> {
    let ctx = AppContext::new(pool);
    let service = BindingAppService::new(ctx);
    let response = service.create(req.into_inner()).await?;
    Ok(Json(response))
}

/// 获取绑定
#[utoipa::path(
    get,
    path = "/bindings/{id}",
    tag = "bindings",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "绑定ID")),
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
    let ctx = AppContext::new(pool);
    let service = BindingAppService::new(ctx);
    let response = service.get_by_id(id).await?;
    Ok(Json(response))
}

/// 结束绑定（解绑）
#[utoipa::path(
    delete,
    path = "/bindings/{id}",
    tag = "bindings",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "绑定ID")),
    responses(
        (status = 200, description = "解绑成功"),
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
    let ctx = AppContext::new(pool);
    let service = BindingAppService::new(ctx);
    service.end_binding(id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 获取设备的绑定历史
#[utoipa::path(
    get,
    path = "/devices/{id}/bindings",
    tag = "bindings",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "设备ID"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "获取成功", body = BindingListResponse),
    )
)]
#[get("/devices/<id>/bindings?<page>&<page_size>")]
pub async fn get_device_bindings(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<BindingListResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let ctx = AppContext::new(pool);
    let service = BindingAppService::new(ctx);
    let response = service.get_device_bindings(id, page.unwrap_or(1), page_size.unwrap_or(20)).await?;
    Ok(Json(response))
}

/// 获取患者的绑定历史
#[utoipa::path(
    get,
    path = "/patients/{id}/bindings",
    tag = "bindings",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "患者ID"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "获取成功", body = BindingListResponse),
    )
)]
#[get("/patients/<id>/bindings?<page>&<page_size>")]
pub async fn get_patient_bindings(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<BindingListResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let ctx = AppContext::new(pool);
    let service = BindingAppService::new(ctx);
    let response = service.get_patient_bindings(id, page.unwrap_or(1), page_size.unwrap_or(20)).await?;
    Ok(Json(response))
}

/// 获取设备当前有效绑定
#[utoipa::path(
    get,
    path = "/devices/{id}/active-binding",
    tag = "bindings",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "设备ID")),
    responses(
        (status = 200, description = "获取成功", body = BindingResponse),
        (status = 404, description = "无有效绑定"),
    )
)]
#[get("/devices/<id>/active-binding")]
pub async fn get_active_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<Option<BindingResponse>>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let ctx = AppContext::new(pool);
    let service = BindingAppService::new(ctx);
    let response = service.get_active_by_device(id).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<Route> {
    routes![
        create_binding,
        get_binding,
        delete_binding,
        get_device_bindings,
        get_patient_bindings,
        get_active_binding,
    ]
}
