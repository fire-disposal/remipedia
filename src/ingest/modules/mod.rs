//! Ingest 模块 - 解耦架构
//!
//! 每个模块独立负责：传输监听 + 协议解析 + 业务处理
//! 模块之间完全解耦，通过统一接口注册

pub mod imu;
pub mod mattress;
pub mod mqtt_runner;
pub mod vision;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;
use crate::errors::AppResult;

/// Ingest模块统一接口
#[async_trait]
pub trait IngestModule {
    /// 启动模块
    async fn start(&self, pool: &PgPool) -> AppResult<()>;
    
    /// 获取模块名称
    fn name(&self) -> &str;
    
    /// 获取模块描述
    fn description(&self) -> &str;
}

/// 模块注册表
pub struct ModuleRegistry {
    modules: Vec<Box<dyn IngestModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn register(&mut self, module: Box<dyn IngestModule>) {
        log::info!("注册Ingest模块: {}", module.name());
        self.modules.push(module);
    }

    pub async fn start_all(&self, pool: &PgPool) -> AppResult<()> {
        for module in &self.modules {
            log::info!("启动模块: {} - {}", module.name(), module.description());
            module.start(pool).await?;
        }
        Ok(())
    }

    pub fn list(&self) -> Vec<(&str, &str)> {
        self.modules.iter()
            .map(|m| (m.name(), m.description()))
            .collect()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 共享工具函数
// ---------------------------------------------------------------------------

/// 解析或自动注册设备。
///
/// 所有 ingest 模块共用此函数，避免三份重复实现。
/// - `device_type`：如 `"vision_camera"`、`"imu_sensor"`、`"smart_mattress"`
/// - `metadata`：可选设备元数据（视觉/IMU 模块会传入传感器能力描述）
pub async fn resolve_or_create_device(
    pool: &PgPool,
    device_id: &str,
    device_type: &str,
    metadata: Option<serde_json::Value>,
) -> AppResult<Uuid> {
    use crate::repository::DeviceRepository;
    use crate::core::entity::NewDevice;

    let repo = DeviceRepository::new(pool);

    // 先尝试按 serial_number 查找
    if let Some(device) = repo.find_by_serial(device_id).await? {
        return Ok(device.id);
    }

    // 不存在则自动创建
    let new_device = NewDevice {
        serial_number: device_id.to_string(),
        device_type: device_type.to_string(),
        status: "active".to_string(),
        firmware_version: None,
        metadata,
    };

    let device = repo.insert(&new_device).await?;
    log::info!("自动注册设备 [{}]: {} -> {}", device_type, device_id, device.id);
    Ok(device.id)
}

// 为具体模块实现IngestModule trait
#[async_trait]
impl IngestModule for mattress::MattressModule {
    async fn start(&self, pool: &PgPool) -> AppResult<()> {
        self.start(pool).await
    }

    fn name(&self) -> &str {
        "mattress_tcp"
    }

    fn description(&self) -> &str {
        "智能床垫TCP模块 (Msgpack协议)"
    }
}

#[async_trait]
impl IngestModule for vision::VisionModule {
    async fn start(&self, pool: &PgPool) -> AppResult<()> {
        self.start(pool).await
    }

    fn name(&self) -> &str {
        "vision_mqtt"
    }

    fn description(&self) -> &str {
        "视觉识别MQTT模块"
    }
}

#[async_trait]
impl IngestModule for imu::ImuModule {
    async fn start(&self, pool: &PgPool) -> AppResult<()> {
        self.start(pool).await
    }

    fn name(&self) -> &str {
        "imu_mqtt"
    }

    fn description(&self) -> &str {
        "IMU传感器MQTT模块 (跌倒检测)"
    }
}
