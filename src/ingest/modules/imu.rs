//! IMU 传感器 MQTT 模块
//!
//! 独立模块：订阅MQTT主题，处理IMU惯性测量单元数据
//! 包含：MQTT订阅 + JSON解析 + 运动分析

use crate::core::entity::{DataPoint, DataCategory, Severity};
use crate::errors::AppResult;
use crate::repository::{DataRepository, RawDataRepository};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// IMU模块配置
#[derive(Debug, Clone)]
pub struct ImuConfig {
    pub mqtt_broker: String,
    pub mqtt_port: u16,
    pub mqtt_topic: String,
    pub client_id: String,
    pub qos: rumqttc::QoS,
    /// 跌倒检测阈值 (m/s²)
    pub fall_threshold: f64,
    /// 静止阈值
    pub static_threshold: f64,
}

impl Default for ImuConfig {
    fn default() -> Self {
        Self {
            mqtt_broker: "localhost".to_string(),
            mqtt_port: 1883,
            mqtt_topic: "device/imu/+/data".to_string(),
            client_id: format!("remipedia_imu_{}", Uuid::new_v4()),
            qos: rumqttc::QoS::AtLeastOnce,
            fall_threshold: 25.0,  // 约2.5g
            static_threshold: 0.5,
        }
    }
}

/// IMU传感器数据
#[derive(Debug, Clone)]
struct ImuData {
    device_id: String,
    timestamp: i64,
    /// 加速度 (m/s²)
    accel_x: f64,
    accel_y: f64,
    accel_z: f64,
    /// 陀螺仪 (deg/s)
    gyro_x: f64,
    gyro_y: f64,
    gyro_z: f64,
    /// 电池电量 (%)
    battery: Option<u8>,
}

/// IMU状态（用于跌倒检测）
#[derive(Debug, Default)]
struct ImuState {
    /// 历史加速度值（用于计算方差）
    accel_history: Vec<f64>,
    /// 上次活动时间
    last_activity: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否处于跌倒状态
    in_fall_state: bool,
    /// 静止开始时间
    static_since: Option<chrono::DateTime<chrono::Utc>>,
}

impl ImuState {
    fn new() -> Self {
        Self {
            accel_history: Vec::with_capacity(10),
            last_activity: None,
            in_fall_state: false,
            static_since: None,
        }
    }

    /// 计算加速度向量的模
    fn calc_accel_magnitude(&self, data: &ImuData) -> f64 {
        (data.accel_x.powi(2) + data.accel_y.powi(2) + data.accel_z.powi(2)).sqrt()
    }

    /// 更新历史记录
    fn update_history(&mut self, magnitude: f64) {
        self.accel_history.push(magnitude);
        if self.accel_history.len() > 10 {
            self.accel_history.remove(0);
        }
    }

    /// 计算加速度方差
    fn calc_variance(&self) -> f64 {
        if self.accel_history.len() < 2 {
            return 0.0;
        }
        let mean = self.accel_history.iter().sum::<f64>() / self.accel_history.len() as f64;
        let variance = self.accel_history.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / self.accel_history.len() as f64;
        variance
    }
}

/// IMU模块
pub struct ImuModule {
    config: ImuConfig,
}

impl ImuModule {
    pub fn new(config: ImuConfig) -> Self {
        Self { config }
    }

    /// 启动模块
    pub async fn start(&self, pool: &PgPool) -> AppResult<()> {
        log::info!(
            "IMU模块启动，订阅: {} on {}:{}", 
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
        let config = self.config.clone();

        tokio::spawn(async move {
            loop {
                match Self::run_mqtt_client(&pool, &broker, port, &client_id, &topic, qos, &config).await {
                    Ok(_) => {
                        log::info!("IMU模块MQTT客户端正常退出");
                        break;
                    }
                    Err(e) => {
                        log::error!("IMU模块MQTT客户端错误: {}, 5秒后重连...", e);
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
        qos: rumqttc::QoS,
        config: &ImuConfig,
    ) -> AppResult<()> {
        let mut mqttoptions = MqttOptions::new(client_id, broker, port);
        mqttoptions.set_keep_alive(Duration::from_secs(30));
        mqttoptions.set_clean_session(false);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);

        // 订阅主题
        client.subscribe(topic, qos).await
            .map_err(|e| crate::errors::AppError::ValidationError(format!("订阅失败: {}", e)))?;

        log::info!("IMU模块已订阅: {}", topic);

        // 状态管理（按设备ID）
        let mut states: std::collections::HashMap<String, ImuState> = std::collections::HashMap::new();

        // 消息处理循环
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let topic = publish.topic.clone();
                    let payload = publish.payload.to_vec();
                    let pool = pool.clone();
                    let config = config.clone();

                    // 解析主题提取设备ID
                    if let Some(device_id) = topic.split('/').nth(2) {
                        let state = states.entry(device_id.to_string()).or_insert_with(ImuState::new);
                        
                        // 处理消息
                        if let Err(e) = Self::handle_message(&topic, &payload, &pool, state, &config).await {
                            log::error!("处理IMU消息失败 [{}]: {}", topic, e);
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    log::info!("IMU模块MQTT连接已建立");
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    log::info!("IMU模块订阅已确认");
                }
                Ok(_) => {}
                Err(e) => {
                    log::error!("IMU模块MQTT错误: {}", e);
                    return Err(crate::errors::AppError::ValidationError(format!("MQTT错误: {}", e)));
                }
            }
        }
    }

    /// 处理单条MQTT消息
    pub async fn handle_message(
        topic: &str,
        payload: &[u8],
        pool: &PgPool,
        state: &mut ImuState,
        config: &ImuConfig,
    ) -> AppResult<Vec<DataPoint>> {
        let raw_repo = RawDataRepository::new(pool);
        let data_repo = DataRepository::new(pool);

        // 归档原始数据
        let raw_id = raw_repo.archive_raw("imu_mqtt", payload, topic.to_string()).await.ok();

        // 解析主题提取设备ID: device/imu/{device_id}/data
        let device_id_str = topic.split('/').nth(2)
            .ok_or_else(|| crate::errors::AppError::ValidationError("无效的主题格式".into()))?;

        // 解析JSON
        let imu_data = match parse_imu_data(payload, device_id_str) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("解析IMU数据失败: {}", e);
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
                log::error!("解析IMU设备失败: {}", e);
                return Err(e);
            }
        };

        // 处理数据（跌倒检测等）
        let points = process_imu_data(imu_data, state, device_uuid, config);

        // 存储
        if !points.is_empty() {
            if let Err(e) = data_repo.insert_datapoints(&points).await {
                log::error!("存储IMU数据失败: {}", e);
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

        Ok(points)
    }
}

/// 解析IMU数据
fn parse_imu_data(payload: &[u8], device_id: &str) -> AppResult<ImuData> {
    let json: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| crate::errors::AppError::ValidationError(format!("JSON解析失败: {}", e)))?;

    let accel = json.get("accelerometer")
        .ok_or_else(|| crate::errors::AppError::ValidationError("缺少accelerometer".into()))?;

    let gyro = json.get("gyroscope")
        .ok_or_else(|| crate::errors::AppError::ValidationError("缺少gyroscope".into()))?;

    Ok(ImuData {
        device_id: device_id.to_string(),
        timestamp: json.get("timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp()),
        accel_x: accel.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
        accel_y: accel.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
        accel_z: accel.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0),
        gyro_x: gyro.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
        gyro_y: gyro.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
        gyro_z: gyro.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0),
        battery: json.get("battery").and_then(|v| v.as_u64()).map(|v| v as u8),
    })
}

/// 处理IMU数据，进行跌倒检测等分析
fn process_imu_data(
    data: ImuData,
    state: &mut ImuState,
    device_id: Uuid,
    config: &ImuConfig,
) -> Vec<DataPoint> {
    let mut points = Vec::new();
    let now = chrono::DateTime::from_timestamp(data.timestamp, 0)
        .unwrap_or_else(chrono::Utc::now);

    // 计算加速度模
    let accel_mag = state.calc_accel_magnitude(&data);
    
    // 更新历史
    state.update_history(accel_mag);

    // 基础传感器数据点
    let sensor_payload = serde_json::json!({
        "accel": { "x": data.accel_x, "y": data.accel_y, "z": data.accel_z, "magnitude": accel_mag },
        "gyro": { "x": data.gyro_x, "y": data.gyro_y, "z": data.gyro_z },
        "battery": data.battery,
    });

    points.push(DataPoint {
        time: now,
        device_id: Some(device_id),
        patient_id: None,
        data_type: "imu_sensor".to_string(),
        data_category: DataCategory::Metric,
        value_numeric: Some(accel_mag),
        value_text: None,
        severity: None,
        status: data.battery.map(|b| crate::core::entity::EventStatus::Active),
        payload: sensor_payload,
        source: "imu_mqtt".to_string(),
    });

    // 跌倒检测算法
    let variance = state.calc_variance();
    
    // 1. 冲击检测（高加速度）
    if accel_mag > config.fall_threshold && !state.in_fall_state {
        state.in_fall_state = true;
        state.static_since = None;
        
        points.push(DataPoint {
            time: now,
            device_id: Some(device_id),
            patient_id: None,
            data_type: "imu_fall_impact".to_string(),
            data_category: DataCategory::Event,
            value_numeric: Some(accel_mag),
            value_text: Some(format!("检测到高加速度冲击: {:.2} m/s²", accel_mag)),
            severity: Some(Severity::Alert),
            status: Some(crate::core::entity::EventStatus::Active),
            payload: serde_json::json!({"accel_magnitude": accel_mag}),
            source: "imu_mqtt".to_string(),
        });
    }

    // 2. 静止检测（跌倒后通常会静止）
    if state.in_fall_state {
        if variance < config.static_threshold {
            // 进入静止状态
            if state.static_since.is_none() {
                state.static_since = Some(now);
            } else if let Some(since) = state.static_since {
                let duration = now.signed_duration_since(since);
                if duration.num_seconds() >= 2 {
                    // 持续静止2秒，确认跌倒
                    points.push(DataPoint {
                        time: now,
                        device_id: Some(device_id),
                        patient_id: None,
                        data_type: "imu_fall_confirmed".to_string(),
                        data_category: DataCategory::Event,
                        value_numeric: Some(accel_mag),
                        value_text: Some("跌倒事件确认".to_string()),
                        severity: Some(Severity::Alert),
                        status: Some(crate::core::entity::EventStatus::Active),
                        payload: serde_json::json!({
                            "impact_accel": accel_mag,
                            "static_duration_sec": duration.num_seconds(),
                        }),
                        source: "imu_mqtt".to_string(),
                    });
                    
                    state.in_fall_state = false;
                    state.static_since = None;
                }
            }
        } else {
            // 恢复活动，取消跌倒状态
            if state.static_since.is_some() {
                state.static_since = None;
                points.push(DataPoint {
                    time: now,
                    device_id: Some(device_id),
                    patient_id: None,
                    data_type: "imu_fall_cancelled".to_string(),
                    data_category: DataCategory::Event,
                    value_numeric: Some(accel_mag),
                    value_text: Some("跌倒状态取消 - 检测到活动".to_string()),
                    severity: Some(Severity::Info),
                    status: Some(crate::core::entity::EventStatus::Active),
                    payload: serde_json::json!({"variance": variance}),
                    source: "imu_mqtt".to_string(),
                });
            }
            state.in_fall_state = false;
        }
    }

    // 3. 低电量警告
    if let Some(battery) = data.battery {
        if battery < 20 {
            points.push(DataPoint {
                time: now,
                device_id: Some(device_id),
                patient_id: None,
                data_type: "imu_low_battery".to_string(),
                data_category: DataCategory::Event,
                value_numeric: Some(battery as f64),
                value_text: Some(format!("IMU设备电量低: {}%", battery)),
                severity: Some(Severity::Warning),
                status: Some(crate::core::entity::EventStatus::Active),
                payload: serde_json::json!({"battery": battery}),
                source: "imu_mqtt".to_string(),
            });
        }
    }

    state.last_activity = Some(now);
    points
}

/// 解析或创建设备
async fn resolve_or_create_device(pool: &PgPool, device_id_str: &str) -> AppResult<Uuid> {
    use crate::repository::DeviceRepository;
    use crate::core::entity::NewDevice;

    let repo = DeviceRepository::new(pool);
    
    if let Some(device) = repo.find_by_serial(device_id_str).await? {
        return Ok(device.id);
    }
    
    let new_device = NewDevice {
        serial_number: device_id_str.to_string(),
        device_type: "imu_sensor".to_string(),
        status: "active".to_string(),
        firmware_version: None,
        metadata: Some(serde_json::json!({
            "capabilities": ["fall_detection", "activity_monitoring"],
            "sensors": ["accelerometer", "gyroscope"]
        })),
    };
    
    let device = repo.insert(&new_device).await?;
    log::info!("自动注册IMU设备: {} -> {}", device_id_str, device.id);
    Ok(device.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imu_data() {
        let payload = br#"{
            "timestamp": 1704067200,
            "accelerometer": {"x": 0.1, "y": 0.2, "z": 9.8},
            "gyroscope": {"x": 0.0, "y": 0.0, "z": 0.1},
            "battery": 85
        }"#;

        let data = parse_imu_data(payload, "imu_001").unwrap();
        assert_eq!(data.accel_z, 9.8);
        assert_eq!(data.battery, Some(85));
    }

    #[test]
    fn test_fall_detection() {
        let config = ImuConfig::default();
        let mut state = ImuState::new();
        let device_id = Uuid::new_v4();

        // 模拟跌倒冲击（高加速度）
        let fall_data = ImuData {
            device_id: "test".to_string(),
            timestamp: 1704067200,
            accel_x: 20.0,
            accel_y: 15.0,
            accel_z: 5.0,
            gyro_x: 0.0,
            gyro_y: 0.0,
            gyro_z: 0.0,
            battery: Some(80),
        };

        let points = process_imu_data(fall_data, &mut state, device_id, &config);
        
        // 应该生成冲击事件
        assert!(points.iter().any(|p| p.data_type == "imu_fall_impact"));
        assert!(state.in_fall_state);
    }
}
