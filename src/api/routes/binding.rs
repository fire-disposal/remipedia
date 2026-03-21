use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::CreateBindingRequest;
use crate::dto::response::BindingListResponse;
use crate::dto::response::BindingResponse;
use crate::errors::{AppError, AppResult};
use crate::service::BindingService;

/// 查询绑定列表
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
    let response = service.query(
        device_id,
        patient_id,
        active_only.unwrap_or(false),
        page.unwrap_or(1),
        page_size.unwrap_or(20),
    ).await?;
    Ok(Json(response))
}

/// 创建绑定
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

/// 获取设备的当前绑定
#[get("/bindings/device/<device_id>")]
pub async fn get_device_binding(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    device_id: &str,
) -> AppResult<Json<Option<BindingResponse>>> {
    let device_id = Uuid::parse_str(device_id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let service = BindingService::new(pool);
    let response = service.get_current_binding(&device_id).await?;
    Ok(Json(response))
}

/// 获取设备的绑定历史
#[get("/bindings/device/<device_id>/history?<page>&<page_size>")]
pub async fn get_device_binding_history(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    device_id: &str,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<BindingListResponse>> {
    let device_id = Uuid::parse_str(device_id).map_err(|_| AppError::ValidationError("无效的设备 ID".into()))?;
    let service = BindingService::new(pool);
    let response = service.get_device_binding_history(
        &device_id,
        page.unwrap_or(1),
        page_size.unwrap_or(20),
    ).await?;
    Ok(Json(response))
}

/// 获取患者的绑定历史
#[get("/bindings/patient/<patient_id>/history?<page>&<page_size>")]
pub async fn get_patient_binding_history(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    patient_id: &str,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<BindingListResponse>> {
    let patient_id = Uuid::parse_str(patient_id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = BindingService::new(pool);
    let response = service.get_patient_binding_history(
        &patient_id,
        page.unwrap_or(1),
        page_size.unwrap_or(20),
    ).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list_bindings, create_binding, delete_binding, get_device_binding,
        get_device_binding_history, get_patient_binding_history
    ]
}