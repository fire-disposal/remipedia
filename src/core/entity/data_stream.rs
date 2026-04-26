use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 数据流类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataStreamType {
    /// 数值型指标流（心率、血氧、加速度等）
    Metric,
    /// 事件/告警流（跌倒检测、低电量等）
    Event,
}

impl std::fmt::Display for DataStreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Metric => write!(f, "metric"),
            Self::Event => write!(f, "event"),
        }
    }
}

impl std::str::FromStr for DataStreamType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "metric" => Ok(Self::Metric),
            "event" => Ok(Self::Event),
            _ => Err(format!("未知数据流类型: {}", s)),
        }
    }
}

/// 逻辑数据源：设备无关的数据流抽象
///
/// 一个 DataStream 对应一个设备+数据类型的组合。
/// 数据查询时通过 stream_id 而非 device_id，实现设备与数据的解耦。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DataStream {
    pub id: Uuid,
    pub name: String,
    pub stream_type: String, // "metric" | "event" — 数据库存储字符串
    pub data_type: String,
    pub device_id: Option<Uuid>,
    pub patient_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DataStream {
    /// 创建新的 DataStream 实例（用于 INSERT 前构建）
    pub fn new(
        name: String,
        stream_type: DataStreamType,
        data_type: String,
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            stream_type: stream_type.to_string(),
            data_type,
            device_id,
            patient_id,
            metadata: serde_json::Value::Object(Default::default()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 获取流类型的枚举表示
    pub fn stream_type_enum(&self) -> Option<DataStreamType> {
        self.stream_type.parse().ok()
    }

    /// 是否为指标流
    pub fn is_metric(&self) -> bool {
        self.stream_type == "metric"
    }

    /// 是否为事件流
    pub fn is_event(&self) -> bool {
        self.stream_type == "event"
    }
}

/// 观测数据点：与 DataStream 关联的数值型指标
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Observation {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub patient_id: Uuid,
    pub value_numeric: Option<rust_decimal::Decimal>,
    pub value_text: Option<String>,
    pub metadata: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}

/// 创建观测数据的请求结构
#[derive(Debug, Clone)]
pub struct NewObservation {
    pub stream_id: Uuid,
    pub patient_id: Uuid,
    pub value_numeric: Option<rust_decimal::Decimal>,
    pub value_text: Option<String>,
    pub metadata: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}
