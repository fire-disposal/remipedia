use chrono::Utc;
use log::info;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::NewBinding;
use crate::dto::request::CreateBindingRequest;
use crate::dto::response::{BindingListResponse, BindingResponse, Pagination};
use crate::errors::{AppError, AppResult};
use crate::repository::{BindingRepository, DeviceRepository, PatientRepository};

pub struct BindingService<'a> {
    binding_repo: BindingRepository<'a>,
    device_repo: DeviceRepository<'a>,
    patient_repo: PatientRepository<'a>,
}

impl<'a> BindingService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            binding_repo: BindingRepository::new(pool),
            device_repo: DeviceRepository::new(pool),
            patient_repo: PatientRepository::new(pool),
        }
    }

    /// 创建绑定
    pub async fn bind(&self, req: CreateBindingRequest) -> AppResult<BindingResponse> {
        // 验证设备存在
        self.device_repo.find_by_id(&req.device_id).await?;

        // 验证患者存在
        self.patient_repo.find_by_id(&req.patient_id).await?;

        // 检查设备是否已有有效绑定
        if self.binding_repo.find_active_by_device(&req.device_id).await?.is_some() {
            return Err(AppError::BindingAlreadyExists);
        }

        let binding = self.binding_repo.create(&NewBinding {
            device_id: req.device_id,
            patient_id: req.patient_id,
            notes: req.notes,
        }).await?;

        info!(
            "绑定创建成功: binding_id={}, device_id={}, patient_id={}",
            binding.id, binding.device_id, binding.patient_id
        );

        Ok(binding.into())
    }

    /// 解除绑定
    pub async fn unbind(&self, binding_id: &Uuid) -> AppResult<()> {
        self.binding_repo.end_binding(binding_id, Utc::now()).await?;
        info!("绑定解除成功: binding_id={}", binding_id);
        Ok(())
    }

    /// 获取设备当前绑定的患者 ID
    pub async fn get_current_binding_subject(&self, device_id: &Uuid) -> AppResult<Option<Uuid>> {
        let binding = self.binding_repo.find_active_by_device(device_id).await?;
        Ok(binding.map(|b| b.patient_id))
    }

    /// 获取设备的绑定历史
    pub async fn get_device_binding_history(&self, device_id: &Uuid, page: u32, page_size: u32) -> AppResult<BindingListResponse> {
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let bindings = self.binding_repo.find_all_by_device(device_id, limit, offset).await?;
        let total = self.binding_repo.count_by_device(device_id).await?;

        let data: Vec<BindingResponse> = bindings.into_iter().map(|b| b.into()).collect();

        Ok(BindingListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total,
                total_pages: (total + limit - 1) / limit,
            },
        })
    }

    /// 获取患者的绑定历史
    pub async fn get_patient_binding_history(&self, patient_id: &Uuid, page: u32, page_size: u32) -> AppResult<BindingListResponse> {
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let bindings = self.binding_repo.find_all_by_patient(patient_id, limit, offset).await?;
        let total = self.binding_repo.count_by_patient(patient_id).await?;

        let data: Vec<BindingResponse> = bindings.into_iter().map(|b| b.into()).collect();

        Ok(BindingListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total,
                total_pages: (total + limit - 1) / limit,
            },
        })
    }

    /// 获取设备的当前绑定
    pub async fn get_current_binding(&self, device_id: &Uuid) -> AppResult<Option<BindingResponse>> {
        let binding = self.binding_repo.find_active_by_device(device_id).await?;
        Ok(binding.map(|b| b.into()))
    }

    /// 查询绑定列表
    pub async fn query(
        &self,
        device_id: Option<Uuid>,
        patient_id: Option<Uuid>,
        active_only: bool,
        page: u32,
        page_size: u32,
    ) -> AppResult<BindingListResponse> {
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let bindings = self.binding_repo.query(device_id, patient_id, active_only, limit, offset).await?;
        let total = self.binding_repo.count_query(device_id, patient_id, active_only).await?;

        let data: Vec<BindingResponse> = bindings.into_iter().map(|b| b.into()).collect();

        Ok(BindingListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total,
                total_pages: (total + limit - 1) / limit,
            },
        })
    }
}

// 实体到响应的转换
impl From<crate::core::entity::Binding> for BindingResponse {
    fn from(binding: crate::core::entity::Binding) -> Self {
        Self {
            id: binding.id,
            device_id: binding.device_id,
            patient_id: binding.patient_id,
            started_at: binding.started_at,
            ended_at: binding.ended_at,
            notes: binding.notes,
        }
    }
}