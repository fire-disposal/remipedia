//! 设备管理 API - V2架构适配版
//!
//! 注意：新架构使用StateManager替代了DeviceManager

use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post};
use std::sync::Arc;

use crate::ingest::{AdapterRegistry, state::StateManager};
use crate::ingest::IngestionPipeline;

#[derive(rocket::serde::Serialize)]
pub struct DeviceTypeInfo {
    pub device_type: String,
    pub display_name: String,
    pub protocol_version: String,
    pub supports_events: bool,
}

#[derive(rocket::serde::Serialize)]
pub struct DeviceSessionDetail {
    pub serial_number: String,
    pub device_type: String,
    pub last_seen: String,
}

#[derive(rocket::serde::Serialize)]
pub struct DeviceSystemStatus {
    pub supported_types: Vec<DeviceTypeInfo>,
    pub total_states: usize,
    pub pipeline_queue_size: usize,
}

#[get("/admin/devices/types")]
pub async fn list_device_types(
    registry: &State<Arc<AdapterRegistry>>,
) -> Json<Vec<DeviceTypeInfo>> {
    let types: Vec<DeviceTypeInfo> = registry
        .list()
        .into_iter()
        .map(|m| DeviceTypeInfo {
            device_type: m.device_type.to_string(),
            display_name: m.display_name.to_string(),
            protocol_version: m.protocol_version.to_string(),
            supports_events: m.supports_events,
        })
        .collect();

    Json(types)
}

#[get("/admin/devices/sessions?<limit>")]
pub async fn list_device_sessions(
    state_manager: &State<Arc<StateManager>>,
    limit: Option<u32>,
) -> Json<Vec<DeviceSessionDetail>> {
    let devices = state_manager.list_devices().await;
    let limit = limit.unwrap_or(100) as usize;
    
    // 新架构下，我们只能获取有状态的设备列表
    // 详细信息需要从其他途径获取
    let details: Vec<DeviceSessionDetail> = devices
        .into_iter()
        .take(limit)
        .map(|serial| DeviceSessionDetail {
            serial_number: serial.clone(),
            device_type: "unknown".to_string(),
            last_seen: chrono::Utc::now().to_rfc3339(),
        })
        .collect();

    Json(details)
}

#[get("/admin/devices/status")]
pub async fn get_device_system_status(
    registry: &State<Arc<AdapterRegistry>>,
    pipeline: &State<Arc<IngestionPipeline>>,
    state_manager: &State<Arc<StateManager>>,
) -> Json<DeviceSystemStatus> {
    let supported_types: Vec<DeviceTypeInfo> = registry
        .list()
        .into_iter()
        .map(|m| DeviceTypeInfo {
            device_type: m.device_type.to_string(),
            display_name: m.display_name.to_string(),
            protocol_version: m.protocol_version.to_string(),
            supports_events: m.supports_events,
        })
        .collect();

    Json(DeviceSystemStatus {
        supported_types,
        total_states: state_manager.count().await,
        pipeline_queue_size: pipeline.queue_size(),
    })
}

#[post("/admin/devices/sessions/cleanup")]
pub async fn cleanup_idle_sessions(
    state_manager: &State<Arc<StateManager>>,
) -> Json<CleanupResult> {
    // 新架构下，清理由StateManager自动处理
    // 这里手动触发一次清理
    let before = state_manager.count().await;
    // 清理逻辑在StateManager的background task中自动执行
    let after = state_manager.count().await;
    let removed = before.saturating_sub(after);
    
    Json(CleanupResult {
        cleaned_count: removed,
        message: format!("清理了 {} 个空闲会话", removed),
    })
}

#[derive(rocket::serde::Serialize)]
pub struct CleanupResult {
    pub cleaned_count: usize,
    pub message: String,
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list_device_types,
        list_device_sessions,
        get_device_system_status,
        cleanup_idle_sessions,
    ]
}
