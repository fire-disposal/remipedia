use crate::errors::{AppError, AppResult};
use crate::ingest::adapters::{DeviceAdapter, smart_mattress_filter::SmartMattressFilter};
use chrono::{DateTime, Utc};
use crc::{Crc, CRC_8_SMBUS};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::VecDeque;

/// 智能床垫数据包结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattressData {
    pub manufacturer: String,    // Ma: 制造商
    pub model: String,           // Mo: 型号
    pub version: i32,            // V: 协议版本
    pub serial_number: String,   // Sn: 设备序列号
    pub firmware_version: i32,   // fv: 固件版本
    pub status: String,          // St: 体征状态
    pub heart_rate: i32,         // Hb: 心跳频率
    pub breath_rate: i32,        // Br: 呼吸频率
    pub wet_status: bool,        // Wt: 尿湿状态
    pub apnea_count: i32,        // Od: 呼吸暂停次数
    pub weight_value: i32,       // We: 辅助重量值
    pub position: [i32; 2],      // P: 身体位置坐标 [头部, 胸部]
}

/// 翻身检测状态
#[derive(Debug, Clone)]
pub struct TurnOverState {
    pub previous_positions: VecDeque<[i32; 2]>,
    pub threshold: f32,
}

impl Default for TurnOverState {
    fn default() -> Self {
        Self {
            previous_positions: VecDeque::with_capacity(4),
            threshold: 2.0, // 默认阈值
        }
    }
}

impl TurnOverState {
    /// 更新位置并检测翻身
    pub fn update_and_detect(&mut self, new_position: [i32; 2]) -> Option<TurnOverEvent> {
        self.previous_positions.push_back(new_position);
        
        // 保持最近4个位置
        if self.previous_positions.len() > 4 {
            self.previous_positions.pop_front();
        }
        
        // 需要至少2个位置才能检测
        if self.previous_positions.len() < 2 {
            return None;
        }
        
        let current = new_position;
        let previous = self.previous_positions[self.previous_positions.len() - 2];
        
        // 计算位置变化值
        let value = ((current[0] - previous[0]).abs() + (current[1] - previous[1]).abs()) as f32;
        
        if value > self.threshold {
            Some(TurnOverEvent {
                position_before: previous,
                position_after: current,
                change_value: value,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }
}

/// 翻身事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOverEvent {
    pub position_before: [i32; 2],
    pub position_after: [i32; 2],
    pub change_value: f32,
    pub timestamp: DateTime<Utc>,
}

/// 智能床垫适配器
pub struct SmartMattressAdapter {
    turn_over_state: std::sync::Mutex<TurnOverState>,
    filter: std::sync::Mutex<SmartMattressFilter>,
}

impl SmartMattressAdapter {
    pub fn new() -> Self {
        Self {
            turn_over_state: std::sync::Mutex::new(TurnOverState::default()),
            filter: std::sync::Mutex::new(SmartMattressFilter::new()),
        }
    }
    
    /// 解析TCP数据包
    pub fn parse_tcp_packet(&self, raw: &[u8]) -> AppResult<MattressData> {
        // 验证数据包最小长度
        if raw.len() < 5 {
            return Err(AppError::ValidationError("数据包长度不足".into()));
        }
        
        // 验证魔数
        if raw[0] != 0xab || raw[1] != 0xcd {
            return Err(AppError::ValidationError("无效的魔数".into()));
        }
        
        // 获取数据长度
        let data_len = raw[2] as usize;
        if raw.len() < data_len + 4 {
            return Err(AppError::ValidationError("数据包长度不匹配".into()));
        }
        
        // 验证CRC
        let crc_value = raw[3];
        let data_bytes = &raw[4..4 + data_len];
        
        let crc_algo = Crc::<u8>::new(&CRC_8_SMBUS);
        let calculated_crc = crc_algo.checksum(data_bytes);
        
        if calculated_crc != crc_value {
            return Err(AppError::ValidationError(format!(
                "CRC校验失败: 期望 {}, 实际 {}",
                crc_value, calculated_crc
            )));
        }
        
        // 解析MessagePack数据
        self.parse_messagepack(data_bytes)
    }
    
    /// 解析MessagePack数据
    fn parse_messagepack(&self, data: &[u8]) -> AppResult<MattressData> {
        let value: serde_json::Value = rmp_serde::from_slice(data)
            .map_err(|e| AppError::ValidationError(format!("MessagePack解析失败: {}", e)))?;
        
        // 验证必需的字段
        let manufacturer = value["Ma"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少制造商字段".into()))?;
        
        let model = value["Mo"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少型号字段".into()))?;
        
        let version = value["V"].as_i64()
            .ok_or_else(|| AppError::ValidationError("缺少版本字段".into()))? as i32;
        
        let serial_number = value["Sn"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少序列号字段".into()))?;
        
        let d_node = value["D"].as_object()
            .ok_or_else(|| AppError::ValidationError("缺少数据节点".into()))?;
        
        // 解析详细数据
        let firmware_version = d_node["fv"].as_i64().unwrap_or(0) as i32;
        let status = d_node["St"].as_str().unwrap_or("unknown").to_string();
        let heart_rate = d_node["Hb"].as_i64().unwrap_or(0) as i32;
        let breath_rate = d_node["Br"].as_i64().unwrap_or(0) as i32;
        let wet_status = d_node["Wt"].as_bool().unwrap_or(false);
        let apnea_count = d_node["Od"].as_i64().unwrap_or(0) as i32;
        let weight_value = d_node["We"].as_i64().unwrap_or(-1) as i32;
        
        // 解析位置坐标
        let empty_array = vec![];
        let position_array = d_node["P"].as_array().unwrap_or(&empty_array);
        let position = if position_array.len() >= 2 {
            [
                position_array[0].as_i64().unwrap_or(0) as i32,
                position_array[1].as_i64().unwrap_or(0) as i32,
            ]
        } else {
            [0, 0]
        };
        
        Ok(MattressData {
            manufacturer: manufacturer.to_string(),
            model: model.to_string(),
            version,
            serial_number: serial_number.to_string(),
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
        if cleaned_data.heart_rate < 30 || cleaned_data.heart_rate > 200 {
            cleaned_data.heart_rate = 0; // 标记为无效
        }
        
        // 呼吸率降噪：过滤异常值
        if cleaned_data.breath_rate < 5 || cleaned_data.breath_rate > 60 {
            cleaned_data.breath_rate = 0; // 标记为无效
        }
        
        // 重量值验证
        if cleaned_data.weight_value < -1 || cleaned_data.weight_value > 20 {
            cleaned_data.weight_value = -1; // 标记为无效
        }
        
        // 状态验证
        match cleaned_data.status.as_str() {
            "on" | "off" | "mov" | "call" => {}, // 有效状态
            _ => cleaned_data.status = "unknown".to_string(),
        }
        
        cleaned_data
    }
}

impl DeviceAdapter for SmartMattressAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 解析TCP数据包
        let mattress_data = self.parse_tcp_packet(raw)?;
        
        // 数据降噪
        let cleaned_data = self.denoise_data(&mattress_data);
        
        // 使用智能过滤器处理数据，只返回有价值的事件
        let mut filter = self.filter.lock().unwrap();
        let valuable_events = filter.process_data(&cleaned_data)?;
        
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
            "valuable_events": valuable_events, // 重要事件列表
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
        
        if !serial_number.starts_with('Z') || serial_number.len() < 4 {
            return Err(AppError::ValidationError("无效的序列号格式".into()));
        }
        
        // 验证状态
        let status = payload["status"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少状态".into()))?;
        
        if !matches!(status, "on" | "off" | "mov" | "call") {
            return Err(AppError::ValidationError("无效的状态值".into()));
        }
        
        Ok(())
    }
    
    fn data_type(&self) -> &'static str {
        "mattress_status"
    }
    
    fn device_type(&self) -> &'static str {
        "smart_mattress"
    }
}

impl Default for SmartMattressAdapter {
    fn default() -> Self {
        Self::new()
    }
}