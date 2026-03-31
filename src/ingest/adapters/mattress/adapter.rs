//! 智能床垫适配器
//!
//! 使用 MattressEventEngine 进行完整的事件检测和分析

use crate::core::value_object::DeviceType;
use crate::errors::{AppError, AppResult};
use crate::ingest::framework::{AdapterOutput, DeviceAdapter, DeviceMetadata, MessagePayload};
use chrono::Utc;
use crc::{Crc, CRC_8_SMBUS};

use super::event_engine::{DeviceState, MattressEventEngine};
use super::types::{AlertLevel, MattressData, MattressEvent};

pub struct MattressAdapter {
    engine: MattressEventEngine,
}

impl MattressAdapter {
    pub fn new() -> Self {
        Self {
            engine: MattressEventEngine::new(),
        }
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

    /// 将 JSON payload 转换为 MattressData
    fn to_mattress_data(&self, payload: &serde_json::Value) -> AppResult<MattressData> {
        let manufacturer = Self::get_field(payload, &["manufacturer", "Ma"])
            .and_then(|v| v.as_str())
            .unwrap_or("HT")
            .to_string();

        let model = Self::get_field(payload, &["model", "Mo"])
            .and_then(|v| v.as_str())
            .unwrap_or("02")
            .to_string();

        let version = Self::get_field(payload, &["version", "V"])
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        let serial_number = Self::get_field(payload, &["serial_number", "Sn"])
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let firmware_version = Self::get_field(payload, &["firmware_version", "fv"])
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        let status = Self::get_field(payload, &["status", "St"])
            .and_then(|v| v.as_str())
            .unwrap_or("normal")
            .to_string();

        let heart_rate = Self::get_field(payload, &["heart_rate", "Hb"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let breath_rate = Self::get_field(payload, &["breath_rate", "Br"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let wet_status = Self::get_field(payload, &["wet_status", "Wt"])
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let apnea_count = Self::get_field(payload, &["apnea_count", "Od"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let weight_value = Self::get_field(payload, &["weight_value", "We"])
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let position = Self::get_field(payload, &["position", "P"])
            .and_then(|v| v.as_array())
            .map(|arr| {
                let mut pos = [0i32; 2];
                if let Some(x) = arr.get(0).and_then(|v| v.as_i64()) {
                    pos[0] = x as i32;
                }
                if let Some(y) = arr.get(1).and_then(|v| v.as_i64()) {
                    pos[1] = y as i32;
                }
                pos
            })
            .unwrap_or([0, 0]);

        Ok(MattressData {
            manufacturer,
            model,
            version,
            serial_number,
            firmware_version,
            status,
            heart_rate,
            breath_rate,
            wet_status,
            apnea_count,
            weight_value,
            position,
        })
    }

    /// 将 MattressEvent 转换为 MessagePayload
    fn event_to_payload(
        &self,
        event: &MattressEvent,
    ) -> (String, Option<String>, Option<String>, serde_json::Value) {
        match event {
            MattressEvent::BedEntry {
                timestamp,
                confidence,
                weight_value,
            } => (
                "bed_entry".to_string(),
                None,
                None,
                serde_json::json!({
                    "event_type": "bed_entry",
                    "timestamp": timestamp,
                    "confidence": confidence,
                    "weight_value": weight_value,
                }),
            ),
            MattressEvent::BedExit {
                timestamp,
                confidence,
                duration_minutes,
            } => (
                "bed_exit".to_string(),
                Some("info".to_string()),
                None,
                serde_json::json!({
                    "event_type": "bed_exit",
                    "timestamp": timestamp,
                    "confidence": confidence,
                    "duration_minutes": duration_minutes,
                }),
            ),
            MattressEvent::SignificantMovement {
                timestamp,
                intensity,
                position_change,
                score,
            } => (
                "significant_movement".to_string(),
                Some("info".to_string()),
                None,
                serde_json::json!({
                    "event_type": "significant_movement",
                    "timestamp": timestamp,
                    "intensity": intensity,
                    "position_change": position_change,
                    "score": score,
                }),
            ),
            MattressEvent::VitalSignsAnomaly {
                timestamp,
                heart_rate,
                heart_rate_level,
                breath_rate,
                breath_rate_level,
                anomaly_type,
            } => {
                let severity = match (heart_rate_level, breath_rate_level) {
                    (AlertLevel::Critical, _) | (_, AlertLevel::Critical) => "alert",
                    (AlertLevel::Warning, _) | (_, AlertLevel::Warning) => "warning",
                    _ => "info",
                };
                (
                    anomaly_type.clone(),
                    Some(severity.to_string()),
                    None,
                    serde_json::json!({
                        "event_type": "vital_signs_anomaly",
                        "timestamp": timestamp,
                        "heart_rate": heart_rate,
                        "heart_rate_level": format!("{:?}", heart_rate_level),
                        "breath_rate": breath_rate,
                        "breath_rate_level": format!("{:?}", breath_rate_level),
                        "anomaly_type": anomaly_type,
                    }),
                )
            }
            MattressEvent::ApneaEvent {
                timestamp,
                duration_seconds,
                severity,
                apnea_count,
            } => {
                let sev = match severity {
                    AlertLevel::Critical => "alert",
                    AlertLevel::Warning => "warning",
                    _ => "info",
                };
                (
                    "apnea_event".to_string(),
                    Some(sev.to_string()),
                    None,
                    serde_json::json!({
                        "event_type": "apnea_event",
                        "timestamp": timestamp,
                        "duration_seconds": duration_seconds,
                        "severity": format!("{:?}", severity),
                        "apnea_count": apnea_count,
                    }),
                )
            }
            MattressEvent::MoistureAlert {
                timestamp,
                wet_status,
                duration_minutes,
                severity,
            } => {
                let sev = match severity {
                    AlertLevel::Critical => "alert",
                    AlertLevel::Warning => "warning",
                    _ => "info",
                };
                (
                    "moisture_alert".to_string(),
                    Some(sev.to_string()),
                    None,
                    serde_json::json!({
                        "event_type": "moisture_alert",
                        "timestamp": timestamp,
                        "wet_status": wet_status,
                        "duration_minutes": duration_minutes,
                        "severity": format!("{:?}", severity),
                    }),
                )
            }
            MattressEvent::ScheduledMeasurement {
                timestamp,
                heart_rate,
                breath_rate,
                apnea_count,
                wet_status,
                weight_value,
                measurement_reason,
            } => (
                "scheduled_measurement".to_string(),
                None,
                None,
                serde_json::json!({
                    "event_type": "scheduled_measurement",
                    "timestamp": timestamp,
                    "heart_rate": heart_rate,
                    "breath_rate": breath_rate,
                    "apnea_count": apnea_count,
                    "wet_status": wet_status,
                    "weight_value": weight_value,
                    "measurement_reason": measurement_reason,
                }),
            ),
        }
    }
}

impl DeviceAdapter for MattressAdapter {
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: DeviceType::SmartMattress,
            display_name: "智能床垫".to_string(),
            description: "智能床垫设备适配器（支持完整事件检测）".to_string(),
            supported_data_types: vec![
                "smart_mattress".to_string(),
                "bed_entry".to_string(),
                "bed_exit".to_string(),
                "significant_movement".to_string(),
                "vital_signs_anomaly".to_string(),
                "apnea_event".to_string(),
                "moisture_alert".to_string(),
                "scheduled_measurement".to_string(),
            ],
            protocol_version: "2.0".to_string(),
        }
    }

    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        let payload = self.parse_packet(raw)?;

        // 验证
        self.validate_payload(&payload)?;

        // 转换为 MattressData
        let data = self.to_mattress_data(&payload)?;

        // 使用事件引擎处理（注意：这里需要 DeviceState，但在无状态设计中我们创建临时状态）
        let mut temp_state = DeviceState::new();
        let events = self.engine.process(&mut temp_state, &data)?;

        // 生成指标数据（定时采集）
        let is_on_bed = data.weight_value > 10;
        let metric_payload = serde_json::json!({
            "heart_rate": data.heart_rate,
            "breath_rate": data.breath_rate,
            "wet_status": data.wet_status,
            "apnea_count": data.apnea_count,
            "weight_value": data.weight_value,
            "position": data.position,
            "is_on_bed": is_on_bed,
            "status": data.status,
        });

        let mut messages = vec![MessagePayload {
            time: Utc::now(),
            data_type: "smart_mattress".to_string(),
            message_type: Some("measurement".to_string()),
            severity: None,
            payload: metric_payload,
        }];

        // 添加事件消息
        for event in events {
            let (event_type, severity, _status, event_payload) = self.event_to_payload(&event);
            messages.push(MessagePayload {
                time: Utc::now(),
                data_type: event_type,
                message_type: Some("event".to_string()),
                severity,
                payload: event_payload,
            });
        }

        Ok(AdapterOutput::Messages(messages))
    }

    fn validate(&self, output: &AdapterOutput) -> AppResult<()> {
        match output {
            AdapterOutput::Messages(msgs) => {
                if msgs.is_empty() {
                    return Err(AppError::ValidationError("空消息".into()));
                }
                for msg in msgs {
                    // 基本验证
                    if msg.data_type.is_empty() {
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
