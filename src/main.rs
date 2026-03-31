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
use remipedia::ingest::{
    IngestionPipeline, PipelineConfig, AdapterRegistry,
    transport::{TcpTransportV2, MqttTransportV2, WebSocketTransportV2},
};
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
    pipeline: Arc<IngestionPipeline>,
) -> Rocket<Build> {
    // 创建适配器注册表
    let mut registry = AdapterRegistry::new();
    
    // 注册床垫适配器
    registry.register(Box::new(
        remipedia::ingest::adapters::mattress::MattressAdapterV2::new()
    ));
    
    // 注册简单转发适配器（示例）
    registry.register(Box::new(
        remipedia::ingest::adapters::ForwardAdapter::from_json(
            remipedia::ingest::adapter::DeviceType::HeartRateMonitor
        )
    ));
    
    let registry = Arc::new(registry);
    
    rocket::build()
        .manage(pool)
        .manage(settings.jwt.clone())
        .manage(settings.mqtt.clone())
        .manage(registry)
        .manage(pipeline)
        .attach(Cors)
        .mount("/", remipedia::api::routes::health::routes())
        .mount("/api/v1", routes())
        .mount("/", swagger_ui())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    info!("🚀 Remipedia IoT Health Platform 启动中...");

    let settings = Settings::new()
        .map_err(|e| anyhow::anyhow!("配置加载失败: {}", e))?;
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

    // 创建新的 IngestionPipeline
    let config = PipelineConfig {
        queue_size: 50,
        batch_size: 10,
        auto_register: true,
        max_states: 10000,
        state_idle_timeout_secs: 1800,
    };
    
    let adapter_registry = Arc::new(AdapterRegistry::new());
    let pipeline = Arc::new(IngestionPipeline::new(&pool, adapter_registry, config));
    
    info!("📡 数据接入管道初始化完成（队列大小: 50）");

    // 启动 Transport 层
    {
        // TCP Transport
        if settings.tcp.enabled {
            let pipeline_clone = pipeline.clone();
            let tcp_bind: std::net::SocketAddr = format!("0.0.0.0:{}", settings.tcp.port)
                .parse()
                .map_err(|_| anyhow::anyhow!("无效的TCP绑定地址"))?;
            let tcp_transport = TcpTransportV2::new(tcp_bind);
            
            tokio::spawn(async move {
                if let Err(e) = tcp_transport.start(pipeline_clone).await {
                    log::error!("TCP Transport 错误: {}", e);
                }
            });
            info!("📡 TCP Transport 启动: {}", settings.tcp.port);
        }

        // MQTT Transport
        if settings.mqtt.enabled {
            let mqtt_cfg = settings.mqtt.clone();
            let pipeline_clone = pipeline.clone();
            let broker = mqtt_cfg.broker.clone();
            let port = mqtt_cfg.port;
            
            tokio::spawn(async move {
                let mqtt_transport = MqttTransportV2::new(
                    mqtt_cfg.broker,
                    mqtt_cfg.port,
                    mqtt_cfg.client_id,
                );
                if let Err(e) = mqtt_transport.start(pipeline_clone).await {
                    log::error!("MQTT Transport 错误: {}", e);
                }
            });
            info!("📡 MQTT Transport 启动: {}:{}", broker, port);
        }

        // WebSocket Transport
        if settings.websocket.enabled {
            let ws_bind: std::net::SocketAddr = format!("0.0.0.0:{}", settings.websocket.port)
                .parse()
                .map_err(|_| anyhow::anyhow!("无效的WebSocket绑定地址"))?;
            let pipeline_clone = pipeline.clone();
            
            tokio::spawn(async move {
                let ws_transport = WebSocketTransportV2::new(ws_bind);
                if let Err(e) = ws_transport.start(pipeline_clone).await {
                    log::error!("WebSocket Transport 错误: {}", e);
                }
            });
            info!("📡 WebSocket Transport 启动: {}", settings.websocket.port);
        }
    }

    // 启动 HTTP 服务器
    let rocket = build_rocket(&settings, pool, pipeline).await;
    info!("🌐 HTTP 服务器启动于 {}:{}", settings.server.host, settings.server.port);

    rocket
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!("服务器启动失败: {}", e))?;

    info!("👋 服务器已关闭");
    Ok(())
}
