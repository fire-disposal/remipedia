//! 视觉识别 MQTT 模块
//!
//! 独立模块：使用rumqttc订阅MQTT主题，处理视觉识别设备的JSON数据
//! 包含：MQTT连接 + 订阅 + JSON解析 + 事件检测

use async_trait::async_trait;
use crate::ingest::types::{DataCategory, DataPoint, Severity};
use crate::errors::{AppError, AppResult};
use crate::ingest::modules::mqtt_runner;
use crate::ingest::modules::store_data_points;
use crate::repository::RawDataRepository;
use crate::service::DataService;
use rumqttc::QoS;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// 视觉识别模块配置
#[derive(Debug, Clone)]
pub struct VisionConfig {
    pub mqtt_broker: String,
    pub mqtt_port: u16,
    pub mqtt_topic: String,
    pub client_id: String,
    pub qos: QoS,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            mqtt_broker: "localhost".to_string(),
            mqtt_port: 1883,
            mqtt_topic: "device/vision/+/detect".to_string(),
            client_id: format!("remipedia_vision_{}", uuid::Uuid::new_v4()),
            qos: QoS::AtLeastOnce,
        }
    }
}

/// 视觉检测结果
#[derive(Debug, Clone)]
struct VisionDetection {
    #[allow(dead_code)]
    device_id: String,
    timestamp: i64,
    event_type: String,      // "fall", "wander", "visitor", etc.
    confidence: f32,         // 置信度 0-1
    location: String,        // 位置描述
    person_id: Option<String>, // 识别到的人员ID（如有）
    image_url: Option<String>, // 截图URL
    metadata: serde_json::Value,
}

/// 视觉识别模块
pub struct VisionModule {
    config: VisionConfig,
}

impl VisionModule {
    pub fn new(config: VisionConfig) -> Self {
        Self { config }
    }

    /// 启动模块
    ///
    /// 1. 构造 MQTT 连接参数
    /// 2. 创建 `VisionHandler`（实现 `MqttMessageHandler` trait）
    /// 3. 通过 `spawn_mqtt_task` + `run_with_handler` 启动后台事件循环
    pub async fn start(&self, pool: &PgPool) -> AppResult<()> {
        log::info!(
            "视觉识别模块启动，订阅: {} on {}:{}",
            self.config.mqtt_topic,
            self.config.mqtt_broker,
            self.config.mqtt_port
        );

        let pool = pool.clone();
        let params = mqtt_runner::MqttParams {
            client_id: self.config.client_id.clone(),
            broker: self.config.mqtt_broker.clone(),
            port: self.config.mqtt_port,
            topic: self.config.mqtt_topic.clone(),
            qos: self.config.qos,
        };

        let handler = Arc::new(VisionHandler);

        mqtt_runner::spawn_mqtt_task("vision".to_string(), params, move |p| {
            let pool = pool.clone();
            let handler = handler.clone();
            async move { mqtt_runner::run_with_handler(p, pool, handler).await }
        });

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MQTT Handler Trait 实现
// ---------------------------------------------------------------------------

/// 视觉模块的 MQTT 消息处理器。
///
/// 无状态结构体，实现 `MqttMessageHandler` trait，
/// 每条 Publish 消息在独立的 tokio task 中异步处理。
struct VisionHandler;

#[async_trait]
impl mqtt_runner::MqttMessageHandler for VisionHandler {
    fn module_name(&self) -> &str {
        "vision"
    }

    async fn handle(&self, publish: rumqttc::Publish, pool: &PgPool) -> AppResult<()> {
        let topic = &publish.topic;
        let payload = &publish.payload;
        let data_service = DataService::new(pool.clone());
        let raw_repo = RawDataRepository::new(pool);

        // 归档原始数据
        let raw_id = raw_repo.archive_raw("vision_mqtt", payload, topic.to_string()).await.ok();

        // 解析主题提取设备ID: device/vision/{device_id}/detect
        let device_id_str = topic.split('/').nth(2)
            .ok_or_else(|| AppError::validation("无效的主题格式"))?;

        // 解析JSON
        let detection = match parse_vision_detection(payload, device_id_str) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("解析视觉检测数据失败: {}", e);
                if let Some(id) = raw_id {
                    let _ = raw_repo.mark_status(
                        id,
                        crate::core::entity::RawIngestStatus::FormatError,
                        Some(&e.to_string())
                    ).await;
                }
                return Err(e);
            }
        };

        // 解析或创建设备
        let device_uuid = match crate::ingest::modules::resolve_or_create_device(
            pool,
            device_id_str,
            "vision_camera",
            Some(serde_json::json!({
                "capabilities": ["fall_detection", "wander_detection"]
            })),
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                log::error!("解析视觉设备失败: {}", e);
                return Err(e);
            }
        };

        // 生成数据点
        let points = create_vision_datapoints(detection, device_uuid);

        // 通过 DataService 存储
        if !points.is_empty() {
            if let Err(e) = store_data_points(&data_service, &points, &device_uuid).await {
                log::error!("存储视觉数据失败: {}", e);
            } else {
                log::debug!("视觉检测数据已存储: {} 条", points.len());
            }
        }

        // 标记成功
        if let Some(id) = raw_id {
            let _ = raw_repo.mark_status(
                id,
                crate::core::entity::RawIngestStatus::Ingested,
                None
            ).await;
        }

        Ok(())
    }
}

/// 解析视觉检测数据
fn parse_vision_detection(payload: &[u8], device_id: &str) -> AppResult<VisionDetection> {
    let json: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| AppError::validation(format!("JSON解析失败: {}", e)))?;

    let event_type = json.get("event_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::validation("缺少event_type"))?;

    Ok(VisionDetection {
        device_id: device_id.to_string(),
        timestamp: json.get("timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp()),
        event_type: event_type.to_string(),
        confidence: json.get("confidence")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.0),
        location: json.get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        person_id: json.get("person_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        image_url: json.get("image_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        metadata: json.get("metadata").cloned().unwrap_or(serde_json::json!({})),
    })
}

/// 生成视觉识别数据点
fn create_vision_datapoints(detection: VisionDetection, device_id: Uuid) -> Vec<DataPoint> {
    let mut points = Vec::new();
    let now = chrono::DateTime::from_timestamp(detection.timestamp, 0)
        .unwrap_or_else(chrono::Utc::now);

    // 根据事件类型确定严重级别
    let severity = match detection.event_type.as_str() {
        "fall" => Severity::Alert,
        "wander" => Severity::Warning,
        "visitor" => Severity::Info,
        "abnormal_behavior" => Severity::Warning,
        _ => Severity::Info,
    };

    // 事件数据点
    let event_payload = serde_json::json!({
        "event_type": detection.event_type,
        "confidence": detection.confidence,
        "location": detection.location,
        "person_id": detection.person_id,
        "image_url": detection.image_url,
        "metadata": detection.metadata,
    });

    points.push(DataPoint {
        time: now,
        device_id: Some(device_id),
        patient_id: None,
        data_type: format!("vision_{}", detection.event_type),
        data_category: DataCategory::Event,
        value_numeric: Some(detection.confidence as f64),
        value_text: Some(format!("{} detected at {}", detection.event_type, detection.location)),
        severity: Some(severity),
        status: Some(crate::ingest::types::EventStatus::Active),
        payload: event_payload,
        source: "vision_mqtt".to_string(),
    });

    // 如果置信度低，添加警告
    if detection.confidence < 0.7 {
        points.push(DataPoint {
            time: now,
            device_id: Some(device_id),
            patient_id: None,
            data_type: "vision_low_confidence".to_string(),
            data_category: DataCategory::Event,
            value_numeric: Some(detection.confidence as f64),
            value_text: Some(format!("低置信度检测: {} ({:.0}%)", 
                detection.event_type, 
                detection.confidence * 100.0)),
            severity: Some(Severity::Warning),
            status: Some(crate::ingest::types::EventStatus::Active),
            payload: serde_json::json!({"original_event": detection.event_type}),
            source: "vision_mqtt".to_string(),
        });
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vision_detection() {
        let payload = br#"{
            "event_type": "fall",
            "confidence": 0.95,
            "location": "living_room",
            "timestamp": 1704067200,
            "person_id": "person_001"
        }"#;

        let detection = parse_vision_detection(payload, "camera_001").unwrap();
        assert_eq!(detection.event_type, "fall");
        assert_eq!(detection.confidence, 0.95);
        assert_eq!(detection.location, "living_room");
    }
}
