use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 时间序列数据实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Datasheet {
    pub time: DateTime<Utc>,
    pub device_id: Uuid,
    pub subject_id: Option<Uuid>,
    pub data_type: String,
    pub payload: serde_json::Value,
    pub source: String,
    pub ingested_at: DateTime<Utc>,
}

/// 数据入库请求
#[derive(Debug, Clone)]
pub struct IngestData {
    pub time: DateTime<Utc>,
    pub device_id: Uuid,
    pub subject_id: Option<Uuid>,
    pub data_type: String,
    pub payload: serde_json::Value,
    pub source: String,
}
