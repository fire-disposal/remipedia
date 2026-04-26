use log::info;
use rocket::{Build, Rocket};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use remipedia::api::fairings::{Cors, RequestLogger};
use remipedia::api::routes;
use remipedia::api::swagger_ui;
use remipedia::config::Settings;
use remipedia::errors::{AppError, AppResult};
use remipedia::ingest::modules::{imu, mattress, vision, ModuleRegistry};
use remipedia::repository::UserRepository;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .with_env_var("RUST_LOG")
                .from_env_lossy(),
        )
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();
}

async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    MIGRATOR.run(pool).await?;
    info!("🗃️ 数据库迁移完成");
    Ok(())
}

fn hash_password(password: &str) -> AppResult<String> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::internal(format!("密码哈希失败: {}", e)))
}

async fn init_admin(pool: &PgPool) -> AppResult<()> {
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
        log::warn!("⚠️  使用了默认密码 'admin123'，请立即修改密码！");
    }

    Ok(())
}

/// 初始化并启动所有 Ingest 模块，返回 ModuleRegistry
async fn init_ingest_modules(pool: &PgPool, settings: &Settings) -> AppResult<ModuleRegistry> {
    let mut registry = ModuleRegistry::new();

    // 注册床垫 TCP 模块
    registry.register(Box::new(mattress::MattressModule::new(
        mattress::MattressConfig {
            bind_addr: format!("0.0.0.0:{}", settings.tcp.port)
                .parse()
                .map_err(|e| AppError::validation(format!("无效的绑定地址: {}", e)))?,
            ..Default::default()
        },
    )));
    info!("📡 注册床垫TCP模块");

    // 注册 MQTT 模块
    if settings.mqtt.enabled {
        registry.register(Box::new(vision::VisionModule::new(
            vision::VisionConfig {
                mqtt_broker: settings.mqtt.broker.clone(),
                mqtt_port: settings.mqtt.port,
                mqtt_topic: "device/vision/+/detect".to_string(),
                client_id: format!("remipedia_vision_{}", uuid::Uuid::new_v4()),
                ..Default::default()
            },
        )));
        info!("📡 注册视觉识别MQTT模块");

        registry.register(Box::new(imu::ImuModule::new(imu::ImuConfig {
            mqtt_broker: settings.mqtt.broker.clone(),
            mqtt_port: settings.mqtt.port,
            mqtt_topic: "device/imu/+/data".to_string(),
            client_id: format!("remipedia_imu_{}", uuid::Uuid::new_v4()),
            ..Default::default()
        })));
        info!("📡 注册IMU传感器MQTT模块");
    }

    registry.start_all(pool).await?;
    info!("✅ 所有Ingest模块已启动");

    Ok(registry)
}

/// 创建 Rocket 应用
async fn build_rocket(settings: &Settings, pool: PgPool, registry: ModuleRegistry) -> Rocket<Build> {
    rocket::build()
        .manage(pool)
        .manage(settings.jwt.clone())
        .manage(settings.mqtt.clone())
        .manage(registry)
        .attach(Cors)
        .attach(RequestLogger)
        .mount("/", remipedia::api::routes::health::routes())
        .mount("/api/v1", routes())
        .mount("/", swagger_ui())
}

#[tokio::main]
async fn main() -> AppResult<()> {
    init_logging();
    info!("🚀 Remipedia IoT Health Platform 启动中...");

    let settings = Settings::new()?;
    info!("📋 配置加载成功");

    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .connect(&settings.database.url)
        .await?;
    info!("🔌 数据库连接池创建成功");

    run_migrations(&pool).await?;
    init_admin(&pool).await?;

    // 启动 Ingest 模块
    let registry = init_ingest_modules(&pool, &settings).await?;

    // 启动 HTTP 服务器
    let rocket = build_rocket(&settings, pool, registry).await;
    info!(
        "🌐 HTTP 服务器启动于 {}:{}",
        settings.server.host, settings.server.port
    );

    rocket.launch().await?;

    info!("👋 服务器已关闭");
    Ok(())
}
