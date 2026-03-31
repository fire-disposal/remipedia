//! 床垫状态实现

use crate::errors::AppResult;
use crate::ingest::adapters::mattress::types::{
    AlertLevel, MattressData, MattressEvent, MattressState,
};
use crate::ingest::state::{DeviceEvent, DeviceState, EventSeverity};
use chrono::Utc;
use std::any::Any;
use std::collections::VecDeque;
use std::time::Instant;

/// 床垫设备状态
#[derive(Debug, Clone)]
pub struct MattressStateV2 {
    pub state: MattressState,
    pub state_history: VecDeque<(MattressState, chrono::DateTime<chrono::Utc>)>,
    pub last_position: Option<[i32; 2]>,
    pub last_vital_signs_event: Option<chrono::DateTime<chrono::Utc>>,
    pub last_apnea_event: Option<chrono::DateTime<chrono::Utc>>,
    pub last_moisture_event: Option<(bool, chrono::DateTime<chrono::Utc>)>,
    pub last_scheduled_measurement: Option<chrono::DateTime<chrono::Utc>>,
    pub apnea_start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub moisture_start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub bed_entry_time: Option<chrono::DateTime<chrono::Utc>>,
    last_accessed: Instant,
}

impl Default for MattressStateV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl MattressStateV2 {
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
            bed_entry_time: None,
            last_accessed: Instant::now(),
        }
    }

    pub fn update_state(
        &mut self,
        new_state: MattressState,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Option<MattressEvent> {
        if self.state != new_state {
            let event = match (&self.state, &new_state) {
                (MattressState::OffBed, MattressState::OnBed) => {
                    self.bed_entry_time = Some(timestamp);
                    Some(MattressEvent::BedEntry {
                        timestamp,
                        confidence: 0.95,
                        weight_value: 0,
                    })
                }
                (MattressState::OnBed, MattressState::OffBed) => {
                    let duration = self
                        .bed_entry_time
                        .map(|t| {
                            let mins = (timestamp - t).num_seconds() as f32 / 60.0;
                            mins.max(0.0)
                        })
                        .unwrap_or(0.0);
                    self.bed_entry_time = None;
                    Some(MattressEvent::BedExit {
                        timestamp,
                        confidence: 0.95,
                        duration_minutes: duration,
                    })
                }
                _ => None,
            };

            self.state = new_state;
            self.state_history.push_back((new_state, timestamp));

            // 保持历史记录在合理范围内
            if self.state_history.len() > 100 {
                self.state_history.pop_front();
            }

            event
        } else {
            None
        }
    }
}

impl DeviceState for MattressStateV2 {
    fn update(&mut self, data: &serde_json::Value) -> AppResult<Vec<DeviceEvent>> {
        let mattress_data = MattressData::from_json(data)
            .ok_or_else(|| crate::errors::AppError::ValidationError("无效的床垫数据".into()))?;

        let mut events = Vec::new();
        let timestamp = Utc::now();

        // 检测当前状态
        let new_state = detect_state(&mattress_data);

        // 1. 状态变化事件
        if let Some(event) = self.update_state(new_state, timestamp) {
            events.push(convert_event(event));
        }

        // 2. 在床状态下的检测
        if self.state == MattressState::OnBed {
            // 体动检测
            if let Some(event) = detect_movement(self, &mattress_data, timestamp) {
                events.push(convert_event(event));
            }

            // 生命体征异常
            if let Some(event) =
                detect_vital_signs_anomaly(&mattress_data, self.last_vital_signs_event, timestamp)
            {
                self.last_vital_signs_event = Some(timestamp);
                events.push(convert_event(event));
            }

            // 呼吸暂停
            if let Some(event) = detect_apnea(self, &mattress_data, timestamp) {
                self.last_apnea_event = Some(timestamp);
                events.push(convert_event(event));
            }

            // 体湿检测
            if let Some(event) = detect_moisture(self, &mattress_data, timestamp) {
                self.last_moisture_event = Some((mattress_data.wet_status, timestamp));
                events.push(convert_event(event));
            }

            // 定时采集
            if let Some(event) = detect_scheduled_measurement(self, &mattress_data, timestamp) {
                self.last_scheduled_measurement = Some(timestamp);
                events.push(convert_event(event));
            }
        }

        // 更新位置历史
        self.last_position = Some(mattress_data.position);
        self.touch();

        Ok(events)
    }

    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "state": format!("{:?}", self.state),
            "state_history_len": self.state_history.len(),
            "last_position": self.last_position,
        })
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn last_accessed(&self) -> Instant {
        self.last_accessed
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn detect_state(data: &MattressData) -> MattressState {
    match data.status.as_str() {
        "off" => MattressState::OffBed,
        "on" => {
            if data.weight_value >= 15 {
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

fn detect_movement(
    state: &MattressStateV2,
    data: &MattressData,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Option<MattressEvent> {
    if let Some(last_pos) = state.last_position {
        let dx = (data.position[0] - last_pos[0]).abs() as f32;
        let dy = (data.position[1] - last_pos[1]).abs() as f32;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance > 5.0 {
            return Some(MattressEvent::SignificantMovement {
                timestamp,
                intensity: distance / 10.0,
                position_change: distance,
                score: (distance as i32).min(10),
            });
        }
    }
    None
}

fn detect_vital_signs_anomaly(
    data: &MattressData,
    last_event: Option<chrono::DateTime<chrono::Utc>>,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Option<MattressEvent> {
    // 事件去抖动：1分钟内不重复报告
    if let Some(last) = last_event {
        if (timestamp - last).num_seconds() < 60 {
            return None;
        }
    }

    let (hr_level, br_level) = AlertLevel::from_vital_signs(data.heart_rate, data.breath_rate);

    if hr_level != AlertLevel::Normal || br_level != AlertLevel::Normal {
        let anomaly_type = if hr_level == AlertLevel::Critical {
            "heart_rate_critical".to_string()
        } else if hr_level == AlertLevel::Warning {
            "heart_rate_warning".to_string()
        } else if br_level == AlertLevel::Critical {
            "breath_rate_critical".to_string()
        } else {
            "breath_rate_warning".to_string()
        };

        return Some(MattressEvent::VitalSignsAnomaly {
            timestamp,
            heart_rate: data.heart_rate,
            heart_rate_level: hr_level,
            breath_rate: data.breath_rate,
            breath_rate_level: br_level,
            anomaly_type,
        });
    }

    None
}

fn detect_apnea(
    state: &mut MattressStateV2,
    data: &MattressData,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Option<MattressEvent> {
    // 呼吸暂停次数增加
    if let Some(last_event) = state.last_apnea_event {
        // 简单的防抖动：5分钟内不重复报告
        if (timestamp - last_event).num_seconds() < 300 {
            return None;
        }
    }

    // 呼吸暂停次数>0 且呼吸频率异常低
    if data.apnea_count > 0 && data.breath_rate < 10 {
        let duration = state
            .apnea_start_time
            .map(|t| (timestamp - t).num_seconds() as i32)
            .unwrap_or(0);

        state.apnea_start_time = Some(timestamp);

        return Some(MattressEvent::ApneaEvent {
            timestamp,
            duration_seconds: duration.max(10),
            severity: if duration > 30 {
                AlertLevel::Critical
            } else {
                AlertLevel::Warning
            },
            apnea_count: data.apnea_count,
        });
    }

    state.apnea_start_time = None;
    None
}

fn detect_moisture(
    state: &mut MattressStateV2,
    data: &MattressData,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Option<MattressEvent> {
    if data.wet_status {
        if state.moisture_start_time.is_none() {
            state.moisture_start_time = Some(timestamp);
        }

        let duration = state
            .moisture_start_time
            .map(|t| ((timestamp - t).num_seconds() / 60) as i32)
            .unwrap_or(0);

        // 持续30分钟以上才报警
        if duration >= 30 {
            // 防抖动：1小时内不重复报告
            if let Some((_, last_time)) = state.last_moisture_event {
                if (timestamp - last_time).num_seconds() < 3600 {
                    return None;
                }
            }

            return Some(MattressEvent::MoistureAlert {
                timestamp,
                wet_status: true,
                duration_minutes: duration,
                severity: if duration > 60 {
                    AlertLevel::Critical
                } else {
                    AlertLevel::Warning
                },
            });
        }
    } else {
        state.moisture_start_time = None;
    }

    None
}

fn detect_scheduled_measurement(
    state: &MattressStateV2,
    data: &MattressData,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Option<MattressEvent> {
    // 每5分钟采集一次
    if let Some(last) = state.last_scheduled_measurement {
        if (timestamp - last).num_seconds() < 300 {
            return None;
        }
    }

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

fn convert_event(event: MattressEvent) -> DeviceEvent {
    DeviceEvent {
        event_type: event.event_type(),
        timestamp: event.timestamp(),
        severity: match event.severity() {
            Some(AlertLevel::Normal) | None => EventSeverity::Info,
            Some(AlertLevel::Warning) => EventSeverity::Warning,
            Some(AlertLevel::Critical) => EventSeverity::Critical,
        },
        payload: serde_json::to_value(&event).unwrap_or_default(),
    }
}
