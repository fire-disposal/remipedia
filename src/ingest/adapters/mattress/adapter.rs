use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::{AdapterOutput, DeviceAdapter, MessagePayload};
use crate::ingest::adapters::mattress::event_engine::MattressEventEngine;
use crate::ingest::adapters::mattress::types::{MattressData, TurnOverState};
use crc::{Crc, CRC_8_SMBUS};
use chrono::Utc;
use std::sync;

/// 智能床垫适配器 - 集成事件引擎
pub struct MattressAdapter {
    // 直接持有事件引擎实例（线程安全由 Mutex 提供），不再通过兼容 wrapper
    event_engine: std::sync::Arc<std::sync::Mutex<MattressEventEngine>>,
    // 设备/适配器级状态（当前设计为单实例状态，与旧行为一致）
    device_state: std::sync::Arc<std::sync::Mutex<crate::ingest::adapters::mattress::event_engine::DeviceState>>,
    turn_over_state: std::sync::Mutex<TurnOverState>,
}

impl MattressAdapter {
    pub fn new() -> Self {
        Self {
            event_engine: std::sync::Arc::new(std::sync::Mutex::new(MattressEventEngine::new())),
            device_state: std::sync::Arc::new(std::sync::Mutex::new(crate::ingest::adapters::mattress::event_engine::DeviceState::new())),
            turn_over_state: std::sync::Mutex::new(TurnOverState::new(2.0)),
        }
    }

    /// 使用自定义配置创建适配器
    /// 使用已存在的事件引擎服务实例（用于测试或注入）
    pub fn with_service(svc: std::sync::Arc<std::sync::Mutex<MattressEventEngine>>) -> Self {
        Self { event_engine: svc, device_state: std::sync::Arc::new(std::sync::Mutex::new(crate::ingest::adapters::mattress::event_engine::DeviceState::new())), turn_over_state: std::sync::Mutex::new(TurnOverState::new(2.0)) }
    }

    /// 解析TCP数据包
    pub fn parse_tcp_packet(&self, raw: &[u8]) -> AppResult<MattressData> {
        // 最少需要 4 字节头部
        if raw.len() < 4 {
            return Err(AppError::ValidationError("包长度不足以包含头部".into()));
        }

        // Magic 校验 (0xab, 0xcd)
        if raw[0] != 0xab || raw[1] != 0xcd {
            return Err(AppError::ValidationError("无效的包头 magic".into()));
        }

        let data_len = raw[2] as usize;
        if raw.len() < 4 + data_len {
            return Err(AppError::ValidationError("包体长度不足".into()));
        }

        let expected_crc = raw[3];
        let data_bytes = &raw[4..4 + data_len];

        // CRC 校验（对 data 部分计算 CRC_8_SMBUS）
        let crc = Crc::<u8>::new(&CRC_8_SMBUS);
        let computed = crc.checksum(data_bytes);
        if computed != expected_crc {
            return Err(AppError::ValidationError(format!(
                "CRC 校验失败: 期望={}, 实际={}",
                expected_crc, computed
            )));
        }

        // 解包 MessagePack -> serde_json::Value 便于字段映射（字段名大小写兼容）
        let v: serde_json::Value = rmp_serde::from_slice(data_bytes).map_err(|e| {
            AppError::ValidationError(format!("MessagePack 解析失败: {}", e))
        })?;

        // helper：根据可能的大/小写键名获取字符串
        let get_str = |obj: &serde_json::Value, keys: &[&str]| -> Option<String> {
            for &k in keys {
                if let Some(s) = obj.get(k).and_then(|x| x.as_str()) {
                    return Some(s.to_string());
                }
            }
            None
        };

        // 顶层字段
        let manufacturer = get_str(&v, &["Ma", "ma"]).ok_or_else(|| {
            AppError::ValidationError("缺少制造商字段 Ma".into())
        })?;

        let model = get_str(&v, &["Mo", "mo"]).ok_or_else(|| {
            AppError::ValidationError("缺少型号字段 Mo".into())
        })?;

        let version = v.get("V").or_else(|| v.get("v")).and_then(|x| x.as_i64()).unwrap_or(1) as i32;

        let serial_number = get_str(&v, &["Sn", "sn"]).ok_or_else(|| {
            AppError::ValidationError("缺少序列号 Sn".into())
        })?;

        // D 节点
        let d = v.get("D").or_else(|| v.get("d")).ok_or_else(|| {
            AppError::ValidationError("缺少 D 节点".into())
        })?;

        let firmware_version = d.get("fv").and_then(|x| x.as_i64()).unwrap_or(0) as i32;

        let status = get_str(d, &["St", "st"]).ok_or_else(|| AppError::ValidationError("缺少状态 St".into()))?;

        // 对于 Mo=03 型号，有些字段可能为默认值或不可信，需以 St 为主判定
        let heart_rate = d.get("Hb").or_else(|| d.get("hb")).and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let breath_rate = d.get("Br").or_else(|| d.get("br")).and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let wet_status = d.get("Wt").or_else(|| d.get("wt")).and_then(|x| x.as_bool()).unwrap_or(false);
        let apnea_count = d.get("Od").or_else(|| d.get("od")).and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let weight_value = d.get("We").or_else(|| d.get("we")).and_then(|x| x.as_i64()).unwrap_or(-1) as i32;

        let position = d
            .get("P")
            .and_then(|p| p.as_array())
            .and_then(|arr| {
                let a0 = arr.get(0).and_then(|x| x.as_i64()).map(|v| v as i32).unwrap_or(0);
                let a1 = arr.get(1).and_then(|x| x.as_i64()).map(|v| v as i32).unwrap_or(0);
                Some([a0, a1])
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

    /// 数据降噪处理
    pub fn denoise_data(&self, data: &MattressData) -> MattressData {
        let mut cleaned_data = data.clone();

        // 心率降噪：过滤异常值
        if cleaned_data.heart_rate > 0 && cleaned_data.heart_rate < 200 {
            // 使用简单的移动平均或其他降噪算法
            // 这里简化处理，实际可以更复杂
        }

        // 呼吸率降噪：过滤异常值
        if cleaned_data.breath_rate > 0 && cleaned_data.breath_rate < 60 {
            // 降噪处理
        }

        // 重量值降噪
        if cleaned_data.weight_value < 0 {
            cleaned_data.weight_value = 0;
        }

        cleaned_data
    }
}

impl DeviceAdapter for MattressAdapter {
    fn parse(&self, raw: &[u8]) -> AppResult<crate::ingest::adapters::AdapterOutput> {
        // 解析TCP数据包并降噪
        let mattress_data = self.parse_tcp_packet(raw)?;
        let cleaned_data = self.denoise_data(&mattress_data);

        // 统一时间戳，避免同一消息中使用多个不同的 now()
        let now = Utc::now();

        // 转换数据类型以适配事件引擎（尽量避免不必要 clone）
        let engine_data = MattressData {
            manufacturer: cleaned_data.manufacturer.clone(),
            model: cleaned_data.model.clone(),
            version: cleaned_data.version,
            serial_number: cleaned_data.serial_number.clone(),
            firmware_version: cleaned_data.firmware_version,
            status: cleaned_data.status.clone(),
            heart_rate: cleaned_data.heart_rate,
            breath_rate: cleaned_data.breath_rate,
            wet_status: cleaned_data.wet_status,
            apnea_count: cleaned_data.apnea_count,
            weight_value: cleaned_data.weight_value,
            position: cleaned_data.position,
        };

        // 使用事件引擎服务处理数据（阻塞式调用，安全在 spawn_blocking 环境中使用）
        // 直接在当前线程加锁调用引擎的 `process`（适配器已在 spawn_blocking 环境中被调用）
        let mattress_events = {
            let mut eng = self.event_engine.lock().map_err(|_| AppError::ValidationError("event engine 锁被毒化".into()))?;
            let mut st = self.device_state.lock().map_err(|_| AppError::ValidationError("device state 锁被毒化".into()))?;
            eng.process(&mut *st, &engine_data)?
        };

        // 检测翻身事件（作为体动的一部分），短期加锁使用
        let turn_over_event = {
            let mut guard = self
                .turn_over_state
                .lock()
                .map_err(|_| AppError::ValidationError("turn_over_state 锁被毒化".into()))?;
            guard.update_and_detect(cleaned_data.position)
        };

        // 构建精简主负载（不嵌套事件），将事件作为独立消息输出以便索引
        let main_payload = serde_json::json!({
            "manufacturer": cleaned_data.manufacturer,
            "model": cleaned_data.model,
            "version": cleaned_data.version,
            "serial_number": cleaned_data.serial_number,
            "firmware_version": cleaned_data.firmware_version,
            "status": cleaned_data.status,
            "heart_rate": cleaned_data.heart_rate,
            "breath_rate": cleaned_data.breath_rate,
            "wet_status": cleaned_data.wet_status,
            "apnea_count": cleaned_data.apnea_count,
            "weight_value": cleaned_data.weight_value,
            "position": cleaned_data.position,
            "turn_over_detected": turn_over_event.is_some(),
            "filtered": true,
        });

        // 构建扁平 MessagePayload：主时间序列消息
        let mut msgs: Vec<MessagePayload> = Vec::new();

        msgs.push(MessagePayload {
            time: now,
            data_type: "smart_mattress".to_string(),
            message_type: None,
            severity: None,
            payload: main_payload,
        });

        // 将事件引擎生成的事件直接序列化为独立消息（避免在主 payload 内嵌套）
        for ev in mattress_events.into_iter() {
            let ev_value = serde_json::to_value(&ev).unwrap_or(serde_json::json!({}));
            let ev_type = match &ev {
                crate::ingest::adapters::mattress::types::MattressEvent::BedEntry { .. } =>
                    Some("bed_entry".to_string()),
                crate::ingest::adapters::mattress::types::MattressEvent::BedExit { .. } =>
                    Some("bed_exit".to_string()),
                crate::ingest::adapters::mattress::types::MattressEvent::VitalSignsAnomaly { .. } =>
                    Some("vital_signs_anomaly".to_string()),
                crate::ingest::adapters::mattress::types::MattressEvent::ApneaEvent { .. } =>
                    Some("apnea_event".to_string()),
                crate::ingest::adapters::mattress::types::MattressEvent::MoistureAlert { .. } =>
                    Some("moisture_alert".to_string()),
                crate::ingest::adapters::mattress::types::MattressEvent::SignificantMovement { .. } =>
                    Some("significant_movement".to_string()),
                crate::ingest::adapters::mattress::types::MattressEvent::ScheduledMeasurement { .. } =>
                    Some("scheduled_measurement".to_string()),
                _ => None,
            };

            msgs.push(MessagePayload {
                time: now,
                data_type: "mattress_event".to_string(),
                message_type: ev_type,
                severity: None,
                payload: ev_value,
            });
        }

        Ok(AdapterOutput::Messages(msgs))
    }

    fn validate(&self, output: &crate::ingest::adapters::AdapterOutput) -> AppResult<()> {
        // 验证制造商/型号/序列号等（基于生成的时间序列 payload）
        let payload = match output {
            crate::ingest::adapters::AdapterOutput::Messages(msgs) => {
                if let Some(m) = msgs.iter().find(|m| m.data_type == "smart_mattress") {
                    &m.payload
                } else {
                    &msgs
                        .get(0)
                        .ok_or_else(|| AppError::ValidationError("适配器未返回有效 payload".into()))?
                        .payload
                }
            }
        };

        // 验证制造商
        let manufacturer = payload["manufacturer"]
            .as_str()
            .ok_or_else(|| AppError::ValidationError("缺少制造商".into()))?;

        if manufacturer != "HT" {
            return Err(AppError::ValidationError("不支持的制造商".into()));
        }

        // 验证型号
        let model = payload["model"]
            .as_str()
            .ok_or_else(|| AppError::ValidationError("缺少型号".into()))?;

        if !matches!(model, "02" | "03") {
            return Err(AppError::ValidationError("不支持的型号".into()));
        }

        // 验证序列号格式
        let serial_number = payload["serial_number"]
            .as_str()
            .ok_or_else(|| AppError::ValidationError("缺少序列号".into()))?;

        if serial_number.len() != 12 {
            return Err(AppError::ValidationError("序列号格式错误".into()));
        }

        Ok(())
    }

    fn device_type(&self) -> &'static str {
        "smart_mattress"
    }

    fn data_type(&self) -> &'static str {
        "smart_mattress"
    }
}
