//! 简化版设备管理器
//! 
//! 使用 Rust 最佳实践：扁平化、单例模式、直接了当

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use log::{debug, error, info};

use crate::core::entity::IngestData;
use crate::core::value_object::DeviceType;
use crate::errors::AppResult;
use crate::service::DataService;
use sqlx::PgPool;

/// 设备运行时状态
pub struct DeviceState {
    pub serial_number: String,
    pub device_type: DeviceType,
    pub last_seen: DateTime<Utc>,
    pub adapter: Box<dyn crate::ingest::DeviceAdapter>,
}

impl DeviceState {
    pub fn new(serial_number: String, device_type: DeviceType) -> AppResult<Self> {
        let adapter = create_adapter(&device_type)?;
        Ok(Self {
            serial_number,
            device_type,
            last_seen: Utc::now(),
            adapter,
        })
    }

    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    pub fn is_idle(&self, timeout: chrono::Duration) -> bool {
        Utc::now() - self.last_seen > timeout
    }
}

/// 创建适配器
fn create_adapter(device_type: &DeviceType) -> AppResult<Box<dyn crate::ingest::DeviceAdapter>> {
    match device_type {
        DeviceType::SmartMattress => Ok(Box::new(
            crate::ingest::adapters::mattress::MattressAdapter::new()
        )),
        DeviceType::HeartRateMonitor => Ok(Box::new(
            crate::ingest::adapters::heart_rate::HeartRateAdapter::new()
        )),
        DeviceType::FallDetector => Ok(Box::new(
            crate::ingest::adapters::fall_detector::FallDetectorAdapter::new()
        )),
    }
}

/// 设备管理器 - 简单单例
pub struct DeviceManager {
    devices: RwLock<HashMap<String, DeviceState>>,
    pool: Arc<PgPool>,
    idle_timeout: chrono::Duration,
}

impl DeviceManager {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            devices: RwLock::new(HashMap::new()),
            pool,
            idle_timeout: chrono::Duration::minutes(30),
        }
    }

    /// 获取或创建设备状态
    pub async fn get_or_create(&self, serial_number: &str, device_type: DeviceType) -> DeviceState {
        let mut devices = self.devices.write().await;
        
        if let Some(state) = devices.get_mut(serial_number) {
            state.touch();
            return DeviceState {
                serial_number: state.serial_number.clone(),
                device_type: state.device_type,
                last_seen: state.last_seen,
                adapter: (&*state.adapter).clone_box(),
            };
        }
        
        let state = DeviceState::new(serial_number.to_string(), device_type).unwrap();
        let result = DeviceState {
            serial_number: state.serial_number.clone(),
            device_type: state.device_type,
            last_seen: state.last_seen,
            adapter: (&*state.adapter).clone_box(),
        };
        
        devices.insert(serial_number.to_string(), state);
        info!("新设备会话: {} ({})", serial_number, result.device_type);
        
        result
    }

    /// 处理设备数据
    pub async fn process(
        &self,
        serial_number: &str,
        device_type: DeviceType,
        raw: Vec<u8>,
        source: &str,
    ) -> AppResult<ProcessResult> {
        let mut state = self.get_or_create(serial_number, device_type).await;
        
        // 解析
        let output = state.adapter.parse(&raw)?;
        state.adapter.validate(&output)?;
        
        // 入库
        let data_service = DataService::new(&self.pool);
        let mut persisted = 0;
        let mut events = 0;
        let mut errors = Vec::new();
        
        if let crate::ingest::AdapterOutput::Messages(msgs) = output {
            for msg in msgs {
                if msg.message_type.is_some() {
                    events += 1;
                }
                
                let ingest = IngestData {
                    time: msg.time,
                    device_id: uuid::Uuid::nil(),
                    subject_id: None,
                    data_type: msg.data_type.clone(),
                    payload: msg.payload,
                    source: source.to_string(),
                };
                
                match data_service.ingest(ingest).await {
                    Ok(_) => persisted += 1,
                    Err(e) => {
                        error!("入库失败: {}", e);
                        errors.push(format!("{}", e));
                    }
                }
            }
        }
        
        debug!("处理完成: {} -> {}条, {}事件", serial_number, persisted, events);
        
        Ok(ProcessResult {
            serial_number: serial_number.to_string(),
            device_type: state.device_type.to_string(),
            persisted,
            events,
            errors,
        })
    }

    /// 清理空闲设备
    pub async fn cleanup_idle(&self) -> usize {
        let mut devices = self.devices.write().await;
        let before = devices.len();
        
        devices.retain(|_, v| !v.is_idle(self.idle_timeout));
        
        let removed = before - devices.len();
        if removed > 0 {
            info!("清理了 {} 个空闲设备", removed);
        }
        
        removed
    }

    /// 获取所有设备信息
    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;
        
        devices
            .iter()
            .map(|(serial, state)| DeviceInfo {
                serial_number: serial.clone(),
                device_type: state.device_type.to_string(),
                last_seen: state.last_seen,
                is_idle: state.is_idle(self.idle_timeout),
            })
            .collect()
    }
}

/// 处理结果
#[derive(Debug)]
pub struct ProcessResult {
    pub serial_number: String,
    pub device_type: String,
    pub persisted: usize,
    pub events: usize,
    pub errors: Vec<String>,
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub serial_number: String,
    pub device_type: String,
    pub last_seen: DateTime<Utc>,
    pub is_idle: bool,
}
