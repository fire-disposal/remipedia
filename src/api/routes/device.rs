use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use sqlx::PgPool;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::application::device::DeviceAppService;
use crate::application::AppContext;
use crate::dto::request::{DeviceQuery, RegisterDeviceRequest, UpdateDeviceRequest};
use crate::dto::response::DeviceListResponse;
use crate::dto::response::DeviceResponse;
use crate::errors::{AppError, AppResult};

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
    let ctx = AppContext::new(pool);
    let service = DeviceAppService::new(ctx);
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
    let ctx = AppContext::new(pool);
    let service = DeviceAppService::new(ctx);
    let response = service.get_by_id(id).await?;
    Ok(Json(response))
}

/// 更新设备状态
#[utoipa::path(
    put,
    path = "/devices/{id}/status",
    tag = "devices",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "设备ID")
    ),
    responses(
        (status = 200, description = "更新成功", body = DeviceResponse),
        (status = 404, description = "设备不存在"),
    )
)]
#[put("/devices/<id>/status", data = "<req>")]
pub async fn update_device_status(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    req: Json<UpdateDeviceRequest>,
) -> AppResult<Json<DeviceResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let ctx = AppContext::new(pool);
    let service = DeviceAppService::new(ctx);
    
    let status = req.status.as_deref().unwrap_or("active");
    let response = service.update_status(id, status).await?;
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
    let ctx = AppContext::new(pool);
    let service = DeviceAppService::new(ctx);
    service.delete(id).await?;
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
    let ctx = AppContext::new(pool);
    let service = DeviceAppService::new(ctx);
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
        update_device_status,
        delete_device,
        list_devices,
    ]
}

#[derive(OpenApi)]
#[openapi(paths(
    register_device,
    get_device,
    update_device_status,
    delete_device,
    list_devices,
))]
pub struct DeviceApiDoc;
