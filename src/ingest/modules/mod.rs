//! Ingest 模块 - 解耦架构
//!
//! 每个模块独立负责：传输监听 + 协议解析 + 业务处理
//! 模块之间完全解耦，通过统一接口注册

pub mod imu;
pub mod mattress;
pub mod mqtt_runner;
pub mod vision;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;
use crate::core::entity::{AlertSeverity, AlertStatus, DataStreamType, NewAlertEvent};
use crate::errors::AppResult;
use crate::ingest::types::{DataCategory, DataPoint, Severity};
use crate::service::DataService;

/// Ingest模块统一接口
#[async_trait]
pub trait IngestModule: Send + Sync {
    /// 启动模块
    async fn start(&self, pool: &PgPool) -> AppResult<()>;
    
    /// 获取模块名称
    fn name(&self) -> &str;
    
    /// 获取模块描述
    fn description(&self) -> &str;

    /// 获取模块健康状态（默认返回 running）
    fn health(&self) -> ModuleHealth {
        ModuleHealth {
            name: self.name().to_string(),
            description: self.description().to_string(),
            is_running: true,
        }
    }
}

/// 模块健康状态
#[derive(Debug, Clone, Serialize)]
pub struct ModuleHealth {
    pub name: String,
    pub description: String,
    pub is_running: bool,
}

/// 模块注册表
pub struct ModuleRegistry {
    modules: Vec<Box<dyn IngestModule>>,
    started_at: DateTime<Utc>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            started_at: Utc::now(),
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

    /// 返回所有模块的健康状态
    pub fn health_check(&self) -> Vec<ModuleHealth> {
        self.modules.iter()
            .map(|m| m.health())
            .collect()
    }

    /// 获取注册表启动时间
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
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

/// 将旧 DataPoint 列表通过 DataService 写入新表
///
/// 桥接函数：Ingest 模块保留 process_* 内部纯函数，在存储时转换。
pub(crate) async fn store_data_points(
    data_service: &DataService,
    points: &[DataPoint],
    device_id: &Uuid,
) -> AppResult<()> {
    for point in points {
        let value_numeric = point
            .value_numeric
            .and_then(rust_decimal::prelude::FromPrimitive::from_f64);
        let patient_id = point.patient_id;

        match point.data_category {
            DataCategory::Metric => {
                data_service
                    .ingest(
                        Some(*device_id),
                        patient_id,
                        point.data_type.clone(),
                        DataStreamType::Metric,
                        value_numeric,
                        point.value_text.clone(),
                        point.payload.clone(),
                    )
                    .await?;
            }
            DataCategory::Event => {
                let severity = point
                    .severity
                    .map(|s| match s {
                        Severity::Info => AlertSeverity::Info,
                        Severity::Warning => AlertSeverity::Warning,
                        Severity::Alert => AlertSeverity::Alert,
                    })
                    .unwrap_or(AlertSeverity::Info);

                let pid = if let Some(pid) = patient_id {
                    pid
                } else {
                    data_service
                        .resolve_patient_from_device(device_id)
                        .await?
                        .unwrap_or(Uuid::nil())
                };

                let stream = data_service
                    .find_or_create_stream(device_id, &point.data_type, &DataStreamType::Event, Some(pid))
                    .await?;

                data_service
                    .insert_alert(&NewAlertEvent {
                        stream_id: stream.id,
                        patient_id: pid,
                        severity,
                        status: AlertStatus::Active,
                        value_numeric,
                        value_text: point.value_text.clone(),
                        payload: point.payload.clone(),
                        recorded_at: point.time,
                    })
                    .await?;
            }
        }
    }
    Ok(())
}

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
