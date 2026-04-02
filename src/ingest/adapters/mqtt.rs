//! MQTT 通用适配器
//!
//! 负责将 MQTT topic + payload 解析为统一的 DataPacket，
//! 便于后续 Pipeline 复用同一处理流程。

use crate::ingest::DataPacket;
use serde_json::Value;

/// MQTT 消息适配器
#[derive(Debug, Clone)]
pub struct MqttAdapter {
    topic_prefix: String,
}

impl MqttAdapter {
    pub fn new(topic_prefix: impl Into<String>) -> Self {
        Self {
            topic_prefix: topic_prefix.into().trim_matches('/').to_string(),
        }
    }

    /// 生成订阅主题
    pub fn subscribe_topic(&self) -> String {
        if self.topic_prefix.is_empty() {
            "devices/+/+".to_string()
        } else {
            format!("{}/devices/+/+", self.topic_prefix)
        }
    }

    /// 将 MQTT 消息转换为 DataPacket
    pub fn to_packet(&self, topic: &str, payload: Vec<u8>) -> Option<DataPacket> {
        let (serial, topic_device_type) = self.parse_topic(topic)?;

        let payload_device_type = Self::extract_device_type(&payload);
        let device_type = topic_device_type
            .or(payload_device_type)
            .unwrap_or_else(|| "unknown".to_string());

        Some(
            DataPacket::new(payload, "mqtt")
                .with_serial(serial)
                .with_device_type(device_type),
        )
    }

    fn parse_topic(&self, topic: &str) -> Option<(String, Option<String>)> {
        let parts: Vec<&str> = topic.split('/').collect();

        // 1) remipedia/devices/{serial}/{type}
        if !self.topic_prefix.is_empty()
            && parts.len() >= 4
            && parts[0] == self.topic_prefix
            && parts[1] == "devices"
        {
            return Some((parts[2].to_string(), Some(parts[3].to_string())));
        }

        // 2) devices/{serial}/{type}
        if parts.len() >= 3 && parts[0] == "devices" {
            return Some((parts[1].to_string(), Some(parts[2].to_string())));
        }

        None
    }

    fn extract_device_type(payload: &[u8]) -> Option<String> {
        serde_json::from_slice::<Value>(payload)
            .ok()
            .and_then(|json| {
                json.get("device_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_topic_with_prefix() {
        let adapter = MqttAdapter::new("remipedia");
        assert_eq!(adapter.subscribe_topic(), "remipedia/devices/+/+");
    }

    #[test]
    fn test_subscribe_topic_without_prefix() {
        let adapter = MqttAdapter::new("");
        assert_eq!(adapter.subscribe_topic(), "devices/+/+");
    }

    #[test]
    fn test_to_packet_from_prefixed_topic() {
        let adapter = MqttAdapter::new("remipedia");
        let packet = adapter
            .to_packet(
                "remipedia/devices/SN123/heart_rate_monitor",
                br#"{"value":72}"#.to_vec(),
            )
            .expect("should parse topic");

        assert_eq!(packet.metadata.serial_number.as_deref(), Some("SN123"));
        assert_eq!(
            packet.metadata.device_type.as_deref(),
            Some("heart_rate_monitor")
        );
    }
}
