use std::sync::Arc;

use log::info;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Build, Rocket};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use remipedia::api::routes;
use remipedia::config::Settings;
use remipedia::ingest::MqttIngest;

/// CORS Fairing
pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r rocket::Request<'_>, response: &mut rocket::Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "Authorization, Content-Type"));
        response.set_header(Header::new("Access-Control-Max-Age", "86400"));
    }
}

/// 初始化日志
fn init_logging() {
    env_logger::Builder::from_env("RUST_LOG")
        .filter_module("remipedia", log::LevelFilter::Debug)
        .filter_module("sqlx", log::LevelFilter::Warn)
        .filter_module("rocket", log::LevelFilter::Info)
        .init();
}

/// 创建 Rocket 应用
async fn build_rocket(settings: &Settings, pool: PgPool) -> Rocket<Build> {
    rocket::build()
        .manage(pool)
        .manage(settings.jwt.clone())
        .manage(settings.mqtt.clone())
        .attach(Cors)
        .mount("/", remipedia::api::routes::health::routes())
        .mount("/api/v1", routes())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    init_logging();

    info!("🚀 Remipedia IoT Health Platform 启动中...");

    // 加载配置
    let settings = Settings::new()
        .map_err(|e| anyhow::anyhow!("配置加载失败: {}", e))?;

    info!("📋 配置加载成功");

    // 创建数据库连接池
    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .connect(&settings.database.url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {}", e))?;

    info!("🔌 数据库连接池创建成功");

    // 启动 MQTT 客户端（如果启用）
    if settings.mqtt.enabled {
        let mqtt_pool = Arc::new(pool.clone());
        let mqtt_config = settings.mqtt.clone();

        tokio::spawn(async move {
            info!("📡 MQTT 客户端启动中...");
            let mqtt_client = MqttIngest::new(mqtt_pool, &mqtt_config).await;
            mqtt_client.subscribe().await;
        });
    }

    // 启动 HTTP 服务器
    let rocket = build_rocket(&settings, pool).await;

    info!("🌐 HTTP 服务器启动于 {}:{}", settings.server.host, settings.server.port);

    rocket
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!("服务器启动失败: {}", e))?;

    info!("👋 服务器已关闭");

    Ok(())
}
