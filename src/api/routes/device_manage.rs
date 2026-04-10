//! 设备管理 API - V2架构适配版
//!
//! 注意：新架构使用独立模块，此文件保留用于查询已注册的设备类型

use rocket::serde::json::Json;
use rocket::{get, State};
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::repository::DeviceRepository;

#[derive(rocket::serde::Serialize, ToSchema)]
pub struct DeviceTypeInfo {
    pub device_type: String,
    pub display_name: String,
    pub description: String,
}

#[derive(rocket::serde::Serialize, ToSchema)]
pub struct DeviceSystemStatus {
    pub supported_types: Vec<DeviceTypeInfo>,
    pub total_devices: i64,
    pub message: String,
}

/// 获取支持的设备类型列表
#[utoipa::path(
    get,
    path = "/admin/devices/types",
    tag = "device_manage",
    responses(
        (status = 200, description = "设备类型列表", body = Vec<DeviceTypeInfo>),
    )
)]
#[get("/admin/devices/types")]
pub async fn list_device_types() -> Json<Vec<DeviceTypeInfo>> {
    // 新架构下的固定设备类型列表
    let types = vec![
        DeviceTypeInfo {
            device_type: "smart_mattress".to_string(),
            display_name: "智能床垫".to_string(),
            description: "TCP协议，Msgpack格式，支持心率/呼吸/离床检测".to_string(),
        },
        DeviceTypeInfo {
            device_type: "vision_camera".to_string(),
            display_name: "视觉识别摄像头".to_string(),
            description: "MQTT协议，JSON格式，支持跌倒/徘徊检测".to_string(),
        },
        DeviceTypeInfo {
            device_type: "imu_sensor".to_string(),
            display_name: "IMU传感器".to_string(),
            description: "MQTT协议，JSON格式，支持跌倒检测".to_string(),
        },
    ];
    
    Json(types)
}

/// 获取设备系统状态
#[utoipa::path(
    get,
    path = "/admin/devices/status",
    tag = "device_manage",
    responses(
        (status = 200, description = "系统状态", body = DeviceSystemStatus),
    )
)]
#[get("/admin/devices/status")]
pub async fn get_device_system_status(
    pool: &State<PgPool>,
) -> Json<DeviceSystemStatus> {
    let device_repo = DeviceRepository::new(pool);
    
    // 获取设备总数
    let total_devices = match device_repo.count_all().await {
        Ok(count) => count,
        Err(_) => 0,
    };
    
    let status = DeviceSystemStatus {
        supported_types: list_device_types().await.into_inner(),
        total_devices,
        message: "新架构：独立模块运行中".to_string(),
    };
    
    Json(status)
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![list_device_types, get_device_system_status]
}
