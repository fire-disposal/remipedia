use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use log::{info, warn};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Build, Rocket};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use remipedia::api::routes;
use remipedia::api::swagger_ui;
use remipedia::config::Settings;
use remipedia::ingest::modules::{mattress, vision, imu, ModuleRegistry};
use remipedia::repository::UserRepository;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

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

    async fn on_response<'r>(
        &self,
        request: &'r rocket::Request<'_>,
        response: &mut rocket::Response<'r>,
    ) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD",
        ));

        if let Some(request_headers) = request.headers().get_one("Access-Control-Request-Headers") {
            response.set_header(Header::new("Access-Control-Allow-Headers", request_headers));
        } else {
            response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        }

        response.set_header(Header::new("Access-Control-Max-Age", "86400"));
        response.set_header(Header::new("Access-Control-Expose-Headers", "*"));
    }
}

fn init_logging() {
    env_logger::Builder::from_env("RUST_LOG")
        .filter_module("remipedia", log::LevelFilter::Debug)
        .filter_module("sqlx", log::LevelFilter::Warn)
        .filter_module("rocket", log::LevelFilter::Info)
        .init();
}

fn hash_password(password: &str) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| anyhow::anyhow!("密码哈希失败: {}", e))
}

async fn init_admin(pool: &PgPool) -> anyhow::Result<()> {
    let user_repo = UserRepository::new(pool);

    if user_repo.exists_super_admin().await? {
        info!("✅ 超级管理员账户已存在，跳过初始化");
        return Ok(());
    }

    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string());
    let password_hash = hash_password(&admin_password)?;

    let admin = user_repo
        .create_super_admin(&admin_username, &password_hash)
        .await?;

    info!("🎉 初始超级管理员账户创建成功!");
    info!("   📧 用户名: {}", admin.username);

    if std::env::var("ADMIN_PASSWORD").is_err() {
        warn!("⚠️  使用了默认密码 'admin123'，请立即修改密码！");
    }

    Ok(())
}

async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("数据库迁移失败: {}", e))?;
    info!("🗃️ 数据库迁移完成");
    Ok(())
}

/// 创建 Rocket 应用
async fn build_rocket(
    settings: &Settings,
    pool: PgPool,
) -> Rocket<Build> {
    rocket::build()
        .manage(pool)
        .manage(settings.jwt.clone())
        .manage(settings.mqtt.clone())
        .attach(Cors)
        .mount("/", remipedia::api::routes::health::routes())
        .mount("/api/v1", routes())
        .mount("/", swagger_ui())
}

/// 初始化并启动所有Ingest模块
async fn init_ingest_modules(pool: &PgPool, settings: &Settings) -> anyhow::Result<()> {
    let mut registry = ModuleRegistry::new();

    // 注册床垫TCP模块
    registry.register(Box::new(mattress::MattressModule::new(
        mattress::MattressConfig {
            bind_addr: format!("0.0.0.0:{}", settings.tcp.port).parse()?,
            ..Default::default()
        }
    )));
    info!("📡 注册床垫TCP模块");

    // 注册视觉识别MQTT模块
    if settings.mqtt.enabled {
        registry.register(Box::new(vision::VisionModule::new(
            vision::VisionConfig {
                mqtt_broker: settings.mqtt.broker.clone(),
                mqtt_port: settings.mqtt.port,
                mqtt_topic: "device/vision/+/detect".to_string(),
                client_id: format!("remipedia_vision_{}", uuid::Uuid::new_v4()),
                ..Default::default()
            }
        )));
        info!("📡 注册视觉识别MQTT模块");

        // 注册IMU传感器MQTT模块
        registry.register(Box::new(imu::ImuModule::new(
            imu::ImuConfig {
                mqtt_broker: settings.mqtt.broker.clone(),
                mqtt_port: settings.mqtt.port,
                mqtt_topic: "device/imu/+/data".to_string(),
                client_id: format!("remipedia_imu_{}", uuid::Uuid::new_v4()),
                ..Default::default()
            }
        )));
        info!("📡 注册IMU传感器MQTT模块");
    }

    // 启动所有模块
    registry.start_all(pool).await
        .map_err(|e| anyhow::anyhow!("启动Ingest模块失败: {}", e))?;

    info!("✅ 所有Ingest模块已启动");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    info!("🚀 Remipedia IoT Health Platform 启动中...");

    let settings = Settings::new().map_err(|e| anyhow::anyhow!("配置加载失败: {}", e))?;
    info!("📋 配置加载成功");

    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .connect(&settings.database.url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {}", e))?;
    info!("🔌 数据库连接池创建成功");

    run_migrations(&pool).await?;
    init_admin(&pool).await?;

    // 启动Ingest模块
    init_ingest_modules(&pool, &settings).await?;

    // 启动 HTTP 服务器
    let rocket = build_rocket(&settings, pool).await;
    info!(
        "🌐 HTTP 服务器启动于 {}:{}",
        settings.server.host, settings.server.port
    );

    rocket
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!("服务器启动失败: {}", e))?;

    info!("👋 服务器已关闭");
    Ok(())
}
