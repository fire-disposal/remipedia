use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::guards::AuthenticatedUser;
use crate::dto::request::{CreateUserRequest, UpdateUserRequest, UserQuery};
use crate::dto::response::UserListResponse;
use crate::dto::response::UserResponse;
use crate::errors::{AppError, AppResult};
use crate::service::UserService;

/// 创建用户（管理员）
#[post("/users", data = "<req>")]
pub async fn create_user(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    req: Json<CreateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    let service = UserService::new(pool);
    let response = service.create(req.into_inner()).await?;
    Ok(Json(response))
}

/// 获取用户
#[get("/users/<id>")]
pub async fn get_user(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<UserResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的用户 ID".into()))?;
    let service = UserService::new(pool);
    let response = service.get_by_id(&id).await?;
    Ok(Json(response))
}

/// 更新用户
#[put("/users/<id>", data = "<req>")]
pub async fn update_user(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    req: Json<UpdateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的用户 ID".into()))?;
    let service = UserService::new(pool);
    let response = service.update(&id, req.into_inner()).await?;
    Ok(Json(response))
}

/// 删除用户（管理员）
#[delete("/users/<id>")]
pub async fn delete_user(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的用户 ID".into()))?;
    let service = UserService::new(pool);
    service.delete(&id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 查询用户列表
#[get("/users?<role>&<status>&<page>&<page_size>")]
pub async fn list_users(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    role: Option<String>,
    status: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<UserListResponse>> {
    let service = UserService::new(pool);
    let query = UserQuery {
        role,
        status,
        page,
        page_size,
    };
    let response = service.query(query).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![create_user, get_user, update_user, delete_user, list_users]
}