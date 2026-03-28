use log::info;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{NewPatient, NewPatientProfile};
use crate::dto::request::{
    CreatePatientProfileRequest, CreatePatientRequest, PatientQuery, UpdatePatientRequest,
};
use crate::dto::response::{
    BindingInfo, DeviceResponse, Pagination, PatientDetailResponse, PatientListResponse, 
    PatientProfileResponse, PatientResponse, PatientStatsResponse,
};
use crate::errors::{AppError, AppResult};
use crate::repository::{BindingRepository, DeviceRepository, PatientRepository};

pub struct PatientService<'a> {
    patient_repo: PatientRepository<'a>,
    device_repo: DeviceRepository<'a>,
    binding_repo: BindingRepository<'a>,
}

impl<'a> PatientService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            patient_repo: PatientRepository::new(pool),
            device_repo: DeviceRepository::new(pool),
            binding_repo: BindingRepository::new(pool),
        }
    }

    /// 创建患者
    pub async fn create(&self, req: CreatePatientRequest) -> AppResult<PatientResponse> {
        // 检查外部 ID 是否已存在
        if let Some(ref external_id) = req.external_id {
            if self
                .patient_repo
                .find_by_external_id(external_id)
                .await?
                .is_some()
            {
                return Err(AppError::ValidationError("外部 ID 已存在".into()));
            }
        }

        let patient = self
            .patient_repo
            .insert(&NewPatient {
                name: req.name,
                external_id: req.external_id,
            })
            .await?;

        // 如果提供了 profile，同时创建档案
        if let Some(profile_req) = req.profile {
            self.upsert_profile(&patient.id, profile_req).await?;
        }

        info!(
            "患者创建成功: patient_id={}, name={}",
            patient.id, patient.name
        );

        // 返回完整信息
        self.get_by_id(&patient.id).await
    }

    /// 获取患者
    pub async fn get_by_id(&self, id: &Uuid) -> AppResult<PatientResponse> {
        let patient = self.patient_repo.find_by_id(id).await?;
        Ok(patient.into())
    }

    /// 获取患者详情（含档案）
    pub async fn get_detail(&self, id: &Uuid) -> AppResult<PatientDetailResponse> {
        let patient = self.patient_repo.find_by_id(id).await?;
        let profile = self.patient_repo.find_profile(id).await?;

        Ok(PatientDetailResponse {
            id: patient.id,
            name: patient.name,
            external_id: patient.external_id,
            created_at: patient.created_at,
            updated_at: patient.updated_at,
            profile: profile.map(|p| p.into()),
        })
    }

    /// 更新患者
    pub async fn update(&self, id: &Uuid, req: UpdatePatientRequest) -> AppResult<PatientResponse> {
        let patient = self
            .patient_repo
            .update(id, req.name.as_deref(), req.external_id.as_deref())
            .await?;

        info!("患者更新成功: patient_id={}", id);

        Ok(patient.into())
    }

    /// 查询患者列表
    pub async fn query(&self, query: PatientQuery) -> AppResult<PatientListResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let patients = self
            .patient_repo
            .find_all(
                query.name.as_deref(),
                query.external_id.as_deref(),
                limit,
                offset,
            )
            .await?;

        let total = self
            .patient_repo
            .count(query.name.as_deref(), query.external_id.as_deref())
            .await?;

        let data: Vec<PatientResponse> = patients.into_iter().map(|p| p.into()).collect();

        Ok(PatientListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total,
                total_pages: (total + limit - 1) / limit,
            },
        })
    }

    /// 删除患者
    pub async fn delete(&self, id: &Uuid) -> AppResult<()> {
        self.patient_repo.delete(id).await?;
        info!("患者删除成功: patient_id={}", id);
        Ok(())
    }

    /// 获取患者绑定的设备列表
    pub async fn get_patient_devices(&self, patient_id: &Uuid, active_only: bool) -> AppResult<Vec<DeviceResponse>> {
        self.patient_repo.find_by_id(patient_id).await?;
        
        let bindings: Vec<crate::core::entity::Binding> = if active_only {
            self.binding_repo.find_active_by_patient(patient_id).await?
                .map(|b| vec![b])
                .unwrap_or_default()
        } else {
            self.binding_repo.find_all_by_patient(patient_id, 1000, 0).await?
        };

        let mut devices = Vec::new();
        for binding in bindings {
            if let Ok(device) = self.device_repo.find_by_id(&binding.device_id).await {
                devices.push(DeviceResponse {
                    id: device.id,
                    serial_number: device.serial_number,
                    device_type: device.device_type,
                    firmware_version: device.firmware_version,
                    status: device.status,
                    metadata: device.metadata,
                    created_at: device.created_at,
                    current_binding: Some(BindingInfo {
                        binding_id: binding.id,
                        patient_id: binding.patient_id,
                        patient_name: None,
                        started_at: binding.started_at,
                    }),
                });
            }
        }

        Ok(devices)
    }

    // ========== 患者档案 ==========

    /// 创建或更新患者档案
    pub async fn upsert_profile(
        &self,
        patient_id: &Uuid,
        req: CreatePatientProfileRequest,
    ) -> AppResult<PatientProfileResponse> {
        // 确保患者存在
        self.patient_repo.find_by_id(patient_id).await?;

        // 删除现有档案
        let _ = self.patient_repo.delete_profile(patient_id).await;

        // 创建新档案
        let profile = self
            .patient_repo
            .insert_profile(&NewPatientProfile {
                patient_id: *patient_id,
                date_of_birth: req.date_of_birth,
                gender: req.gender,
                blood_type: req.blood_type,
                contact_phone: req.contact_phone,
                address: req.address,
                emergency_contact: req.emergency_contact,
                emergency_phone: req.emergency_phone,
                medical_id: req.medical_id,
                allergies: req.allergies,
                medical_history: req.medical_history,
                notes: req.notes,
                tags: req.tags,
                metadata: req.metadata,
            })
            .await?;

        info!("患者档案创建成功: patient_id={}", patient_id);

        Ok(profile.into())
    }

    /// 获取患者档案
    pub async fn get_profile(
        &self,
        patient_id: &Uuid,
    ) -> AppResult<Option<PatientProfileResponse>> {
        let profile = self.patient_repo.find_profile(patient_id).await?;
        Ok(profile.map(|p| p.into()))
    }

    /// 删除患者档案
    pub async fn delete_profile(&self, patient_id: &Uuid) -> AppResult<()> {
        self.patient_repo.find_by_id(patient_id).await?;
        self.patient_repo.delete_profile(patient_id).await?;
        info!("患者档案删除成功: patient_id={}", patient_id);
        Ok(())
    }

    /// 获取患者统计信息
    pub async fn get_stats(&self, patient_id: &Uuid) -> AppResult<PatientStatsResponse> {
        // 确保患者存在
        self.patient_repo.find_by_id(patient_id).await?;

        // 获取绑定的设备数量
        let device_count = self.binding_repo.count_by_patient(patient_id).await?;

        // 获取有效绑定数
        let active_bindings = self.binding_repo.find_all_by_patient(patient_id, 1000, 0).await?;
        let active_device_count = active_bindings.iter().filter(|b| b.ended_at.is_none()).count() as i64;

        Ok(PatientStatsResponse {
            device_count,
            active_device_count,
        })
    }
}

// 实体到响应的转换
impl From<crate::core::entity::Patient> for PatientResponse {
    fn from(patient: crate::core::entity::Patient) -> Self {
        Self {
            id: patient.id,
            name: patient.name,
            external_id: patient.external_id,
            created_at: patient.created_at,
            updated_at: patient.updated_at,
        }
    }
}

impl From<crate::core::entity::PatientProfile> for PatientProfileResponse {
    fn from(profile: crate::core::entity::PatientProfile) -> Self {
        Self {
            date_of_birth: profile.date_of_birth,
            gender: profile.gender,
            blood_type: profile.blood_type,
            contact_phone: profile.contact_phone,
            address: profile.address,
            emergency_contact: profile.emergency_contact,
            emergency_phone: profile.emergency_phone,
            medical_id: profile.medical_id,
            allergies: profile.allergies,
            medical_history: profile.medical_history,
            notes: profile.notes,
            tags: profile.tags,
        }
    }
}
