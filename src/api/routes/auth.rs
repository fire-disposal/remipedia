use rocket::serde::json::Json;
use rocket::State;
use rocket::post;
use sqlx::PgPool;

use crate::api::guards::AuthenticatedUser;
use crate::config::JwtConfig;
use crate::dto::request::{ChangePasswordRequest, LoginRequest, LogoutRequest, RefreshTokenRequest};
use crate::dto::response::{LoginResponse, RefreshTokenResponse};
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

/// 刷新令牌
#[post("/auth/refresh", data = "<req>")]
pub async fn refresh_token(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    req: Json<RefreshTokenRequest>,
) -> AppResult<Json<RefreshTokenResponse>> {
    let service = AuthService::new(pool, jwt_config);
    let response = service.refresh_token(req.into_inner()).await?;
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

/// 登出
#[post("/auth/logout", data = "<req>")]
pub async fn logout(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    _user: AuthenticatedUser,
    req: Json<LogoutRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let service = AuthService::new(pool, jwt_config);
    service.logout(&req.refresh_token).await?;
    Ok(Json(serde_json::json!({ "success": true, "message": "登出成功" })))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![login, refresh_token, change_password, logout]
}