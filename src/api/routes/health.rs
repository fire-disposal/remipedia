use rocket::get;
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;

use crate::errors::AppResult;

/// Favicon SVG
#[get("/favicon.svg")]
pub async fn favicon_svg() -> Option<(rocket::http::ContentType, &'static str)> {
    const SVG: &str = include_str!("../../../static/favicon.svg");
    Some((rocket::http::ContentType::SVG, SVG))
}

/// Favicon ICO (浏览器默认请求)
#[get("/favicon.ico")]
pub async fn favicon_ico() -> Option<(rocket::http::ContentType, &'static str)> {
    // 返回 SVG，现代浏览器支持
    const SVG: &str = include_str!("../../../static/favicon.svg");
    Some((rocket::http::ContentType::SVG, SVG))
}

/// 根路径 - API 信息
#[get("/")]
pub async fn index() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "Remipedia IoT Health Platform",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "swagger_ui": "/swagger-ui/",
        "openapi_json": "/api-docs/openapi.json"
    }))
}

/// 健康检查
#[get("/health")]
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "remipedia",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// 就绪检查（含数据库连接）
#[get("/ready")]
pub async fn ready(pool: &State<PgPool>) -> AppResult<Json<serde_json::Value>> {
    // 检查数据库连接
    let db_ok: bool = sqlx::query("SELECT 1")
        .fetch_one(pool.inner())
        .await
        .is_ok();

    if db_ok {
        Ok(Json(serde_json::json!({
            "status": "ready",
            "checks": {
                "database": "ok"
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    } else {
        Ok(Json(serde_json::json!({
            "status": "not_ready",
            "checks": {
                "database": "failed"
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    }
}

/// 存活检查
#[get("/live")]
pub async fn live() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![favicon_svg, favicon_ico, index, health, ready, live]
}
