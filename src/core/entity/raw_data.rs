use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawIngestStatus {
    Stored,
    Ingested,
    Ignored,
    FormatError,
    ProcessingError,
}

impl std::fmt::Display for RawIngestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stored => write!(f, "stored"),
            Self::Ingested => write!(f, "ingested"),
            Self::Ignored => write!(f, "ignored"),
            Self::FormatError => write!(f, "format_error"),
            Self::ProcessingError => write!(f, "processing_error"),
        }
    }
}

impl std::str::FromStr for RawIngestStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stored" => Ok(Self::Stored),
            "ingested" => Ok(Self::Ingested),
            "ignored" => Ok(Self::Ignored),
            "format_error" => Ok(Self::FormatError),
            "processing_error" => Ok(Self::ProcessingError),
            _ => Err(format!("未知 ingest 状态: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RawDataRecord {
    pub id: Uuid,
    pub source: String,
    pub serial_number: Option<String>,
    pub device_type: Option<String>,
    pub remote_addr: Option<String>,
    pub metadata: serde_json::Value,
    pub raw_payload: Vec<u8>,
    pub raw_payload_text: Option<String>,
    pub status: String,
    pub status_message: Option<String>,
    pub received_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct RawDataQuery {
    pub source: Option<String>,
    pub serial_number: Option<String>,
    pub device_type: Option<String>,
    pub status: Option<RawIngestStatus>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub page: u32,
    pub page_size: u32,
}
