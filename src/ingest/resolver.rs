//! 设备解析器
//!
//! 负责设备自动发现和ID解析

use crate::core::entity::NewDevice;
use crate::errors::{AppError, AppResult};
use crate::ingest::adapter::DeviceType;
use crate::repository::{BindingRepository, DeviceRepository};
use sqlx::PgPool;
use uuid::Uuid;

/// 设备解析器
pub struct DeviceResolver<'a> {
    pool: &'a PgPool,
    auto_register: bool,
    default_patient_id: Option<Uuid>,
}

impl<'a> DeviceResolver<'a> {
    pub fn new(pool: &'a PgPool, auto_register: bool) -> Self {
        Self {
            pool,
            auto_register,
            default_patient_id: None,
        }
    }

    pub fn with_default_patient(mut self, patient_id: Uuid) -> Self {
        self.default_patient_id = Some(patient_id);
        self
    }

    /// 解析设备
    ///
    /// 返回：(device_uuid, patient_uuid)
    pub async fn resolve(
        &self,
        serial_number: &str,
        device_type: DeviceType,
    ) -> AppResult<(Uuid, Option<Uuid>)> {
        let device_repo = DeviceRepository::new(self.pool);
        let binding_repo = BindingRepository::new(self.pool);
        
        // 1. 先尝试查找设备
        match device_repo.find_by_serial(serial_number).await? {
            Some(device) => {
                // 找到设备，查询绑定
                let patient_id = self.find_patient_id(&device.id, &binding_repo).await?;
                Ok((device.id, patient_id))
            }
            None if self.auto_register => {
                // 未找到且允许自动注册
                self.auto_register_device(serial_number, device_type, &device_repo, &binding_repo).await
            }
            None => {
                // 未找到且不允许自动注册
                Err(AppError::NotFound(format!(
                    "设备未找到: {}",
                    serial_number
                )))
            }
        }
    }

    /// 解析设备（带metadata）
    pub async fn resolve_with_metadata(
        &self,
        serial_number: &str,
        device_type: DeviceType,
    ) -> AppResult<ResolvedDevice> {
        let device_type_clone = device_type.clone();
        let (device_id, patient_id) = self.resolve(serial_number, device_type).await?;

        Ok(ResolvedDevice {
            device_id,
            patient_id,
            serial_number: serial_number.to_string(),
            device_type: device_type_clone,
        })
    }

    /// 查找患者ID
    async fn find_patient_id(
        &self,
        device_id: &Uuid,
        binding_repo: &BindingRepository<'_>,
    ) -> AppResult<Option<Uuid>> {
        // 优先查询绑定关系
        match binding_repo.find_active_by_device(device_id).await? {
            Some(binding) => Ok(Some(binding.patient_id)),
            None => {
                // 无绑定，返回默认患者ID（如配置）
                Ok(self.default_patient_id)
            }
        }
    }

    /// 自动注册设备
    async fn auto_register_device(
        &self,
        serial_number: &str,
        device_type: DeviceType,
        device_repo: &DeviceRepository<'_>,
        binding_repo: &BindingRepository<'_>,
    ) -> AppResult<(Uuid, Option<Uuid>)> {
        log::info!(
            "自动注册设备: serial={}, type={:?}",
            serial_number,
            device_type
        );

        let new_device = NewDevice {
            serial_number: serial_number.to_string(),
            device_type: device_type.to_string(),
            status: "active".to_string(),
            firmware_version: None,
            metadata: None,
        };

        match device_repo.insert(&new_device).await {
            Ok(device) => {
                log::info!("设备自动注册成功: device_id={}", device.id);
                Ok((device.id, self.default_patient_id))
            }
            Err(AppError::DatabaseError(sqlx::Error::Database(db_err)))
                if db_err.code().map(|c| c == "23505").unwrap_or(false) =>
            {
                // 唯一约束冲突，说明并发创建，重新查询
                log::warn!("设备并发创建，重新查询: {}", serial_number);
                if let Some(device) =
                    device_repo.find_by_serial(serial_number).await?
                {
                    let patient_id = self.find_patient_id(&device.id, binding_repo).await?;
                    Ok((device.id, patient_id))
                } else {
                    Err(AppError::InternalError)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// 批量解析（用于批量处理）
    pub async fn resolve_batch(
        &self,
        devices: &[(String, DeviceType)],
    ) -> Vec<AppResult<ResolvedDevice>> {
        let mut results = Vec::with_capacity(devices.len());

        for (serial, device_type) in devices {
            let result = self
                .resolve_with_metadata(serial, device_type.clone())
                .await;
            results.push(result);
        }

        results
    }
}

/// 解析后的设备信息
#[derive(Debug, Clone)]
pub struct ResolvedDevice {
    pub device_id: Uuid,
    pub patient_id: Option<Uuid>,
    pub serial_number: String,
    pub device_type: DeviceType,
}

impl ResolvedDevice {
    /// 创建 DataPoint 时填充 device_id 和 patient_id
    pub fn populate_datapoint(
        &self,
        mut datapoint: crate::core::entity::DataPoint,
    ) -> crate::core::entity::DataPoint {
        datapoint.device_id = Some(self.device_id);
        datapoint.patient_id = self.patient_id;
        datapoint
    }
}
