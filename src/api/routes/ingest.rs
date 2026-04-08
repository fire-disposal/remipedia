use rocket::serde::json::Json;
use rocket::{get, State};
use rocket::http::{ContentType, Header};
use rocket::response::stream::ByteStream;
use sqlx::PgPool;
use utoipa::ToSchema;
use std::io::Cursor;

use crate::api::guards::AuthenticatedUser;
use crate::config::MqttConfig;
use crate::dto::request::RawDataQuery;
use crate::dto::response::RawDataQueryResponse;
use crate::errors::AppResult;
use crate::service::IngestRawService;

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

/// 查询 ingest 原始归档数据
#[utoipa::path(
    get,
    path = "/ingest/raw",
    tag = "ingest",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("source" = Option<String>, Query, description = "来源筛选"),
        ("serial_number" = Option<String>, Query, description = "序列号筛选"),
        ("device_type" = Option<String>, Query, description = "设备类型筛选"),
        ("status" = Option<String>, Query, description = "状态筛选"),
        ("start_time" = Option<String>, Query, description = "开始时间 (RFC3339, received_at)"),
        ("end_time" = Option<String>, Query, description = "结束时间 (RFC3339, received_at)"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = RawDataQueryResponse),
    )
)]
#[get("/ingest/raw?<source>&<serial_number>&<device_type>&<status>&<start_time>&<end_time>&<page>&<page_size>")]
pub async fn query_raw_data(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    source: Option<String>,
    serial_number: Option<String>,
    device_type: Option<String>,
    status: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<RawDataQueryResponse>> {
    let service = IngestRawService::new(pool);

    let query = RawDataQuery {
        source,
        serial_number,
        device_type,
        status,
        start_time: start_time.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        end_time: end_time.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    };

    let response = service.query(query).await?;
    Ok(Json(response))
}

/// 导出原始数据
#[utoipa::path(
    get,
    path = "/ingest/raw/export",
    tag = "ingest",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("source" = Option<String>, Query, description = "来源筛选"),
        ("serial_number" = Option<String>, Query, description = "序列号筛选"),
        ("device_type" = Option<String>, Query, description = "设备类型筛选"),
        ("status" = Option<String>, Query, description = "状态筛选"),
        ("start_time" = Option<String>, Query, description = "开始时间 (RFC3339)"),
        ("end_time" = Option<String>, Query, description = "结束时间 (RFC3339)"),
        ("format" = Option<String>, Query, description = "导出格式 (json 或 csv，默认json)"),
    ),
    responses(
        (status = 200, description = "导出成功", body = String),
    )
)]
#[get("/ingest/raw/export?<source>&<serial_number>&<device_type>&<status>&<start_time>&<end_time>&<format>")]
pub async fn export_raw_data(
    pool: &State<PgPool>,
    _user: AuthenticatedUser,
    source: Option<String>,
    serial_number: Option<String>,
    device_type: Option<String>,
    status: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    format: Option<String>,
) -> AppResult<Vec<u8>> {
    let service = IngestRawService::new(pool);

    let query = RawDataQuery {
        source,
        serial_number,
        device_type,
        status,
        start_time: start_time.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        end_time: end_time.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
        page: 1,
        page_size: 10000,
    };

    let response = service.query(query).await?;

    let export_format = format.unwrap_or_else(|| "json".to_string());
    let data = match export_format.to_lowercase().as_str() {
        "csv" => service.export_csv(&response.data)?,
        _ => {
            serde_json::to_vec_pretty(&response.data)
                .map_err(|e| crate::errors::AppError::ValidationError(format!("JSON序列化失败: {}", e)))?
        }
    };

    Ok(data)
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![mqtt_protocol_doc, query_raw_data, export_raw_data]
}
