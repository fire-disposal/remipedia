use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 设备-患者绑定实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Binding {
    pub id: Uuid,
    pub device_id: Uuid,
    pub patient_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 新绑定
#[derive(Debug, Clone)]
pub struct NewBinding {
    pub device_id: Uuid,
    pub patient_id: Uuid,
    pub notes: Option<String>,
}

/// 用户-患者绑定实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPatientBinding {
    pub id: Uuid,
    pub user_id: Uuid,
    pub patient_id: Uuid,
    pub relation: Option<String>,
    pub created_at: DateTime<Utc>,
}