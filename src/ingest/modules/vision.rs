//! 视觉识别 MQTT 模块
//!
//! 独立模块：使用rumqttc订阅MQTT主题，处理视觉识别设备的JSON数据
//! 包含：MQTT连接 + 订阅 + JSON解析 + 事件检测

use crate::core::entity::{DataPoint, DataCategory, Severity};
use crate::errors::AppResult;
use crate::repository::{DataRepository, RawDataRepository};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use sqlx::PgPool;
use std::time::Duration;
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
    pub async fn start(&self, pool: &PgPool) -> AppResult<()> {
        log::info!(
            "视觉识别模块启动，订阅: {} on {}:{}", 
            self.config.mqtt_topic, 
            self.config.mqtt_broker,
            self.config.mqtt_port
        );

        let pool = pool.clone();
        let broker = self.config.mqtt_broker.clone();
        let port = self.config.mqtt_port;
        let client_id = self.config.client_id.clone();
        let topic = self.config.mqtt_topic.clone();
        let qos = self.config.qos;

        tokio::spawn(async move {
            loop {
                match Self::run_mqtt_client(&pool, &broker, port, &client_id, &topic, qos).await {
                    Ok(_) => {
                        log::info!("视觉识别MQTT客户端正常退出");
                        break;
                    }
                    Err(e) => {
                        log::error!("视觉识别MQTT客户端错误: {}, 5秒后重连...", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// 运行MQTT客户端
    async fn run_mqtt_client(
        pool: &PgPool,
        broker: &str,
        port: u16,
        client_id: &str,
        topic: &str,
        qos: QoS,
    ) -> AppResult<()> {
        let mut mqttoptions = MqttOptions::new(client_id, broker, port);
        mqttoptions.set_keep_alive(Duration::from_secs(30));
        mqttoptions.set_clean_session(false); // 持久会话，避免消息丢失

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);

        // 订阅主题
        client.subscribe(topic, qos).await
            .map_err(|e| crate::errors::AppError::ValidationError(format!("订阅失败: {}", e)))?;

        log::info!("视觉识别模块已订阅: {}", topic);

        // 消息处理循环
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let topic = publish.topic.clone();
                    let payload = publish.payload.to_vec();
                    let pool = pool.clone();
                    
                    // 异步处理消息
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_message(&topic, &payload, &pool).await {
                            log::error!("处理视觉检测消息失败 [{}]: {}", topic, e);
                        }
                    });
                }
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    log::info!("视觉识别模块MQTT连接已建立");
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    log::info!("视觉识别模块订阅已确认");
                }
                Ok(_) => {}
                Err(e) => {
                    log::error!("视觉识别MQTT错误: {}", e);
                    return Err(crate::errors::AppError::ValidationError(format!("MQTT错误: {}", e)));
                }
            }
        }
    }

    /// 处理单条MQTT消息
    async fn handle_message(
        topic: &str,
        payload: &[u8],
        pool: &PgPool,
    ) -> AppResult<()> {
        let raw_repo = RawDataRepository::new(pool);
        let data_repo = DataRepository::new(pool);

        // 归档原始数据
        let raw_id = raw_repo.archive_raw("vision_mqtt", payload, topic.to_string()).await.ok();

        // 解析主题提取设备ID: device/vision/{device_id}/detect
        let device_id_str = topic.split('/').nth(2)
            .ok_or_else(|| crate::errors::AppError::ValidationError("无效的主题格式".into()))?;

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
        let device_uuid = match resolve_or_create_device(pool, device_id_str).await {
            Ok(id) => id,
            Err(e) => {
                log::error!("解析视觉设备失败: {}", e);
                return Err(e);
            }
        };

        // 生成数据点
        let points = create_vision_datapoints(detection, device_uuid);

        // 存储
        if !points.is_empty() {
            if let Err(e) = data_repo.insert_datapoints(&points).await {
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
        .map_err(|e| crate::errors::AppError::ValidationError(format!("JSON解析失败: {}", e)))?;

    let event_type = json.get("event_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::errors::AppError::ValidationError("缺少event_type".into()))?;

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
        status: Some(crate::core::entity::EventStatus::Active),
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
            status: Some(crate::core::entity::EventStatus::Active),
            payload: serde_json::json!({"original_event": detection.event_type}),
            source: "vision_mqtt".to_string(),
        });
    }

    points
}

/// 解析或创建设备
async fn resolve_or_create_device(pool: &PgPool, device_id_str: &str) -> AppResult<Uuid> {
    use crate::repository::DeviceRepository;
    use crate::core::entity::NewDevice;

    let repo = DeviceRepository::new(pool);
    
    // 尝试查找
    if let Some(device) = repo.find_by_serial(device_id_str).await? {
        return Ok(device.id);
    }
    
    // 自动创建设备
    let new_device = NewDevice {
        serial_number: device_id_str.to_string(),
        device_type: "vision_camera".to_string(),
        status: "active".to_string(),
        firmware_version: None,
        metadata: Some(serde_json::json!({
            "capabilities": ["fall_detection", "wander_detection"]
        })),
    };
    
    let device = repo.insert(&new_device).await?;
    log::info!("自动注册视觉设备: {} -> {}", device_id_str, device.id);
    Ok(device.id)
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
