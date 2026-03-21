use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::{CreatePatientProfileRequest, CreatePatientRequest, PatientQuery, UpdatePatientRequest};
use crate::dto::response::{PatientDetailResponse, PatientListResponse, PatientProfileResponse, PatientResponse};
use crate::errors::{AppError, AppResult};
use crate::service::PatientService;

/// 创建患者
#[post("/patients", data = "<req>")]
pub async fn create_patient(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    req: Json<CreatePatientRequest>,
) -> AppResult<Json<PatientResponse>> {
    let service = PatientService::new(pool);
    let response = service.create(req.into_inner()).await?;
    Ok(Json(response))
}

/// 获取患者
#[get("/patients/<id>")]
pub async fn get_patient(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<PatientResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = PatientService::new(pool);
    let response = service.get_by_id(&id).await?;
    Ok(Json(response))
}

/// 获取患者详情（含档案）
#[get("/patients/<id>/detail")]
pub async fn get_patient_detail(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<PatientDetailResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = PatientService::new(pool);
    let response = service.get_detail(&id).await?;
    Ok(Json(response))
}

/// 更新患者
#[put("/patients/<id>", data = "<req>")]
pub async fn update_patient(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    req: Json<UpdatePatientRequest>,
) -> AppResult<Json<PatientResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = PatientService::new(pool);
    let response = service.update(&id, req.into_inner()).await?;
    Ok(Json(response))
}

/// 删除患者
#[delete("/patients/<id>")]
pub async fn delete_patient(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = PatientService::new(pool);
    service.delete(&id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 查询患者列表
#[get("/patients?<name>&<external_id>&<page>&<page_size>")]
pub async fn list_patients(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    name: Option<String>,
    external_id: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<PatientListResponse>> {
    let service = PatientService::new(pool);
    let query = PatientQuery {
        name,
        external_id,
        page,
        page_size,
    };
    let response = service.query(query).await?;
    Ok(Json(response))
}

// ========== 患者档案 ==========

/// 创建或更新患者档案
#[put("/patients/<id>/profile", data = "<req>")]
pub async fn upsert_patient_profile(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    req: Json<CreatePatientProfileRequest>,
) -> AppResult<Json<PatientProfileResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = PatientService::new(pool);
    let response = service.upsert_profile(&id, req.into_inner()).await?;
    Ok(Json(response))
}

/// 获取患者档案
#[get("/patients/<id>/profile")]
pub async fn get_patient_profile(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<Option<PatientProfileResponse>>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的患者 ID".into()))?;
    let service = PatientService::new(pool);
    let response = service.get_profile(&id).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        create_patient, get_patient, get_patient_detail, update_patient, delete_patient, list_patients,
        upsert_patient_profile, get_patient_profile
    ]
}