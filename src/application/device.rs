//! Device应用服务

use uuid::Uuid;

use crate::application::AppContext;
use crate::core::domain::device::{Device, DeviceRepository};
use crate::core::domain::shared::DeviceId;
use crate::core::value_object::DeviceTypeId;
use crate::dto::request::{DeviceQuery, RegisterDeviceRequest, UpdateDeviceRequest};
use crate::dto::response::{DeviceListResponse, DeviceResponse, Pagination};
use crate::errors::{AppError, AppResult};

pub struct DeviceAppService<'a> {
    ctx: AppContext<'a>,
}

impl<'a> DeviceAppService<'a> {
    pub fn new(ctx: AppContext<'a>) -> Self {
        Self { ctx }
    }

    /// 注册设备
    pub async fn register(&self, req: RegisterDeviceRequest) -> AppResult<DeviceResponse> {
        // 检查序列号唯一性
        if self.ctx.device_repo().exists_by_serial(&req.serial_number).await? {
            return Err(AppError::ValidationError("序列号已存在".into()));
        }

        // 创建设备
        let device = Device::create(
            req.serial_number,
            DeviceTypeId::new(req.device_type),
        ).map_err(|e| AppError::ValidationError(e.to_string()))?;

        // 保存
        self.ctx.device_repo().save(&device).await?;

        Ok(to_response(device))
    }

    /// 获取设备
    pub async fn get_by_id(&self, id: Uuid) -> AppResult<DeviceResponse> {
        let device = self.ctx.device_repo()
            .find_by_id(&DeviceId::from_uuid(id))
            .await?
            .ok_or_else(|| AppError::NotFound(format!("设备: {}", id)))?;

        Ok(to_response(device))
    }

    /// 更新设备状态
    pub async fn update_status(&self, id: Uuid, status: &str) -> AppResult<DeviceResponse> {
        let mut device = self.ctx.device_repo()
            .find_by_id(&DeviceId::from_uuid(id))
            .await?
            .ok_or_else(|| AppError::NotFound(format!("设备: {}", id)))?;

        match status {
            "active" => device.activate().map_err(|e| AppError::ValidationError(e.to_string()))?,
            "inactive" => device.deactivate().map_err(|e| AppError::ValidationError(e.to_string()))?,
            "maintenance" => device.start_maintenance().map_err(|e| AppError::ValidationError(e.to_string()))?,
            _ => return Err(AppError::ValidationError("无效状态".into())),
        }

        self.ctx.device_repo().save(&device).await?;
        Ok(to_response(device))
    }

    /// 查询设备列表
    pub async fn query(&self, query: DeviceQuery) -> AppResult<DeviceListResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        // 简化实现：先查所有再过滤
        let all_devices = if let Some(ref dtype) = query.device_type {
            self.ctx.device_repo()
                .find_by_type(&DeviceTypeId::new(dtype.clone()), 1000, 0)
                .await?
        } else {
            vec![] // 简化：需要实现find_all
        };

        let data: Vec<DeviceResponse> = all_devices.into_iter().map(to_response).collect();

        Ok(DeviceListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total: 0,
                total_pages: 0,
            },
        })
    }

    /// 删除设备
    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        self.ctx.device_repo()
            .delete(&DeviceId::from_uuid(id))
            .await?;
        Ok(())
    }
}

fn to_response(device: Device) -> DeviceResponse {
    DeviceResponse {
        id: device.id().as_uuid(),
        serial_number: device.serial_number().to_string(),
        device_type: device.device_type().to_string(),
        firmware_version: device.firmware_version().map(|s| s.to_string()),
        status: device.status().to_string(),
        metadata: device.metadata().clone(),
        created_at: device.created_at(),
        current_binding: None, // 简化
    }
}
