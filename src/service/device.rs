use log::info;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::NewDevice;
use crate::core::value_object::DeviceType;
use crate::dto::request::{DeviceQuery, RegisterDeviceRequest, UpdateDeviceRequest};
use crate::api::routes::device::DeviceStatsResponse;
use crate::dto::response::{BindingInfo, DeviceListResponse, DeviceResponse, Pagination};
use crate::errors::{AppError, AppResult};
use crate::repository::{BindingRepository, DeviceRepository};

pub struct DeviceService<'a> {
    device_repo: DeviceRepository<'a>,
    binding_repo: BindingRepository<'a>,
}

impl<'a> DeviceService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            device_repo: DeviceRepository::new(pool),
            binding_repo: BindingRepository::new(pool),
        }
    }

    /// 注册设备
    pub async fn register(&self, req: RegisterDeviceRequest) -> AppResult<DeviceResponse> {
        // 验证设备类型
        DeviceType::from_str(&req.device_type).ok_or_else(|| {
            AppError::ValidationError(format!("未知设备类型: {}", req.device_type))
        })?;

        // 检查序列号是否已存在
        if self
            .device_repo
            .exists_by_serial(&req.serial_number)
            .await?
        {
            return Err(AppError::ValidationError("设备序列号已存在".into()));
        }

        let device = self
            .device_repo
            .insert(&NewDevice {
                serial_number: req.serial_number,
                device_type: req.device_type,
                firmware_version: req.firmware_version,
                status: "active".to_string(),
                metadata: req.metadata,
            })
            .await?;

        info!(
            "设备注册成功: device_id={}, serial_number={}",
            device.id, device.serial_number
        );

        Ok(device.into())
    }

    /// 自动注册或获取设备
    pub async fn auto_register_or_get(
        &self,
        serial_number: &str,
        device_type: &str,
    ) -> AppResult<crate::core::entity::Device> {
        // 尝试查找现有设备
        if let Some(device) = self.device_repo.find_by_serial(serial_number).await? {
            info!("设备已存在: device_id={}", device.id);
            return Ok(device);
        }

        // 验证设备类型
        let dev_type = DeviceType::from_str(device_type)
            .ok_or_else(|| AppError::ValidationError(format!("未知设备类型: {}", device_type)))?;

        // 自动创建设备，处理并发插入导致的唯一约束冲突（serial_number 唯一）
        match self
            .device_repo
            .insert(&NewDevice::new(
                serial_number.to_string(),
                dev_type.as_str().to_string(),
            ))
            .await
        {
            Ok(device) => {
                info!(
                    "设备自动注册成功: device_id={}, device_type={}",
                    device.id, device_type
                );
                Ok(device)
            }
            Err(e) => {
                // 如果是唯一约束冲突（Postgres code 23505），说明可能是并发创建，尝试重新查询并返回已存在设备
                if let AppError::DatabaseError(sqlx::Error::Database(db_err)) = &e {
                    if db_err.code().map(|c| c == "23505").unwrap_or(false) {
                        if let Some(device) =
                            self.device_repo.find_by_serial(serial_number).await?
                        {
                            info!("设备已存在(并发创建): device_id={}", device.id);
                            return Ok(device);
                        }
                    }
                }

                Err(e)
            }
        }
    }

    /// 获取设备
    pub async fn get_by_id(&self, id: &Uuid) -> AppResult<DeviceResponse> {
        let device = self.device_repo.find_by_id(id).await?;
        let binding = self.binding_repo.find_active_by_device(id).await?;

        Ok(DeviceResponse {
            id: device.id,
            serial_number: device.serial_number,
            device_type: device.device_type,
            firmware_version: device.firmware_version,
            status: device.status,
            metadata: device.metadata,
            created_at: device.created_at,
            current_binding: binding.map(|b| BindingInfo {
                binding_id: b.id,
                patient_id: b.patient_id,
                patient_name: None,
                started_at: b.started_at,
            }),
        })
    }

    /// 更新设备
    pub async fn update(&self, id: &Uuid, req: UpdateDeviceRequest) -> AppResult<DeviceResponse> {
        let device = self
            .device_repo
            .update(
                id,
                req.firmware_version.as_deref(),
                req.status.as_deref(),
                req.metadata.as_ref(),
            )
            .await?;

        info!("设备更新成功: device_id={}", id);

        Ok(device.into())
    }

    /// 更新设备状态
    pub async fn update_status(&self, id: &Uuid, status: &str) -> AppResult<DeviceResponse> {
        let device = self.device_repo.update_status(id, status).await?;
        info!("设备状态更新成功: device_id={}, status={}", id, status);
        Ok(device.into())
    }

    /// 查询设备列表
    pub async fn query(&self, query: DeviceQuery) -> AppResult<DeviceListResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let devices = self
            .device_repo
            .find_all(
                query.device_type.as_deref(),
                query.status.as_deref(),
                query.serial_number.as_deref(),
                limit,
                offset,
            )
            .await?;

        let total = self
            .device_repo
            .count(
                query.device_type.as_deref(),
                query.status.as_deref(),
                query.serial_number.as_deref(),
            )
            .await?;

        let mut data = Vec::with_capacity(devices.len());
        for device in devices {
            let binding = self.binding_repo.find_active_by_device(&device.id).await?;
            data.push(DeviceResponse {
                id: device.id,
                serial_number: device.serial_number,
                device_type: device.device_type,
                firmware_version: device.firmware_version,
                status: device.status,
                metadata: device.metadata,
                created_at: device.created_at,
                current_binding: binding.map(|b| BindingInfo {
                    binding_id: b.id,
                    patient_id: b.patient_id,
                    patient_name: None,
                    started_at: b.started_at,
                }),
            });
        }

        Ok(DeviceListResponse {
            data,
            pagination: Pagination::new(page, page_size, total),
        })
    }

    /// 删除设备
    pub async fn delete(&self, id: &Uuid) -> AppResult<()> {
        self.device_repo.delete(id).await?;
        info!("设备删除成功: device_id={}", id);
        Ok(())
    }

    /// 获取设备统计
    pub async fn get_stats(&self) -> AppResult<DeviceStatsResponse> {
        let total = self.device_repo.count(None, None, None).await?;
        let active = self.device_repo.count(None, Some("active"), None).await?;
        let inactive = self.device_repo.count(None, Some("inactive"), None).await?;
        
        Ok(DeviceStatsResponse {
            total,
            active,
            inactive,
            online: 0,
            offline: 0,
        })
    }
}

// 实体到响应的转换
impl From<crate::core::entity::Device> for DeviceResponse {
    fn from(device: crate::core::entity::Device) -> Self {
        Self {
            id: device.id,
            serial_number: device.serial_number,
            device_type: device.device_type,
            firmware_version: device.firmware_version,
            status: device.status,
            metadata: device.metadata,
            created_at: device.created_at,
            current_binding: None,
        }
    }
}
