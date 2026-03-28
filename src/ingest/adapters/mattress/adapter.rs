//! 智能床垫适配器
//!
//! 无状态设计：仅负责 TCP 包解析和验证，状态由 DeviceManager 管理

use crate::core::value_object::DeviceTypeId;
use crate::errors::{AppError, AppResult};
use crate::ingest::framework::{AdapterOutput, DeviceAdapter, DeviceMetadata, MessagePayload};
use chrono::Utc;
use crc::{Crc, CRC_8_SMBUS};

pub struct MattressAdapter;

impl MattressAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 解析 TCP 数据包
    pub fn parse_packet(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 最少需要 4 字节头部
        if raw.len() < 4 {
            return Err(AppError::ValidationError("包长度不足以包含头部".into()));
        }

        // Magic 校验
        if raw[0] != 0xab || raw[1] != 0xcd {
            return Err(AppError::ValidationError("无效的包头 magic".into()));
        }

        let data_len = raw[2] as usize;
        if raw.len() < 4 + data_len {
            return Err(AppError::ValidationError("包体长度不足".into()));
        }

        // CRC 校验
        let expected_crc = raw[3];
        let data_bytes = &raw[4..4 + data_len];
        let crc = Crc::<u8>::new(&CRC_8_SMBUS);
        let computed = crc.checksum(data_bytes);
        if computed != expected_crc {
            return Err(AppError::ValidationError(format!(
                "CRC 校验失败: 期望={}, 实际={}",
                expected_crc, computed
            )));
        }

        // MessagePack 解码
        let value: serde_json::Value = rmp_serde::from_slice(data_bytes)
            .map_err(|e| AppError::ValidationError(format!("MessagePack 解析失败: {}", e)))?;

        Ok(value)
    }

    /// 提取字段（兼容大小写）
    fn get_field<'a>(obj: &'a serde_json::Value, keys: &[&str]) -> Option<&'a serde_json::Value> {
        for &k in keys {
            if let Some(v) = obj.get(k) {
                return Some(v);
            }
        }
        None
    }

    /// 验证数据包
    fn validate_payload(&self, payload: &serde_json::Value) -> AppResult<()> {
        // 验证制造商
        let manufacturer = Self::get_field(payload, &["manufacturer", "Ma"])
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ValidationError("缺少制造商".into()))?;

        if manufacturer != "HT" {
            return Err(AppError::ValidationError("不支持的制造商".into()));
        }

        // 验证型号
        let model = Self::get_field(payload, &["model", "Mo"])
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ValidationError("缺少型号".into()))?;

        if !matches!(model, "02" | "03") {
            return Err(AppError::ValidationError("不支持的型号".into()));
        }

        // 验证序列号
        let serial = Self::get_field(payload, &["serial_number", "Sn"])
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ValidationError("缺少序列号".into()))?;

        if serial.len() < 4 {
            return Err(AppError::ValidationError("序列号格式错误".into()));
        }

        Ok(())
    }
}

impl DeviceAdapter for MattressAdapter {
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: DeviceTypeId::new(DeviceTypeId::SMART_MATTRESS),
            display_name: "智能床垫".to_string(),
            description: "智能床垫设备适配器".to_string(),
            supported_data_types: vec!["smart_mattress".to_string()],
            protocol_version: "1.0".to_string(),
        }
    }

    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        let payload = self.parse_packet(raw)?;

        // 验证
        self.validate_payload(&payload)?;

        // 提取字段
        let heart_rate = Self::get_field(&payload, &["heart_rate", "Hb"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let breath_rate = Self::get_field(&payload, &["breath_rate", "Br"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let wet_status = Self::get_field(&payload, &["wet_status", "Wt"])
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let apnea_count = Self::get_field(&payload, &["apnea_count", "Od"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let weight_value = Self::get_field(&payload, &["weight_value", "We"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let position: Option<Vec<i32>> = Self::get_field(&payload, &["position", "P"])
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_i64())
                    .map(|v| v as i32)
                    .collect()
            });

        // 检测状态（简化版：离床判断）
        let is_on_bed = weight_value > 10;

        // 检测异常
        let (message_type, severity) = if !is_on_bed {
            ("bed_exit".to_string(), Some("info".to_string()))
        } else if heart_rate > 0 && (heart_rate < 40 || heart_rate > 150) {
            (
                "heart_rate_abnormal".to_string(),
                Some("warning".to_string()),
            )
        } else if apnea_count > 10 {
            ("apnea_alert".to_string(), Some("critical".to_string()))
        } else if wet_status {
            ("moisture_alert".to_string(), Some("warning".to_string()))
        } else {
            ("measurement".to_string(), None)
        };

        let payload_json = serde_json::json!({
            "heart_rate": heart_rate,
            "breath_rate": breath_rate,
            "wet_status": wet_status,
            "apnea_count": apnea_count,
            "weight_value": weight_value,
            "position": position,
            "is_on_bed": is_on_bed,
        });

        let msg = MessagePayload {
            time: Utc::now(),
            data_type: "smart_mattress".to_string(),
            message_type: Some(message_type),
            severity,
            payload: payload_json,
        };

        Ok(AdapterOutput::Messages(vec![msg]))
    }

    fn validate(&self, output: &AdapterOutput) -> AppResult<()> {
        match output {
            AdapterOutput::Messages(msgs) => {
                if msgs.is_empty() {
                    return Err(AppError::ValidationError("空消息".into()));
                }
                for msg in msgs {
                    // 基本验证
                    if msg.data_type != "smart_mattress" {
                        return Err(AppError::ValidationError("无效的数据类型".into()));
                    }
                }
                Ok(())
            }
            AdapterOutput::Empty => Ok(()),
        }
    }
}

impl Default for MattressAdapter {
    fn default() -> Self {
        Self::new()
    }
}
