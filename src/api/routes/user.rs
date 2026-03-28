use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put, routes, Route};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::guards::{AdminUser, AuthenticatedUser};
use crate::application::user::UserAppService;
use crate::application::AppContext;
use crate::dto::request::{CreateUserRequest, UpdateUserRequest, UserQuery};
use crate::dto::response::{UserListResponse, UserResponse};
use crate::errors::{AppError, AppResult};

/// 创建用户（仅管理员）
#[utoipa::path(
    post,
    path = "/users",
    tag = "users",
    security(("bearer_auth" = [])),
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "创建成功", body = UserResponse),
        (status = 400, description = "验证失败"),
        (status = 409, description = "用户名已存在"),
    )
)]
#[post("/users", data = "<req>")]
pub async fn create_user(
    pool: &State<PgPool>,
    _admin: AdminUser,
    req: Json<CreateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    let ctx = AppContext::new(pool);
    let service = UserAppService::new(ctx);
    let response = service.create(req.into_inner()).await?;
    Ok(Json(response))
}

/// 获取用户
#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "用户ID")),
    responses(
        (status = 200, description = "获取成功", body = UserResponse),
        (status = 404, description = "用户不存在"),
    )
)]
#[get("/users/<id>")]
pub async fn get_user(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
) -> AppResult<Json<UserResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的用户 ID".into()))?;
    let ctx = AppContext::new(pool);
    let service = UserAppService::new(ctx);
    let response = service.get_by_id(id).await?;
    Ok(Json(response))
}

/// 更新用户
#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "用户ID")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "更新成功", body = UserResponse),
        (status = 404, description = "用户不存在"),
    )
)]
#[put("/users/<id>", data = "<req>")]
pub async fn update_user(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    id: &str,
    req: Json<UpdateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的用户 ID".into()))?;
    let ctx = AppContext::new(pool);
    let service = UserAppService::new(ctx);
    let response = service.update(id, req.into_inner()).await?;
    Ok(Json(response))
}

/// 删除用户（仅管理员）
#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "用户ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "用户不存在"),
    )
)]
#[delete("/users/<id>")]
pub async fn delete_user(
    pool: &State<PgPool>,
    _admin: AdminUser,
    id: &str,
) -> AppResult<Json<serde_json::Value>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的用户 ID".into()))?;
    let ctx = AppContext::new(pool);
    let service = UserAppService::new(ctx);
    service.delete(id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 查询用户列表
#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    security(("bearer_auth" = [])),
    params(
        ("role" = Option<String>, Query, description = "角色筛选"),
        ("status" = Option<String>, Query, description = "状态筛选"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = UserListResponse),
    )
)]
#[get("/users?<role>&<status>&<page>&<page_size>")]
pub async fn list_users(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    role: Option<String>,
    status: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<UserListResponse>> {
    let ctx = AppContext::new(pool);
    let service = UserAppService::new(ctx);
    let query = UserQuery {
        role,
        status,
        page,
        page_size,
    };
    let response = service.query(query).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<Route> {
    routes![create_user, get_user, update_user, delete_user, list_users]
}
