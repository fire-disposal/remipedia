use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use sqlx::PgPool;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::{CreatePatientRequest, PatientQuery, UpdatePatientRequest};
use crate::dto::response::{PatientDetailResponse, PatientListResponse, PatientResponse};
use crate::errors::{AppError, AppResult};
use crate::service::PatientService;

/// 创建患者
#[utoipa::path(
    post,
    path = "/patients",
    tag = "patients",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreatePatientRequest,
    responses(
        (status = 200, description = "创建成功", body = PatientResponse),
        (status = 400, description = "验证失败"),
    )
)]
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
#[utoipa::path(
    get,
    path = "/patients/{id}",
    tag = "patients",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "患者ID")
    ),
    responses(
        (status = 200, description = "获取成功", body = PatientResponse),
        (status = 404, description = "患者不存在"),
    )
)]
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
#[utoipa::path(
    get,
    path = "/patients/{id}/detail",
    tag = "patients",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "患者ID")
    ),
    responses(
        (status = 200, description = "获取成功", body = PatientDetailResponse),
        (status = 404, description = "患者不存在"),
    )
)]
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
#[utoipa::path(
    put,
    path = "/patients/{id}",
    tag = "patients",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "患者ID")
    ),
    request_body = UpdatePatientRequest,
    responses(
        (status = 200, description = "更新成功", body = PatientResponse),
        (status = 404, description = "患者不存在"),
    )
)]
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
#[utoipa::path(
    delete,
    path = "/patients/{id}",
    tag = "patients",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "患者ID")
    ),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "患者不存在"),
    )
)]
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
#[utoipa::path(
    get,
    path = "/patients",
    tag = "patients",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("name" = Option<String>, Query, description = "姓名筛选"),
        ("external_id" = Option<String>, Query, description = "外部ID筛选"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = PatientListResponse),
    )
)]
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

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        create_patient, get_patient, get_patient_detail, update_patient, delete_patient, list_patients
    ]
}

#[derive(OpenApi)]
#[openapi(paths(create_patient, get_patient, get_patient_detail, update_patient, delete_patient, list_patients))]
pub struct PatientApiDoc;