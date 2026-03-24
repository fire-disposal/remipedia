//! 跌倒检测器类型定义

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 跌倒检测事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FallDetectorEventType {
    /// 人物跌倒
    PersonFall,
    /// 人物静止
    PersonStill,
    /// 人物进入
    PersonEnter,
    /// 人物离开
    PersonLeave,
}

impl FallDetectorEventType {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "person_fall" => Some(Self::PersonFall),
            "person_still" => Some(Self::PersonStill),
            "person_enter" => Some(Self::PersonEnter),
            "person_leave" => Some(Self::PersonLeave),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PersonFall => "person_fall",
            Self::PersonStill => "person_still",
            Self::PersonEnter => "person_enter",
            Self::PersonLeave => "person_leave",
        }
    }

    /// 是否为告警事件（跌倒需要告警）
    pub fn is_alert(&self) -> bool {
        matches!(self, Self::PersonFall)
    }
}

/// 跌倒检测事件消息（MQTT上行消息格式）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FallDetectorMessage {
    /// 事件类型
    pub event_type: FallDetectorEventType,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
    /// 事件时间戳 (RFC3339格式，可选，不提供则使用服务器时间)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// 解析后的跌倒检测数据
#[derive(Debug, Clone)]
pub struct FallDetectorData {
    /// 事件类型
    pub event_type: FallDetectorEventType,
    /// 置信度
    pub confidence: f32,
    /// 事件时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 跌倒告警事件（用于存储和通知）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallAlertEvent {
    /// 事件类型
    pub event_type: FallDetectorEventType,
    /// 置信度
    pub confidence: f32,
    /// 事件时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 严重程度 (low, medium, high)
    pub severity: String,
}