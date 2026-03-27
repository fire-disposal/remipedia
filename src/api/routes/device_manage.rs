//! 设备管理 API - 简化版

use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post};
use std::sync::Arc;
use chrono::Utc;

use crate::ingest::{AdapterRegistry, DeviceManager};
use crate::errors::AppResult;

#[derive(rocket::serde::Serialize)]
pub struct DeviceTypeInfo {
    pub device_type: String,
    pub display_name: String,
    pub supported_data_types: Vec<String>,
    pub protocol_version: String,
}

#[derive(rocket::serde::Serialize)]
pub struct DeviceSessionDetail {
    pub serial_number: String,
    pub device_type: String,
    pub last_seen: String,
    pub is_idle: bool,
}

#[derive(rocket::serde::Serialize)]
pub struct DeviceSystemStatus {
    pub supported_types: Vec<DeviceTypeInfo>,
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub idle_sessions: usize,
}

#[get("/admin/devices/types")]
pub async fn list_device_types(
    registry: &State<Arc<AdapterRegistry>>,
) -> Json<Vec<DeviceTypeInfo>> {
    let types: Vec<DeviceTypeInfo> = registry
        .all_metadata()
        .into_iter()
        .map(|m| DeviceTypeInfo {
            device_type: m.device_type.to_string(),
            display_name: m.display_name.to_string(),
            supported_data_types: m.supported_data_types.iter().map(|s| s.to_string()).collect(),
            protocol_version: m.protocol_version.to_string(),
        })
        .collect();

    Json(types)
}

#[get("/admin/devices/sessions?<idle_only>&<limit>")]
pub async fn list_device_sessions(
    manager: &State<Arc<DeviceManager>>,
    idle_only: Option<bool>,
    limit: Option<u32>,
) -> Json<Vec<DeviceSessionDetail>> {
    let devices = manager.list_devices().await;
    
    let limit = limit.unwrap_or(100) as usize;
    
    let details: Vec<DeviceSessionDetail> = devices
        .into_iter()
        .filter(|s| !idle_only.unwrap_or(false) || s.is_idle)
        .take(limit)
        .map(|s| DeviceSessionDetail {
            serial_number: s.serial_number,
            device_type: s.device_type,
            last_seen: s.last_seen.to_rfc3339(),
            is_idle: s.is_idle,
        })
        .collect();

    Json(details)
}

#[get("/admin/devices/status")]
pub async fn get_device_system_status(
    registry: &State<Arc<AdapterRegistry>>,
    manager: &State<Arc<DeviceManager>>,
) -> Json<DeviceSystemStatus> {
    let devices = manager.list_devices().await;
    
    let total = devices.len();
    let idle = devices.iter().filter(|s| s.is_idle).count();
    
    let supported_types: Vec<DeviceTypeInfo> = registry
        .all_metadata()
        .into_iter()
        .map(|m| DeviceTypeInfo {
            device_type: m.device_type.to_string(),
            display_name: m.display_name.to_string(),
            supported_data_types: m.supported_data_types.iter().map(|s| s.to_string()).collect(),
            protocol_version: m.protocol_version.to_string(),
        })
        .collect();

    Json(DeviceSystemStatus {
        supported_types,
        total_sessions: total,
        active_sessions: total - idle,
        idle_sessions: idle,
    })
}

#[post("/admin/devices/sessions/cleanup")]
pub async fn cleanup_idle_sessions(
    manager: &State<Arc<DeviceManager>>,
) -> Json<CleanupResult> {
    let removed = manager.cleanup_idle().await;
    
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
