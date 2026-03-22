use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

/// 床垫状态
#[derive(Debug, Clone, PartialEq)]
pub enum MattressState {
    OffBed,      // 离床
    OnBed,       // 在床
    Moving,      // 体动
    Calling,     // 呼叫
}

/// 警报级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AlertLevel {
    Normal,
    Warning,
    Critical,
}

/// 床垫事件类型 - 事件驱动原生架构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MattressEvent {
    // 状态变化事件
    BedEntry {
        timestamp: DateTime<Utc>,
        confidence: f32,
        weight_value: i32,
    },
    BedExit {
        timestamp: DateTime<Utc>,
        confidence: f32,
        duration_minutes: f32,
    },
    SignificantMovement {
        timestamp: DateTime<Utc>,
        intensity: f32,        // 体动强度 0-10
        position_change: f32,  // 位置变化值
        score: i32,            // 体动评分 1-10
    },
    
    // 生命体征异常事件 - 事件原生
    VitalSignsAnomaly {
        timestamp: DateTime<Utc>,
        heart_rate: i32,
        heart_rate_level: AlertLevel,
        breath_rate: i32,
        breath_rate_level: AlertLevel,
        anomaly_type: String,  // "heart_rate_high", "breath_rate_low", etc.
    },
    
    // 呼吸暂停事件
    ApneaEvent {
        timestamp: DateTime<Utc>,
        duration_seconds: i32,
        severity: AlertLevel,
        apnea_count: i32,
    },
    
    // 体湿异常事件
    MoistureAlert {
        timestamp: DateTime<Utc>,
        wet_status: bool,
        duration_minutes: i32,
        severity: AlertLevel,
    },
    
    // 定时完整数据采集事件（只在床状态触发）
    ScheduledMeasurement {
        timestamp: DateTime<Utc>,
        heart_rate: i32,
        breath_rate: i32,
        apnea_count: i32,
        wet_status: bool,
        weight_value: i32,
        measurement_reason: String, // "scheduled", "anomaly_follow_up", etc.
    },
}

/// 智能采样配置
#[derive(Debug, Clone)]
pub struct SmartSamplingConfig {
    pub normal_interval_minutes: i32,      // 正常状态：5分钟
    pub warning_interval_minutes: i32,     // 警告状态：1分钟
    pub critical_interval_seconds: i32,    // 危险状态：10秒
}

impl Default for SmartSamplingConfig {
    fn default() -> Self {
        Self {
            normal_interval_minutes: 5,
            warning_interval_minutes: 1,
            critical_interval_seconds: 10,
        }
    }
}

/// 生命体征正常范围配置
#[derive(Debug, Clone)]
pub struct VitalSignsConfig {
    pub heart_rate_normal_range: (i32, i32),    // (min, max)
    pub breath_rate_normal_range: (i32, i32),
    pub apnea_critical_threshold: i32,          // 呼吸暂停危险阈值
    pub moisture_alert_threshold_minutes: i32,  // 体湿警告阈值
}

impl Default for VitalSignsConfig {
    fn default() -> Self {
        Self {
            heart_rate_normal_range: (60, 100),    // 正常心率范围
            breath_rate_normal_range: (12, 20),    // 正常呼吸频率范围
            apnea_critical_threshold: 10,          // 10秒呼吸暂停为危险
            moisture_alert_threshold_minutes: 30,  // 30分钟持续体湿为警告
        }
    }
}

/// 翻身检测状态
#[derive(Debug, Clone)]
pub struct TurnOverState {
    pub previous_positions: VecDeque<[i32; 2]>,
    pub threshold: f32,
}

impl TurnOverState {
    pub fn new(threshold: f32) -> Self {
        Self {
            previous_positions: VecDeque::with_capacity(10),
            threshold,
        }
    }

    /// 更新位置并检测翻身
    pub fn update_and_detect(&mut self, new_position: [i32; 2]) -> Option<TurnOverEvent> {
        self.previous_positions.push_back(new_position);
        
        // 保持历史记录在合理范围内
        if self.previous_positions.len() > 10 {
            self.previous_positions.pop_front();
        }
        
        // 需要至少3个位置点才能检测翻身
        if self.previous_positions.len() >= 3 {
            let positions: Vec<_> = self.previous_positions.iter().cloned().collect();
            let recent = &positions[positions.len()-3..];
            
            // 检测位置变化是否超过阈值
            let total_movement = recent.windows(2)
                .map(|w| {
                    let dx = (w[1][0] - w[0][0]).abs() as f32;
                    let dy = (w[1][1] - w[0][1]).abs() as f32;
                    (dx * dx + dy * dy).sqrt()
                })
                .sum::<f32>();
            
            if total_movement > self.threshold {
                let position_before = recent[0];
                let position_after = recent[2];
                
                return Some(TurnOverEvent {
                    position_before,
                    position_after,
                    movement_distance: total_movement,
                    timestamp: chrono::Utc::now(),
                });
            }
        }
        
        None
    }
}

/// 翻身事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOverEvent {
    pub position_before: [i32; 2],
    pub position_after: [i32; 2],
    pub movement_distance: f32,
    pub timestamp: DateTime<Utc>,
}