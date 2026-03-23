use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, SaltString, PasswordHasher},
    Argon2,
};
use log::{error, info, warn};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Build, Rocket};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use remipedia::api::routes;
use remipedia::api::swagger_ui;
use remipedia::config::Settings;
use remipedia::ingest::{MqttIngest, TcpServer};
use remipedia::repository::UserRepository;

/// CORS Fairing - 尽可能宽松的配置
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
        // 允许所有来源
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));

        // 允许所有标准HTTP方法
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD, CONNECT, TRACE",
        ));

        // 动态返回请求的 Access-Control-Request-Headers，或允许所有
        if let Some(request_headers) = request.headers().get_one("Access-Control-Request-Headers") {
            response.set_header(Header::new(
                "Access-Control-Allow-Headers",
                request_headers,
            ));
        } else {
            response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        }

        // 预检请求缓存时间（24小时）
        response.set_header(Header::new("Access-Control-Max-Age", "86400"));

        // 暴露所有响应头给客户端
        response.set_header(Header::new("Access-Control-Expose-Headers", "*"));
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

/// 哈希密码
fn hash_password(password: &str) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| anyhow::anyhow!("密码哈希失败: {}", e))
}

/// 初始化管理员账户
async fn init_admin(pool: &PgPool) -> anyhow::Result<()> {
    let user_repo = UserRepository::new(pool);

    // 检查是否已存在管理员
    if user_repo.exists_admin().await? {
        info!("✅ 管理员账户已存在，跳过初始化");
        return Ok(());
    }

    // 从环境变量获取管理员信息，或使用默认值
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

    // 哈希密码
    let password_hash = hash_password(&admin_password)?;

    // 创建管理员
    let admin = user_repo.create_admin(&admin_username, &password_hash).await?;

    info!("🎉 初始管理员账户创建成功!");
    info!("   📧 用户名: {}", admin.username);
    
    // 安全提示
    if std::env::var("ADMIN_PASSWORD").is_err() {
        warn!("⚠️  使用了默认密码 'admin123'，请立即修改密码！");
        warn!("   设置环境变量 ADMIN_PASSWORD 来使用自定义密码");
    }

    Ok(())
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
        .mount("/", swagger_ui())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    init_logging();

    info!("🚀 Remipedia IoT Health Platform 启动中...");

    // 加载配置
    let settings = Settings::new().map_err(|e| anyhow::anyhow!("配置加载失败: {}", e))?;

    info!("📋 配置加载成功");

    // 创建数据库连接池
    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .connect(&settings.database.url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {}", e))?;

    info!("🔌 数据库连接池创建成功");

    // 初始化管理员账户
    init_admin(&pool).await?;

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

    // 启动 TCP 服务器（如果启用）
    if settings.tcp.enabled {
        let tcp_pool = Arc::new(pool.clone());
        let tcp_config = settings.tcp.clone();

        tokio::spawn(async move {
            info!("🔌 TCP 服务器启动中...");
            let tcp_server = TcpServer::new(tcp_config, tcp_pool);
            if let Err(e) = tcp_server.start().await {
                error!("TCP 服务器启动失败: {}", e);
            }
        });
    }

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
