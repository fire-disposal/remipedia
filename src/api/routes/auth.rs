use rocket::serde::json::Json;
use rocket::State;
use rocket::post;
use sqlx::PgPool;

use crate::api::guards::AuthenticatedUser;
use crate::config::JwtConfig;
use crate::dto::request::{ChangePasswordRequest, LoginRequest};
use crate::dto::response::LoginResponse;
use crate::errors::AppResult;
use crate::service::AuthService;

/// 用户登录
#[post("/auth/login", data = "<req>")]
pub async fn login(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    req: Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let service = AuthService::new(pool, jwt_config);
    let response = service.login(req.into_inner()).await?;
    Ok(Json(response))
}

/// 修改密码
#[post("/auth/change-password", data = "<req>")]
pub async fn change_password(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    user: AuthenticatedUser,
    req: Json<ChangePasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let service = AuthService::new(pool, jwt_config);
    service.change_password(&user.id, req.into_inner()).await?;
    Ok(Json(serde_json::json!({ "success": true, "message": "密码修改成功" })))
}

/// 登出（客户端清除 token 即可）
#[post("/auth/logout")]
pub async fn logout() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "success": true, "message": "登出成功" }))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![login, change_password, logout]
}