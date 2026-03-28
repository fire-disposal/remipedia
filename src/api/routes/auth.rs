use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post, routes, Route};
use sqlx::PgPool;

use crate::api::guards::AuthenticatedUser;
use crate::application::auth::AuthAppService;
use crate::config::JwtConfig;
use crate::errors::AppResult;

/// 登录请求
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
}

/// 刷新令牌请求
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// 用户登录
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "认证失败"),
    )
)]
#[post("/auth/login", data = "<req>")]
pub async fn login(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    req: Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let service = AuthAppService::new(pool, jwt_config);
    let (access_token, refresh_token) = service.login(&req.username, &req.password).await?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
    }))
}

/// 刷新令牌
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "刷新成功", body = LoginResponse),
        (status = 401, description = "无效的刷新令牌"),
    )
)]
#[post("/auth/refresh", data = "<req>")]
pub async fn refresh_token(
    pool: &State<PgPool>,
    jwt_config: &State<JwtConfig>,
    req: Json<RefreshTokenRequest>,
) -> AppResult<Json<LoginResponse>> {
    let service = AuthAppService::new(pool, jwt_config);
    let (access_token, refresh_token) = service.refresh_token(&req.refresh_token).await?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
    }))
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
        (status = 200, description = "获取成功"),
        (status = 401, description = "未认证"),
    )
)]
#[get("/auth/me")]
pub async fn me(user: AuthenticatedUser) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "id": user.id.to_string(),
        "role": user.role.to_string(),
    })))
}

pub fn routes() -> Vec<Route> {
    routes![login, refresh_token, me]
}
