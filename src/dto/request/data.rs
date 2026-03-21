use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 数据上报请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReportRequest {
    pub timestamp: Option<DateTime<Utc>>,
    pub device_id: Uuid,
    pub subject_id: Option<Uuid>,
    pub data_type: String,
    pub payload: serde_json::Value,
}

/// 创建绑定请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBindingRequest {
    pub device_id: Uuid,
    pub patient_id: Uuid,
    pub notes: Option<String>,
}

/// 数据查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuery {
    pub device_id: Option<Uuid>,
    pub subject_id: Option<Uuid>,
    pub data_type: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub page: u32,
    pub page_size: u32,
}

impl Default for DataQuery {
    fn default() -> Self {
        Self {
            device_id: None,
            subject_id: None,
            data_type: None,
            start_time: None,
            end_time: None,
            page: 1,
            page_size: 20,
        }
    }
}