use rocket::serde::json::Json;
use rocket::{get, State};
use utoipa::ToSchema;

use crate::config::MqttConfig;

#[derive(Debug, Clone, rocket::serde::Serialize, ToSchema)]
pub struct MqttProtocolDoc {
    pub protocol: String,
    pub version: String,
    pub topic_pattern: String,
    pub qos: String,
    pub payload_required_fields: Vec<String>,
    pub payload_optional_fields: Vec<String>,
    pub sample_topic: String,
    pub sample_payload: serde_json::Value,
    pub broker_recommendation: BrokerRecommendation,
}

#[derive(Debug, Clone, rocket::serde::Serialize, ToSchema)]
pub struct BrokerRecommendation {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub keep_alive_seconds: u16,
    pub note: String,
}

/// MQTT 内部协议说明
#[utoipa::path(
    get,
    path = "/ingest/mqtt/protocol",
    tag = "ingest",
    responses(
        (status = 200, description = "MQTT 协议文档", body = MqttProtocolDoc),
    )
)]
#[get("/ingest/mqtt/protocol")]
pub async fn mqtt_protocol_doc(mqtt_config: &State<MqttConfig>) -> Json<MqttProtocolDoc> {
    let topic_pattern = if mqtt_config.topic_prefix.is_empty() {
        "devices/{serial_number}/{device_type}".to_string()
    } else {
        format!(
            "{}/devices/{{serial_number}}/{{device_type}}",
            mqtt_config.topic_prefix
        )
    };

    let sample_topic = topic_pattern
        .replace("{serial_number}", "SN-001")
        .replace("{device_type}", "heart_rate_monitor");

    Json(MqttProtocolDoc {
        protocol: "mqtt".to_string(),
        version: "v2".to_string(),
        topic_pattern,
        qos: "at_least_once (QoS1)".to_string(),
        payload_required_fields: vec![
            "timestamp (RFC3339)".to_string(),
            "value 或 data".to_string(),
        ],
        payload_optional_fields: vec![
            "device_type (topic 未提供时建议提供)".to_string(),
            "serial_number / sn".to_string(),
            "metadata".to_string(),
        ],
        sample_topic,
        sample_payload: serde_json::json!({
            "timestamp": "2026-04-02T00:00:00Z",
            "device_type": "heart_rate_monitor",
            "value": 72,
            "metadata": {
                "firmware": "1.0.0"
            }
        }),
        broker_recommendation: BrokerRecommendation {
            host: mqtt_config.broker.clone(),
            port: mqtt_config.port,
            client_id: mqtt_config.client_id.clone(),
            keep_alive_seconds: 30,
            note: "生产环境建议禁用匿名访问、启用用户名密码与 TLS。".to_string(),
        },
    })
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![mqtt_protocol_doc]
}
