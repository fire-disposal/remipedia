//! 跌倒检测器适配器模块
//!
//! ## MQTT 协议
//! - 主题：`{prefix}/{serial_number}/event`
//! - 消息：
//! ```json
//! {
//!   "event_type": "person_fall" | "person_still" | "person_enter" | "person_leave",
//!   "timestamp": "2024-01-15T10:30:00Z",
//!   "details": {"any":"json"}
//! }
//! ```
//! - 本适配器仅做数据接入，不包含置信度判定逻辑。

mod adapter;
mod types;

pub use adapter::FallDetectorAdapter;
pub use types::{FallDetectorData, FallDetectorEventType, FallDetectorMessage};
