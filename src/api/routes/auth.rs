use rocket::post;
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::api::guards::AuthenticatedUser;
use crate::config::JwtConfig;
use crate::dto::request::{
    ChangePasswordRequest, LoginRequest, LogoutRequest, RefreshTokenRequest,
};
use crate::dto::response::{LoginResponse, RefreshTokenResponse};
use crate::errors::AppResult;
use crate::service::AuthService;

/// 用户登录
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "用户名或密码错误"),
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "刷新成功", body = RefreshTokenResponse),
        (status = 401, description = "无效的刷新令牌"),
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/change-password",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "密码修改成功"),
        (status = 401, description = "认证失败或旧密码错误"),
    )
)]
#[post("/auth/change-password", data = "<req>")]
pub async fn change_password(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    user: AuthenticatedUser,
    req: Json<ChangePasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let service = AuthService::new(pool, jwt_config);
    service.change_password(&user.id, req.into_inner()).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "message": "密码修改成功" }),
    ))
}

/// 登出
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "登出成功"),
        (status = 401, description = "认证失败"),
    )
)]
#[post("/auth/logout", data = "<req>")]
pub async fn logout(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    _user: AuthenticatedUser,
    req: Json<LogoutRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let service = AuthService::new(pool, jwt_config);
    service.logout(&req.refresh_token).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "message": "登出成功" }),
    ))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![login, refresh_token, change_password, logout]
}

#[derive(OpenApi)]
#[openapi(paths(login, refresh_token, change_password, logout))]
pub struct AuthApiDoc;
