use rocket::{get, post};
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::api::guards::AuthenticatedUser;
use crate::config::JwtConfig;
use crate::dto::request::{
    ChangePasswordRequest, LoginRequest, RefreshTokenRequest, RegisterRequest, RevokeTokenRequest, VerifyTokenRequest,
};
use crate::dto::response::{LoginResponse, RefreshTokenResponse, RegisterResponse, RevokeResponse, SessionListResponse, UserInfo, VerifyTokenResponse};
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

/// 登出（撤销当前用户所有令牌）
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "登出成功"),
        (status = 401, description = "认证失败"),
    )
)]
#[post("/auth/logout")]
pub async fn logout(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    user: AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let service = AuthService::new(pool, jwt_config);
    // 撤销用户所有刷新令牌
    service.revoke(&user.id, RevokeTokenRequest { refresh_token: None }).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "message": "登出成功" }),
    ))
}

/// 获取当前用户信息
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "获取成功", body = UserInfo),
        (status = 401, description = "未认证"),
    )
)]
#[get("/auth/me")]
pub async fn get_me(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    user: AuthenticatedUser,
) -> AppResult<Json<UserInfo>> {
    let service = AuthService::new(pool, jwt_config);
    let response = service.get_me(&user.id).await?;
    Ok(Json(response))
}

/// 验证 Token 有效性
#[utoipa::path(
    post,
    path = "/auth/verify",
    tag = "auth",
    request_body = VerifyTokenRequest,
    responses(
        (status = 200, description = "验证结果", body = VerifyTokenResponse),
    )
)]
#[post("/auth/verify", data = "<req>")]
pub async fn verify_token(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    req: Json<VerifyTokenRequest>,
) -> AppResult<Json<VerifyTokenResponse>> {
    let service = AuthService::new(pool, jwt_config);
    let response = service.verify_token(req.into_inner()).await?;
    Ok(Json(response))
}

/// 用户注册
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "注册成功", body = RegisterResponse),
        (status = 400, description = "注册失败"),
        (status = 409, description = "用户名或邮箱已被使用"),
    )
)]
#[post("/auth/register", data = "<req>")]
pub async fn register(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    req: Json<RegisterRequest>,
) -> AppResult<Json<RegisterResponse>> {
    let service = AuthService::new(pool, jwt_config);
    let response = service.register(req.into_inner()).await?;
    Ok(Json(response))
}

/// 撤销令牌
#[utoipa::path(
    post,
    path = "/auth/revoke",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    request_body = RevokeTokenRequest,
    responses(
        (status = 200, description = "撤销成功", body = RevokeResponse),
    )
)]
#[post("/auth/revoke", data = "<req>")]
pub async fn revoke_token(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    user: AuthenticatedUser,
    req: Json<RevokeTokenRequest>,
) -> AppResult<Json<RevokeResponse>> {
    let service = AuthService::new(pool, jwt_config);
    let response = service.revoke(&user.id, req.into_inner()).await?;
    Ok(Json(response))
}

/// 获取用户会话列表
#[utoipa::path(
    get,
    path = "/auth/sessions",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "获取成功", body = SessionListResponse),
    )
)]
#[get("/auth/sessions")]
pub async fn list_sessions(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    user: AuthenticatedUser,
) -> AppResult<Json<SessionListResponse>> {
    let service = AuthService::new(pool, jwt_config);
    let response = service.list_sessions(&user.id).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![login, refresh_token, change_password, logout, get_me, verify_token, register, revoke_token, list_sessions]
}

#[derive(OpenApi)]
#[openapi(paths(login, refresh_token, change_password, logout, get_me, verify_token, register, revoke_token, list_sessions))]
pub struct AuthApiDoc;
