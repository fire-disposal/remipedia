//! 设备管理 API - V2架构适配版
//!
//! 注意：新架构使用StateManager替代了DeviceManager

use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post};
use std::sync::Arc;

use crate::ingest::AdapterRegistry;
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
    pub message: String,
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

#[get("/admin/devices/sessions")]
pub async fn list_device_sessions() -> Json<Vec<DeviceSessionDetail>> {
    // V2架构：状态管理在内部，API层不直接访问
    // 返回空列表，实际设备状态通过其他监控手段查看
    Json(vec![])
}

#[get("/admin/devices/status")]
pub async fn get_device_system_status(
    registry: &State<Arc<AdapterRegistry>>,
    pipeline: &State<Arc<IngestionPipeline>>,
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
        total_states: 0, // V2架构：状态管理在内部
        pipeline_queue_size: pipeline.queue_size(),
        message: "V2架构：设备状态管理在接入层内部".to_string(),
    })
}

#[post("/admin/devices/sessions/cleanup")]
pub async fn cleanup_idle_sessions() -> Json<CleanupResult> {
    // V2架构：空闲清理由StateManager自动处理
    // 清理逻辑在background task中自动执行
    Json(CleanupResult {
        cleaned_count: 0,
        message: "V2架构：空闲状态自动清理".to_string(),
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
