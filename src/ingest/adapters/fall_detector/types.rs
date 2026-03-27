//! 跌倒检测器类型定义

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// 跌倒检测事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FallDetectorEventType {
    PersonFall,
    PersonStill,
    PersonEnter,
    PersonLeave,
}

impl FallDetectorEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PersonFall => "person_fall",
            Self::PersonStill => "person_still",
            Self::PersonEnter => "person_enter",
            Self::PersonLeave => "person_leave",
        }
    }
}

/// 跌倒检测事件消息（MQTT 上行）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FallDetectorMessage {
    pub event_type: FallDetectorEventType,
    /// 事件时间戳 (RFC3339, 可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// 透传附加信息（可选）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// 解析后的跌倒检测数据
#[derive(Debug, Clone)]
pub struct FallDetectorData {
    pub event_type: FallDetectorEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: Option<Value>,
}
