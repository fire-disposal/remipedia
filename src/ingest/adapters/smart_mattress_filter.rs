use crate::errors::AppResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 床垫状态机
#[derive(Debug, Clone, PartialEq)]
pub enum MattressState {
    OffBed,      // 离床
    OnBed,       // 在床
    Moving,      // 体动
    Calling,     // 呼叫
}

/// 有价值的事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValuableEvent {
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
    MeasurementSnapshot {
        timestamp: DateTime<Utc>,
        heart_rate: i32,
        breath_rate: i32,
        apnea_count: i32,
        wet_status: bool,
    },
}

/// 智能床垫数据过滤器
pub struct SmartMattressFilter {
    state: MattressState,
    state_history: VecDeque<(MattressState, DateTime<Utc>)>,
    last_measurement: Option<MeasurementSnapshot>,
    last_position: Option<[i32; 2]>,
    bed_entry_threshold: i32,
    bed_exit_threshold: i32,
    movement_score_threshold: f32,
    measurement_interval_minutes: i32,
}

impl SmartMattressFilter {
    pub fn new() -> Self {
        Self {
            state: MattressState::OffBed,
            state_history: VecDeque::with_capacity(100),
            last_measurement: None,
            last_position: None,
            bed_entry_threshold: 15,      // 重量值>15认为上床
            bed_exit_threshold: 10,       // 重量值<10认为离床
            movement_score_threshold: 3.0, // 体动评分阈值
            measurement_interval_minutes: 5, // 每5分钟存档一次测量值
        }
    }

    /// 处理床垫数据，返回有价值的事件
    pub fn process_data(&mut self, data: &crate::ingest::adapters::smart_mattress::MattressData) -> AppResult<Vec<ValuableEvent>> {
        let mut events = Vec::new();
        let timestamp = Utc::now();

        // 1. 状态检测和事件生成
        let new_state = self.detect_state(data);
        
        // 2. 上床/下床事件检测
        if new_state != self.state {
            if let Some(event) = self.detect_state_change(&new_state, timestamp, data) {
                events.push(event);
            }
            self.state = new_state.clone();
            self.state_history.push_back((new_state, timestamp));
            
            // 保持历史记录在合理范围内
            if self.state_history.len() > 100 {
                self.state_history.pop_front();
            }
        }

        // 3. 体动评分和事件检测
        if let Some(event) = self.detect_significant_movement(data, timestamp) {
            events.push(event);
        }

        // 4. 定期测量值存档
        if let Some(event) = self.should_save_measurement(data, timestamp) {
            events.push(event);
        }

        // 5. 更新位置历史
        self.last_position = Some(data.position);

        Ok(events)
    }

    /// 检测当前状态
    fn detect_state(&self, data: &crate::ingest::adapters::smart_mattress::MattressData) -> MattressState {
        match data.status.as_str() {
            "off" => MattressState::OffBed,
            "on" => {
                if data.weight_value >= self.bed_entry_threshold {
                    MattressState::OnBed
                } else {
                    MattressState::OffBed
                }
            },
            "mov" => MattressState::Moving,
            "call" => MattressState::Calling,
            _ => MattressState::OffBed,
        }
    }

    /// 检测状态变化事件
    fn detect_state_change(
        &mut self, 
        new_state: &MattressState, 
        timestamp: DateTime<Utc>,
        data: &crate::ingest::adapters::smart_mattress::MattressData
    ) -> Option<ValuableEvent> {
        match (&self.state, new_state) {
            (MattressState::OffBed, MattressState::OnBed) => {
                // 上床事件
                Some(ValuableEvent::BedEntry {
                    timestamp,
                    confidence: self.calculate_bed_entry_confidence(data),
                    weight_value: data.weight_value,
                })
            },
            (MattressState::OnBed, MattressState::OffBed) => {
                // 下床事件
                let duration = self.calculate_bed_duration(timestamp);
                Some(ValuableEvent::BedExit {
                    timestamp,
                    confidence: self.calculate_bed_exit_confidence(data),
                    duration_minutes: duration,
                })
            },
            _ => None,
        }
    }

    /// 检测有意义的体动事件
    fn detect_significant_movement(
        &mut self, 
        data: &crate::ingest::adapters::smart_mattress::MattressData,
        timestamp: DateTime<Utc>
    ) -> Option<ValuableEvent> {
        if data.status != "mov" {
            return None;
        }

        // 基于位置变化计算体动强度
        let intensity = if let Some(last_pos) = self.last_position {
            let dx = (data.position[0] - last_pos[0]).abs() as f32;
            let dy = (data.position[1] - last_pos[1]).abs() as f32;
            (dx + dy).min(10.0) // 限制在0-10范围
        } else {
            0.0
        };

        // 计算体动评分（1-10）
        let score = self.calculate_movement_score(data, intensity);

        if score >= self.movement_score_threshold as i32 {
            Some(ValuableEvent::SignificantMovement {
                timestamp,
                intensity,
                position_change: intensity,
                score,
            })
        } else {
            None
        }
    }

    /// 计算体动评分
    fn calculate_movement_score(
        &self, 
        data: &crate::ingest::adapters::smart_mattress::MattressData,
        intensity: f32
    ) -> i32 {
        let mut score = 0;

        // 基于位置变化的评分
        if intensity > 2.0 {
            score += 3;
        } else if intensity > 1.0 {
            score += 2;
        } else if intensity > 0.5 {
            score += 1;
        }

        // 基于心率变化的评分
        if data.heart_rate > 0 && data.heart_rate < 100 {
            if data.heart_rate > 80 {
                score += 2;
            } else if data.heart_rate > 70 {
                score += 1;
            }
        }

        // 基于呼吸率变化的评分
        if data.breath_rate > 0 && data.breath_rate < 30 {
            if data.breath_rate > 20 {
                score += 2;
            } else if data.breath_rate > 15 {
                score += 1;
            }
        }

        score.min(10).max(1)
    }

    /// 判断是否应该保存测量值
    fn should_save_measurement(
        &mut self,
        data: &crate::ingest::adapters::smart_mattress::MattressData,
        timestamp: DateTime<Utc>
    ) -> Option<ValuableEvent> {
        // 只有在上床状态下才保存测量值
        if self.state != MattressState::OnBed {
            return None;
        }

        // 检查是否到了保存间隔
        let should_save = match &self.last_measurement {
            None => true, // 第一次测量
            Some(last) => {
                let elapsed = timestamp.signed_duration_since(last.timestamp);
                elapsed.num_minutes() >= self.measurement_interval_minutes as i64
            }
        };

        if should_save && data.heart_rate > 0 && data.breath_rate > 0 {
            let event = ValuableEvent::MeasurementSnapshot {
                timestamp,
                heart_rate: data.heart_rate,
                breath_rate: data.breath_rate,
                apnea_count: data.apnea_count,
                wet_status: data.wet_status,
            };
            
            self.last_measurement = Some(MeasurementSnapshot {
                timestamp,
                heart_rate: data.heart_rate,
                breath_rate: data.breath_rate,
                apnea_count: data.apnea_count,
                wet_status: data.wet_status,
            });

            Some(event)
        } else {
            None
        }
    }

    /// 计算上床置信度
    fn calculate_bed_entry_confidence(&self, data: &crate::ingest::adapters::smart_mattress::MattressData) -> f32 {
        let mut confidence: f32 = 0.0;

        // 基于重量值的置信度
        if data.weight_value >= self.bed_entry_threshold {
            confidence += 0.6;
            if data.weight_value >= 17 {
                confidence += 0.2; // 高重量值增加置信度
            }
        }

        // 基于状态的置信度
        if data.status == "on" {
            confidence += 0.3;
        }

        // 基于心率的置信度
        if data.heart_rate > 30 && data.heart_rate < 120 {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }

    /// 计算下床置信度
    fn calculate_bed_exit_confidence(&self, data: &crate::ingest::adapters::smart_mattress::MattressData) -> f32 {
        let mut confidence: f32 = 0.0;

        // 基于重量值的置信度
        if data.weight_value <= self.bed_exit_threshold {
            confidence += 0.7;
            if data.weight_value <= 5 {
                confidence += 0.2; // 极低重量值增加置信度
            }
        }

        // 基于状态的置信度
        if data.status == "off" {
            confidence += 0.2;
        }

        // 基于心率的置信度（离床后心率应该降低）
        if data.heart_rate == 0 || data.heart_rate > 120 {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }

    /// 计算在床时长
    fn calculate_bed_duration(&self, current_time: DateTime<Utc>) -> f32 {
        let mut duration = 0.0;
        let mut found_entry = false;

        // 从状态历史中查找最近一次上床时间
        for (state, timestamp) in self.state_history.iter().rev() {
            if *state == MattressState::OnBed {
                found_entry = true;
                duration = current_time.signed_duration_since(*timestamp).num_minutes() as f32;
                break;
            }
        }

        if !found_entry {
            // 如果没有找到上床记录，返回估算值
            30.0
        } else {
            duration
        }
    }
}

impl Default for SmartMattressFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 测量快照结构
#[derive(Debug, Clone)]
struct MeasurementSnapshot {
    timestamp: DateTime<Utc>,
    heart_rate: i32,
    breath_rate: i32,
    apnea_count: i32,
    wet_status: bool,
}