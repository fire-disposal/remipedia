//! 跌倒检测器适配器模块
//!
//! ## MQTT协议规范
//!
//! ### 主题格式
//! - 上行事件: `{prefix}/{serial_number}/event`
//!
//! ### 消息格式 (JSON)
//! ```json
//! {
//!     "event_type": "person_fall" | "person_still" | "person_enter" | "person_leave",
//!     "confidence": 0.85,
//!     "timestamp": "2024-01-15T10:30:00Z"  // 可选，RFC3339格式
//! }
//! ```
//!
//! ### 事件类型说明
//! - `person_fall`: 人物跌倒 - 需要告警，置信度需>=0.5
//! - `person_still`: 人物静止 - 状态监测
//! - `person_enter`: 人物进入 - 区域监测
//! - `person_leave`: 人物离开 - 区域监测
//!
//! ### 自动注册
//! 设备首次发送事件时，系统根据Topic中的serial_number自动注册设备。

mod adapter;
mod types;

pub use adapter::FallDetectorAdapter;
pub use types::{FallAlertEvent, FallDetectorData, FallDetectorEventType, FallDetectorMessage};