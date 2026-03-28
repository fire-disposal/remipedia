use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use sqlx::PgPool;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::{DeviceQuery, RegisterDeviceRequest, UpdateDeviceRequest};
use crate::dto::response::BindingListResponse;
use crate::dto::response::DeviceListResponse;
use crate::dto::response::DeviceResponse;
use crate::errors::{AppError, AppResult};
use crate::service::{BindingService, DeviceService};

#[derive(rocket::serde::Serialize, utoipa::ToSchema)]
#[schema(example = json!({
    "total": 100,
    "active": 80,
    "inactive": 20,
    "online": 50,
    "offline": 50
}))]
pub struct DeviceStatsResponse {
    pub total: i64,
    pub active: i64,
    pub inactive: i64,
    pub online: i64,
    pub offline: i64,
}

/// 注册设备
#[utoipa::path(
    post,
    path = "/devices",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
    request_body = RegisterDeviceRequest,
    responses(
        (status = 200, description = "注册成功", body = DeviceResponse),
        (status = 400, description = "验证失败或设备已存在"),
    )
)]
#[post("/devices", data = "<req>")]
pub async fn register_device(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    req: Json<RegisterDeviceRequest>,
) -> AppResult<Json<DeviceResponse>> {
    let service = DeviceService::new(pool);
    let response = service.register(req.into_inner()).await?;
    Ok(Json(response))
}

/// 获取设备
#[utoipa::path(
    get,
    path = "/devices/{id}",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "设备ID")
    ),
    responses(
        (status = 200, description = "获取成功", body = DeviceResponse),
        (status = 404, description = "设备不存在"),
    )
)]
#[get("/devices/<id>")]
pub async fn get_device(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<DeviceResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let service = DeviceService::new(pool);
    let response = service.get_by_id(&id).await?;
    Ok(Json(response))
}

/// 更新设备
#[utoipa::path(
    put,
    path = "/devices/{id}",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "设备ID")
    ),
    request_body = UpdateDeviceRequest,
    responses(
        (status = 200, description = "更新成功", body = DeviceResponse),
        (status = 404, description = "设备不存在"),
    )
)]
#[put("/devices/<id>", data = "<req>")]
pub async fn update_device(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    req: Json<UpdateDeviceRequest>,
) -> AppResult<Json<DeviceResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let service = DeviceService::new(pool);
    let response = service.update(&id, req.into_inner()).await?;
    Ok(Json(response))
}

/// 删除设备
#[utoipa::path(
    delete,
    path = "/devices/{id}",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "设备ID")
    ),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "设备不存在"),
    )
)]
#[delete("/devices/<id>")]
pub async fn delete_device(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let service = DeviceService::new(pool);
    service.delete(&id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 查询设备列表
#[utoipa::path(
    get,
    path = "/devices",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("device_type" = Option<String>, Query, description = "设备类型筛选"),
        ("status" = Option<String>, Query, description = "状态筛选"),
        ("serial_number" = Option<String>, Query, description = "序列号筛选"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = DeviceListResponse),
    )
)]
#[get("/devices?<device_type>&<status>&<serial_number>&<page>&<page_size>")]
pub async fn list_devices(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    device_type: Option<String>,
    status: Option<String>,
    serial_number: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<DeviceListResponse>> {
    let service = DeviceService::new(pool);
    let query = DeviceQuery {
        device_type,
        status,
        serial_number,
        page,
        page_size,
    };
    let response = service.query(query).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        register_device,
        get_device,
        update_device,
        delete_device,
        list_devices,
        get_device_stats,
        get_device_bindings
    ]
}

/// 获取设备统计
#[utoipa::path(
    get,
    path = "/devices/stats",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "获取成功", body = DeviceStatsResponse),
    )
)]
#[get("/devices/stats")]
pub async fn get_device_stats(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
) -> AppResult<Json<DeviceStatsResponse>> {
    let service = DeviceService::new(pool);
    let stats = service.get_stats().await?;
    Ok(Json(stats))
}

/// 获取设备的绑定历史
#[utoipa::path(
    get,
    path = "/devices/{id}/bindings",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
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
    let service = BindingService::new(pool);
    let response = service.get_device_binding_history(
        &id,
        page.unwrap_or(1),
        page_size.unwrap_or(20),
    ).await?;
    Ok(Json(response))
}

#[derive(OpenApi)]
#[openapi(paths(
    register_device,
    get_device,
    update_device,
    delete_device,
    list_devices,
    get_device_stats,
    get_device_bindings
))]
pub struct DeviceApiDoc;
