use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::State;
use rocket::{delete, get, post};
use sqlx::PgPool;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::{CreateBindingRequest, EndBindingRequest, SwitchBindingRequest};
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
        (status = 201, description = "绑定成功", body = BindingResponse),
        (status = 400, description = "设备已有有效绑定"),
        (status = 404, description = "设备或患者不存在"),
    )
)]
#[post("/bindings", data = "<req>")]
pub async fn create_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    req: Json<CreateBindingRequest>,
) -> AppResult<(Status, Json<BindingResponse>)> {
    let service = BindingService::new(pool);
    let response = service.bind(req.into_inner()).await?;
    Ok((Status::Created, Json(response)))
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
        (status = 204, description = "解除成功"),
        (status = 404, description = "绑定不存在"),
    )
)]
#[delete("/bindings/<id>")]
pub async fn delete_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Status> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的绑定 ID".into()))?;
    let service = BindingService::new(pool);
    service.unbind(&id).await?;
    Ok(Status::NoContent)
}

/// 结束绑定（显式结束）
#[utoipa::path(
    post,
    path = "/bindings/{id}/end",
    tag = "bindings",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "绑定ID")
    ),
    request_body = EndBindingRequest,
    responses(
        (status = 200, description = "结束成功", body = BindingResponse),
        (status = 404, description = "绑定不存在或已结束"),
    )
)]
#[post("/bindings/<id>/end", data = "<req>")]
pub async fn end_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    req: Json<EndBindingRequest>,
) -> AppResult<Json<BindingResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的绑定 ID".into()))?;
    let service = BindingService::new(pool);
    let response = service.end_binding(&id, req.into_inner()).await?;
    Ok(Json(response))
}

/// 切换绑定（强制换绑）
#[utoipa::path(
    post,
    path = "/bindings/switch",
    tag = "bindings",
    security(
        ("bearer_auth" = [])
    ),
    request_body = SwitchBindingRequest,
    responses(
        (status = 201, description = "切换成功", body = BindingResponse),
        (status = 400, description = "设备已有有效绑定"),
        (status = 404, description = "设备或患者不存在"),
    )
)]
#[post("/bindings/switch", data = "<req>")]
pub async fn switch_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    req: Json<SwitchBindingRequest>,
) -> AppResult<(Status, Json<BindingResponse>)> {
    let service = BindingService::new(pool);
    let response = service.switch_binding(req.into_inner()).await?;
    Ok((Status::Created, Json(response)))
}

/// 查询绑定历史
#[utoipa::path(
    get,
    path = "/bindings/history",
    tag = "bindings",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("device_id" = Option<String>, Query, description = "设备ID筛选"),
        ("patient_id" = Option<String>, Query, description = "患者ID筛选"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = BindingListResponse),
    )
)]
#[get("/bindings/history?<device_id>&<patient_id>&<page>&<page_size>")]
pub async fn list_binding_history(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    device_id: Option<String>,
    patient_id: Option<String>,
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
            false, // 返回所有历史（包括已结束的）
            page.unwrap_or(1),
            page_size.unwrap_or(20),
        )
        .await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list_bindings,
        create_binding,
        delete_binding,
        get_binding,
        end_binding,
        switch_binding,
        list_binding_history
    ]
}

#[derive(OpenApi)]
#[openapi(paths(list_bindings, create_binding, delete_binding, get_binding, end_binding, switch_binding, list_binding_history))]
pub struct BindingApiDoc;
