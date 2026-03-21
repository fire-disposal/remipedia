use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::{DeviceQuery, RegisterDeviceRequest, UpdateDeviceRequest};
use crate::dto::response::DeviceListResponse;
use crate::dto::response::DeviceResponse;
use crate::errors::{AppError, AppResult};
use crate::service::DeviceService;

/// 注册设备
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
    rocket::routes![register_device, get_device, update_device, delete_device, list_devices]
}