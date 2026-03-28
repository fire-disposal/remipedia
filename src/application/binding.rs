//! Binding 应用服务

use chrono::Utc;
use uuid::Uuid;

use crate::application::AppContext;
use crate::core::domain::binding::BindingDomainService;
use crate::core::domain::binding::BindingRepository;
use crate::core::domain::shared::{BindingId, DeviceId, PatientId};
use crate::dto::request::CreateBindingRequest;
use crate::dto::response::{BindingListResponse, BindingResponse, Pagination};
use crate::errors::AppResult;

pub struct BindingAppService<'a> {
    ctx: AppContext<'a>,
}

impl<'a> BindingAppService<'a> {
    pub fn new(ctx: AppContext<'a>) -> Self {
        Self { ctx }
    }

    /// 创建绑定（绑定设备到患者）
    pub async fn create(&self, req: CreateBindingRequest) -> AppResult<BindingResponse> {
        let service = BindingDomainService::new(
            self.ctx.device_repo(),
            self.ctx.binding_repo(),
        );

        let binding = service.bind_device_to_patient(
            DeviceId::from_uuid(req.device_id),
            PatientId::from_uuid(req.patient_id),
            req.notes,
        ).await?;

        Ok(to_response(&binding))
    }

    /// 获取绑定
    pub async fn get_by_id(&self, id: Uuid) -> AppResult<BindingResponse> {
        let binding = self.ctx.binding_repo()
            .find_by_id(&BindingId::from_uuid(id))
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(format!("绑定: {}", id)))?;

        Ok(to_response(&binding))
    }

    /// 结束绑定（解绑）
    pub async fn end_binding(&self, id: Uuid) -> AppResult<()> {
        let binding_id = BindingId::from_uuid(id);
        self.ctx.binding_repo()
            .end_binding(&binding_id, Utc::now())
            .await?;
        Ok(())
    }

    /// 获取设备的绑定历史
    pub async fn get_device_bindings(
        &self,
        device_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> AppResult<BindingListResponse> {
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let bindings = self.ctx.binding_repo()
            .find_by_device(&DeviceId::from_uuid(device_id), limit, offset)
            .await?;

        let data: Vec<BindingResponse> = bindings.iter().map(to_response).collect();

        Ok(BindingListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total: 0,
                total_pages: 0,
            },
        })
    }

    /// 获取患者的绑定历史
    pub async fn get_patient_bindings(
        &self,
        patient_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> AppResult<BindingListResponse> {
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let bindings = self.ctx.binding_repo()
            .find_by_patient(&PatientId::from_uuid(patient_id), limit, offset)
            .await?;

        let data: Vec<BindingResponse> = bindings.iter().map(to_response).collect();

        Ok(BindingListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total: 0,
                total_pages: 0,
            },
        })
    }

    /// 获取设备当前有效绑定
    pub async fn get_active_by_device(&self, device_id: Uuid) -> AppResult<Option<BindingResponse>> {
        let binding = self.ctx.binding_repo()
            .find_active_by_device(&DeviceId::from_uuid(device_id))
            .await?;

        Ok(binding.map(|b| to_response(&b)))
    }
}

fn to_response(binding: &crate::core::domain::binding::Binding) -> BindingResponse {
    BindingResponse {
        id: binding.id().as_uuid(),
        device_id: binding.device_id().as_uuid(),
        patient_id: binding.patient_id().as_uuid(),
        started_at: binding.started_at(),
        ended_at: binding.ended_at(),
        notes: binding.notes().map(|s| s.to_string()),
    }
}
