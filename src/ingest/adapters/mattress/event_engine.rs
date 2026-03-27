use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use std::collections::VecDeque;

use crate::errors::AppResult;

use super::types::{
    AlertLevel, MattressData, MattressEvent, MattressState, SmartSamplingConfig, VitalSignsConfig,
};

/// 设备级状态：由外部状态存储持有并传入引擎处理函数
pub struct DeviceState {
    pub state: MattressState,
    pub state_history: VecDeque<(MattressState, DateTime<Utc>)>,
    pub last_position: Option<[i32; 2]>,

    // 事件驱动状态
    pub last_vital_signs_event: Option<DateTime<Utc>>,
    pub last_apnea_event: Option<DateTime<Utc>>,
    pub last_moisture_event: Option<(bool, DateTime<Utc>)>,
    pub last_scheduled_measurement: Option<DateTime<Utc>>,
    pub apnea_start_time: Option<DateTime<Utc>>,
    pub moisture_start_time: Option<DateTime<Utc>>,
}

impl DeviceState {
    pub fn new() -> Self {
        Self {
            state: MattressState::OffBed,
            state_history: VecDeque::with_capacity(100),
            last_position: None,
            last_vital_signs_event: None,
            last_apnea_event: None,
            last_moisture_event: None,
            last_scheduled_measurement: None,
            apnea_start_time: None,
            moisture_start_time: None,
        }
    }
}

/// 床垫事件引擎 - 仅包含配置（无内部可变状态），运行时把 `DeviceState` 传入以获得纯函数式行为
pub struct MattressEventEngine {
    // 配置参数
    pub bed_entry_threshold: i32,
    pub bed_exit_threshold: i32,
    pub movement_score_threshold: f32,
    pub sampling_config: SmartSamplingConfig,
    pub vital_signs_config: VitalSignsConfig,
}

impl MattressEventEngine {
    /// 创建新的事件引擎（配置使用默认值）
    pub fn new() -> Self {
        Self {
            bed_entry_threshold: 15,
            bed_exit_threshold: 10,
            movement_score_threshold: 3.0,
            sampling_config: SmartSamplingConfig::default(),
            vital_signs_config: VitalSignsConfig::default(),
        }
    }

    /// 使用自定义配置创建事件引擎
    pub fn with_config(
        sampling_config: SmartSamplingConfig,
        vital_signs_config: VitalSignsConfig,
    ) -> Self {
        Self {
            sampling_config,
            vital_signs_config,
            bed_entry_threshold: 15,
            bed_exit_threshold: 10,
            movement_score_threshold: 3.0,
        }
    }

    /// 处理床垫数据：纯函数式接口，接收 `DeviceState` 的可变引用以更新状态并返回事件列表
    pub fn process(
        &self,
        state: &mut DeviceState,
        data: &MattressData,
    ) -> AppResult<Vec<MattressEvent>> {
        let mut events = Vec::new();
        let timestamp = Utc::now();

        // 数据有效性检查
        if data.heart_rate < 0 || data.breath_rate < 0 || data.apnea_count < 0 {
            return Err(crate::errors::AppError::ValidationError(
                "生命体征数据不能为负数".into(),
            ));
        }

        // 1. 状态检测和事件生成
        let new_state = self.detect_state(data);

        // 2. 上床/下床事件检测
        if new_state != state.state {
            if let Some(event) = self.detect_state_change(state, &new_state, timestamp, data) {
                events.push(event);
                info!("床垫状态变化事件: {:?} -> {:?}", state.state, new_state);
            }
            state.state = new_state.clone();
            state.state_history.push_back((new_state, timestamp));

            // 保持历史记录在合理范围内
            if state.state_history.len() > 100 {
                state.state_history.pop_front();
            }
        }

        // 3. 事件驱动检测（只在床状态下）
        if state.state == MattressState::OnBed {
            let initial_event_count = events.len();

            // 3.1 生命体征异常检测
            if let Some(event) = self.detect_vital_signs_anomaly(state, timestamp, data) {
                events.push(event);
                warn!(
                    "检测到生命体征异常: 心率={}, 呼吸率={}",
                    data.heart_rate, data.breath_rate
                );
            }

            // 3.2 呼吸暂停事件检测
            if let Some(event) = self.detect_apnea_event(state, timestamp, data) {
                events.push(event);
                warn!("检测到呼吸暂停事件: 暂停次数={}", data.apnea_count);
            }

            // 3.3 体湿异常事件检测
            if let Some(event) = self.detect_moisture_alert(state, timestamp, data) {
                events.push(event);
                if data.wet_status {
                    warn!("检测到体湿异常事件");
                }
            }

            // 3.4 体动事件检测
            if let Some(event) = self.detect_movement_event(state, timestamp, data) {
                events.push(event);
                info!("检测到有意义体动事件");
            }

            // 3.5 定时完整数据采集（只在床状态）
            if let Some(event) = self.detect_scheduled_measurement(state, timestamp, data) {
                events.push(event);
                info!(
                    "定时完整数据采集: 心率={}, 呼吸率={}",
                    data.heart_rate, data.breath_rate
                );
            }

            // 记录事件生成统计
            let new_events = events.len() - initial_event_count;
            if new_events > 0 {
                info!("事件驱动检测生成 {} 个新事件", new_events);
            }
        }

        // 4. 更新位置历史
        state.last_position = Some(data.position);

        // 5. 记录处理统计
        debug!(
            "床垫数据处理完成: 生成 {} 个事件, 当前状态: {:?}",
            events.len(),
            state.state
        );

        Ok(events)
    }

    /// 检测当前状态
    fn detect_state(&self, data: &MattressData) -> MattressState {
        match data.status.as_str() {
            "off" => MattressState::OffBed,
            "on" => {
                if data.weight_value >= self.bed_entry_threshold {
                    MattressState::OnBed
                } else {
                    MattressState::OffBed
                }
            }
            "mov" => MattressState::Moving,
            "call" => MattressState::Calling,
            _ => MattressState::OffBed,
        }
    }

    // 以下方法实现与原有SmartMattressFilter相同的事件检测逻辑
    // 为节省空间，这里只展示关键方法签名，具体实现与之前相同

    fn detect_state_change(
        &self,
        state: &DeviceState,
        new_state: &MattressState,
        timestamp: DateTime<Utc>,
        data: &MattressData,
    ) -> Option<MattressEvent> {
        match (&state.state, new_state) {
            (MattressState::OffBed, MattressState::OnBed) => Some(MattressEvent::BedEntry {
                timestamp,
                confidence: self.calculate_bed_entry_confidence(data),
                weight_value: data.weight_value,
            }),
            (MattressState::OnBed, MattressState::OffBed) => {
                let duration = self.calculate_bed_duration(timestamp);
                Some(MattressEvent::BedExit {
                    timestamp,
                    confidence: self.calculate_bed_exit_confidence(data),
                    duration_minutes: duration,
                })
            }
            _ => None,
        }
    }

    fn detect_vital_signs_anomaly(
        &self,
        state: &mut DeviceState,
        timestamp: DateTime<Utc>,
        data: &MattressData,
    ) -> Option<MattressEvent> {
        // 检查采样间隔
        if !self.should_sample_vital_signs(state, timestamp) {
            return None;
        }

        let heart_rate = data.heart_rate;
        let breath_rate = data.breath_rate;

        // 数据有效性检查
        if heart_rate <= 0 || breath_rate <= 0 {
            return None;
        }

        let (hr_min, hr_max) = self.vital_signs_config.heart_rate_normal_range;
        let (br_min, br_max) = self.vital_signs_config.breath_rate_normal_range;

        let mut anomaly_type = String::new();
        let mut heart_rate_level = AlertLevel::Normal;
        let mut breath_rate_level = AlertLevel::Normal;

        // 心率异常检测
        if heart_rate < hr_min {
            heart_rate_level = AlertLevel::Warning;
            anomaly_type.push_str("heart_rate_low;");
        } else if heart_rate > hr_max {
            heart_rate_level = AlertLevel::Warning;
            anomaly_type.push_str("heart_rate_high;");
        }

        // 呼吸频率异常检测
        if breath_rate < br_min {
            breath_rate_level = AlertLevel::Warning;
            anomaly_type.push_str("breath_rate_low;");
        } else if breath_rate > br_max {
            breath_rate_level = AlertLevel::Warning;
            anomaly_type.push_str("breath_rate_high;");
        }

        // 危险级别判断
        if heart_rate < 40 || heart_rate > 150 {
            heart_rate_level = AlertLevel::Critical;
        }
        if breath_rate < 8 || breath_rate > 30 {
            breath_rate_level = AlertLevel::Critical;
        }

        // 只有在异常情况下才生成事件
        if !anomaly_type.is_empty() {
            state.last_vital_signs_event = Some(timestamp);

            warn!(
                "生命体征异常检测: 心率={}({:?}), 呼吸率={}({:?}), 异常类型: {}",
                heart_rate, heart_rate_level, breath_rate, breath_rate_level, anomaly_type
            );

            Some(MattressEvent::VitalSignsAnomaly {
                timestamp,
                heart_rate,
                heart_rate_level,
                breath_rate,
                breath_rate_level,
                anomaly_type,
            })
        } else {
            debug!("生命体征正常: 心率={}, 呼吸率={}", heart_rate, breath_rate);
            None
        }
    }

    fn detect_apnea_event(
        &self,
        state: &mut DeviceState,
        timestamp: DateTime<Utc>,
        data: &MattressData,
    ) -> Option<MattressEvent> {
        let current_apnea = data.apnea_count;

        // 呼吸暂停检测逻辑
        if current_apnea > 0 {
            if state.apnea_start_time.is_none() {
                // 开始新的呼吸暂停事件
                state.apnea_start_time = Some(timestamp);
            }

            let duration = if let Some(start_time) = state.apnea_start_time {
                timestamp.signed_duration_since(start_time).num_seconds() as i32
            } else {
                0
            };

            let severity = if duration >= self.vital_signs_config.apnea_critical_threshold {
                AlertLevel::Critical
            } else {
                AlertLevel::Warning
            };

            // 更新最后事件时间，避免频繁触发
            if self.should_trigger_apnea_event(state, timestamp) {
                state.last_apnea_event = Some(timestamp);

                warn!(
                    "呼吸暂停事件检测: 持续时间={}秒, 严重程度={:?}, 暂停次数={}",
                    duration, severity, current_apnea
                );

                Some(MattressEvent::ApneaEvent {
                    timestamp,
                    duration_seconds: duration,
                    severity,
                    apnea_count: current_apnea,
                })
            } else {
                debug!(
                    "呼吸暂停持续中: 持续时间={}秒, 暂停次数={}",
                    duration, current_apnea
                );
                None
            }
        } else {
            // 呼吸恢复正常，重置计时
            if state.apnea_start_time.is_some() {
                info!("呼吸暂停结束，恢复正常呼吸");
                state.apnea_start_time = None;
            }
            None
        }
    }

    fn detect_moisture_alert(
        &self,
        state: &mut DeviceState,
        timestamp: DateTime<Utc>,
        data: &MattressData,
    ) -> Option<MattressEvent> {
        let current_wet = data.wet_status;

        match (current_wet, state.last_moisture_event) {
            (true, None) => {
                // 开始体湿事件
                state.moisture_start_time = Some(timestamp);
                state.last_moisture_event = Some((true, timestamp));
                None // 不立即触发，等待持续时间
            }
            (true, Some((prev_wet, prev_time))) if prev_wet => {
                // 持续体湿，检查是否达到警告阈值
                let duration_minutes =
                    timestamp.signed_duration_since(prev_time).num_minutes() as i32;

                if duration_minutes >= self.vital_signs_config.moisture_alert_threshold_minutes {
                    if self.should_trigger_moisture_event(state, timestamp) {
                        state.last_moisture_event = Some((true, timestamp));

                        Some(MattressEvent::MoistureAlert {
                            timestamp,
                            wet_status: true,
                            duration_minutes,
                            severity: AlertLevel::Warning,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            (false, Some((prev_wet, prev_time))) if prev_wet => {
                // 体湿结束事件
                let duration_minutes =
                    timestamp.signed_duration_since(prev_time).num_minutes() as i32;
                state.last_moisture_event = Some((false, timestamp));
                state.moisture_start_time = None;

                Some(MattressEvent::MoistureAlert {
                    timestamp,
                    wet_status: false,
                    duration_minutes,
                    severity: AlertLevel::Normal,
                })
            }
            _ => None,
        }
    }

    fn detect_movement_event(
        &self,
        state: &DeviceState,
        timestamp: DateTime<Utc>,
        data: &MattressData,
    ) -> Option<MattressEvent> {
        if data.status != "mov" {
            return None;
        }

        let intensity = self.calculate_movement_intensity(state, data);
        let score = self.calculate_movement_score(data, intensity);

        if score >= self.movement_score_threshold as i32 {
            Some(MattressEvent::SignificantMovement {
                timestamp,
                intensity,
                position_change: intensity,
                score,
            })
        } else {
            None
        }
    }

    fn detect_scheduled_measurement(
        &self,
        state: &mut DeviceState,
        timestamp: DateTime<Utc>,
        data: &MattressData,
    ) -> Option<MattressEvent> {
        // 只在床状态下进行定时采集
        if state.state != MattressState::OnBed {
            return None;
        }

        // 检查是否到了定时采集时间
        if !self.should_sample_scheduled_measurement(state, timestamp) {
            return None;
        }

        // 数据有效性检查
        if data.heart_rate <= 0 || data.breath_rate <= 0 {
            return None;
        }

        state.last_scheduled_measurement = Some(timestamp);

        Some(MattressEvent::ScheduledMeasurement {
            timestamp,
            heart_rate: data.heart_rate,
            breath_rate: data.breath_rate,
            apnea_count: data.apnea_count,
            wet_status: data.wet_status,
            weight_value: data.weight_value,
            measurement_reason: "scheduled".to_string(),
        })
    }

    fn calculate_movement_intensity(&self, state: &DeviceState, data: &MattressData) -> f32 {
        if let Some(last_pos) = state.last_position {
            let dx = (data.position[0] - last_pos[0]).abs() as f32;
            let dy = (data.position[1] - last_pos[1]).abs() as f32;
            (dx + dy).min(10.0) // 限制在0-10范围
        } else {
            0.0
        }
    }

    fn calculate_movement_score(&self, data: &MattressData, intensity: f32) -> i32 {
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

    // 智能采样策略辅助方法
    fn should_sample_vital_signs(&self, state: &DeviceState, _timestamp: DateTime<Utc>) -> bool {
        let alert_level = self.assess_current_alert_level();

        match alert_level {
            AlertLevel::Normal => self
                .check_time_interval_minutes(state, self.sampling_config.normal_interval_minutes),
            AlertLevel::Warning => self
                .check_time_interval_minutes(state, self.sampling_config.warning_interval_minutes),
            AlertLevel::Critical => self
                .check_time_interval_seconds(state, self.sampling_config.critical_interval_seconds),
        }
    }

    fn assess_current_alert_level(&self) -> AlertLevel {
        // 这里可以根据历史事件和当前状态综合评估
        // 简化实现：默认正常，后续可以根据实际需求扩展
        AlertLevel::Normal
    }

    fn check_time_interval_minutes(&self, state: &DeviceState, interval: i32) -> bool {
        if let Some(last_event) = state.last_vital_signs_event {
            let elapsed = Utc::now().signed_duration_since(last_event);
            elapsed.num_minutes() >= interval as i64
        } else {
            true // 第一次采样
        }
    }

    fn check_time_interval_seconds(&self, state: &DeviceState, interval: i32) -> bool {
        if let Some(last_event) = state.last_vital_signs_event {
            let elapsed = Utc::now().signed_duration_since(last_event);
            elapsed.num_seconds() >= interval as i64
        } else {
            true // 第一次采样
        }
    }

    fn should_trigger_apnea_event(&self, state: &DeviceState, timestamp: DateTime<Utc>) -> bool {
        if let Some(last_event) = state.last_apnea_event {
            let elapsed = timestamp.signed_duration_since(last_event);
            elapsed.num_seconds() >= 30 // 30秒内不重复触发
        } else {
            true
        }
    }

    fn should_trigger_moisture_event(&self, state: &DeviceState, timestamp: DateTime<Utc>) -> bool {
        if let Some((_, last_time)) = state.last_moisture_event {
            let elapsed = timestamp.signed_duration_since(last_time);
            elapsed.num_minutes() >= 5 // 5分钟内不重复触发
        } else {
            true
        }
    }

    fn should_sample_scheduled_measurement(
        &self,
        state: &DeviceState,
        timestamp: DateTime<Utc>,
    ) -> bool {
        if let Some(last_measurement) = state.last_scheduled_measurement {
            let elapsed = timestamp.signed_duration_since(last_measurement);
            elapsed.num_minutes() >= self.sampling_config.normal_interval_minutes as i64
        } else {
            true // 第一次测量
        }
    }

    fn calculate_bed_entry_confidence(&self, data: &MattressData) -> f32 {
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

    fn calculate_bed_exit_confidence(&self, data: &MattressData) -> f32 {
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
            confidence += 0.3;
        }

        confidence.min(1.0)
    }

    fn calculate_bed_duration(&self, _timestamp: DateTime<Utc>) -> f32 {
        // 简化的持续时间计算，实际应该基于上床时间
        30.0 // 默认30分钟，实际应该根据实际上床时间计算
    }
}
