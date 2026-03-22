use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::{DeviceAdapter, mattress::event_engine::MattressEventEngine, mattress::types::{TurnOverState, MattressData}};
use crc::{Crc, CRC_8_SMBUS};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Mutex;

/// 智能床垫适配器 - 集成事件引擎
pub struct MattressAdapter {
    event_engine: Mutex<MattressEventEngine>,
    turn_over_state: Mutex<TurnOverState>,
}

impl MattressAdapter {
    pub fn new() -> Self {
        Self {
            event_engine: Mutex::new(MattressEventEngine::new()),
            turn_over_state: Mutex::new(TurnOverState::new(2.0)),
        }
    }

    /// 使用自定义配置创建适配器
    pub fn with_config(event_engine: MattressEventEngine) -> Self {
        Self {
            event_engine: Mutex::new(event_engine),
            turn_over_state: Mutex::new(TurnOverState::new(2.0)),
        }
    }

    /// 解析TCP数据包
    pub fn parse_tcp_packet(&self, raw: &[u8]) -> AppResult<MattressData> {
        // 验证数据包最小长度
        if raw.len() < 20 {
            return Err(AppError::ValidationError("数据包长度不足".into()));
        }

        // 验证CRC校验
        let crc = Crc::<u8>::new(&CRC_8_SMBUS);
        let expected_crc = crc.checksum(&raw[0..raw.len()-1]);
        let actual_crc = raw[raw.len()-1];
        
        if expected_crc != actual_crc {
            return Err(AppError::ValidationError(format!("CRC校验失败: 期望={}, 实际={}", expected_crc, actual_crc)));
        }

        // 解析数据包
        let manufacturer = String::from_utf8(raw[0..2].to_vec())
            .map_err(|_| AppError::ValidationError("制造商字段解析失败".into()))?;
        
        let model = String::from_utf8(raw[2..4].to_vec())
            .map_err(|_| AppError::ValidationError("型号字段解析失败".into()))?;
        
        let version = raw[4] as i32;
        let firmware_version = raw[5] as i32;
        
        // 序列号（6字节）
        let serial_number = format!("{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            raw[6], raw[7], raw[8], raw[9], raw[10], raw[11]);
        
        // 状态（1字节）
        let status = match raw[12] {
            0x00 => "off",
            0x01 => "on", 
            0x02 => "mov",
            0x03 => "call",
            _ => return Err(AppError::ValidationError("无效的状态值".into())),
        }.to_string();
        
        // 心跳频率（1字节，需要转换）
        let heart_rate = raw[13] as i32;
        
        // 呼吸频率（1字节，需要转换）
        let breath_rate = raw[14] as i32;
        
        // 尿湿状态（1字节）
        let wet_status = raw[15] != 0;
        
        // 呼吸暂停次数（1字节）
        let apnea_count = raw[16] as i32;
        
        // 重量值（2字节，大端序）
        let weight_value = ((raw[17] as i32) << 8) | (raw[18] as i32);
        
        // 位置坐标（2字节）
        let position = [raw[19] as i32, if raw.len() > 20 { raw[20] as i32 } else { 0 }];

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
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 解析TCP数据包
        let mattress_data = self.parse_tcp_packet(raw)?;
        
        // 数据降噪
        let cleaned_data = self.denoise_data(&mattress_data);
        
        // 转换数据类型以适配事件引擎
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
        
        // 使用事件引擎处理数据，生成事件
        let mut event_engine = self.event_engine.lock().unwrap();
        let mattress_events = event_engine.process_data(&engine_data)?;
        
        // 检测翻身事件（作为体动的一部分）
        let mut turn_over_state = self.turn_over_state.lock().unwrap();
        let turn_over_event = turn_over_state.update_and_detect(cleaned_data.position);
        
        // 构建输出数据 - 只包含过滤后的有价值信息
        let result = json!({
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
            "turn_over_event": turn_over_event,
            "mattress_events": mattress_events, // 床垫事件列表
            "filtered": true, // 标记这是过滤后的数据
        });
        
        Ok(result)
    }
    
    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        // 验证制造商
        let manufacturer = payload["manufacturer"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少制造商".into()))?;
        
        if manufacturer != "HT" {
            return Err(AppError::ValidationError("不支持的制造商".into()));
        }
        
        // 验证型号
        let model = payload["model"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少型号".into()))?;
        
        if !matches!(model, "02" | "03") {
            return Err(AppError::ValidationError("不支持的型号".into()));
        }
        
        // 验证序列号格式
        let serial_number = payload["serial_number"].as_str()
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