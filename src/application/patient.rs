//! Patient 应用服务

use uuid::Uuid;

use crate::application::AppContext;
use crate::core::domain::patient::{Patient, PatientProfile, PatientRepository};
use crate::core::domain::shared::PatientId;
use crate::dto::request::{CreatePatientProfileRequest, CreatePatientRequest, PatientQuery, UpdatePatientRequest};
use crate::dto::response::{Pagination, PatientDetailResponse, PatientListResponse, PatientProfileResponse, PatientResponse};
use crate::errors::AppResult;

pub struct PatientAppService<'a> {
    ctx: AppContext<'a>,
}

impl<'a> PatientAppService<'a> {
    pub fn new(ctx: AppContext<'a>) -> Self {
        Self { ctx }
    }

    /// 创建患者
    pub async fn create(&self, req: CreatePatientRequest) -> AppResult<PatientResponse> {
        // 检查外部ID唯一性
        if let Some(ref ext_id) = req.external_id {
            if self.ctx.patient_repo().exists_by_external_id(ext_id).await? {
                return Err(crate::errors::AppError::ValidationError("外部ID已存在".into()));
            }
        }

        let patient = Patient::create(req.name, req.external_id)?;
        self.ctx.patient_repo().save(&patient).await?;

        Ok(to_response(&patient))
    }

    /// 获取患者
    pub async fn get_by_id(&self, id: Uuid) -> AppResult<PatientResponse> {
        let patient = self.ctx.patient_repo()
            .find_by_id(&PatientId::from_uuid(id))
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(format!("患者: {}", id)))?;

        Ok(to_response(&patient))
    }

    /// 获取患者详情（含档案）
    pub async fn get_detail(&self, id: Uuid) -> AppResult<PatientDetailResponse> {
        let patient_id = PatientId::from_uuid(id);
        let patient = self.ctx.patient_repo()
            .find_by_id(&patient_id)
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(format!("患者: {}", id)))?;

        let profile = self.ctx.patient_repo().find_profile(&patient_id).await?;

        Ok(PatientDetailResponse {
            id: patient.id().as_uuid(),
            name: patient.name().to_string(),
            external_id: patient.external_id().map(|s| s.to_string()),
            created_at: patient.created_at(),
            updated_at: patient.updated_at(),
            profile: profile.map(|p| PatientProfileResponse {
                date_of_birth: p.date_of_birth,
                gender: p.gender.map(|g| g.to_string()),
                blood_type: p.blood_type.map(|b| b.to_string()),
                contact_phone: p.contact_phone,
                address: p.address,
                emergency_contact: p.emergency_contact,
                emergency_phone: p.emergency_phone,
                medical_id: p.medical_id,
                allergies: p.allergies,
                medical_history: p.medical_history,
                notes: p.notes,
                tags: p.tags,
            }),
        })
    }

    /// 更新患者
    pub async fn update(&self, id: Uuid, req: UpdatePatientRequest) -> AppResult<PatientResponse> {
        let patient_id = PatientId::from_uuid(id);
        let mut patient = self.ctx.patient_repo()
            .find_by_id(&patient_id)
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(format!("患者: {}", id)))?;

        if let Some(name) = req.name {
            patient.update_name(name)?;
        }
        if let Some(external_id) = req.external_id {
            patient.update_external_id(Some(external_id));
        }

        self.ctx.patient_repo().save(&patient).await?;
        Ok(to_response(&patient))
    }

    /// 删除患者
    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let patient_id = PatientId::from_uuid(id);
        // 先删除档案
        self.ctx.patient_repo().delete_profile(&patient_id).await.ok();
        // 再删除患者
        self.ctx.patient_repo().delete(&patient_id).await?;
        Ok(())
    }

    /// 查询患者列表
    pub async fn query(&self, query: PatientQuery) -> AppResult<PatientListResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let patients = self.ctx.patient_repo()
            .find_all(query.name.as_deref(), limit, offset)
            .await?;

        let data: Vec<PatientResponse> = patients.iter().map(to_response).collect();

        Ok(PatientListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total: 0, // 简化
                total_pages: 0,
            },
        })
    }

    /// 创建/更新档案
    pub async fn upsert_profile(
        &self,
        patient_id: Uuid,
        req: CreatePatientProfileRequest,
    ) -> AppResult<PatientProfileResponse> {
        let pid = PatientId::from_uuid(patient_id);
        
        // 确保患者存在
        self.ctx.patient_repo()
            .find_by_id(&pid)
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(format!("患者: {}", patient_id)))?;

        use crate::core::value_object::{BloodType, Gender};
        
        let profile = PatientProfile::new()
            .with_basic_info(
                req.date_of_birth,
                req.gender.and_then(|g| g.parse().ok()),
                req.blood_type.and_then(|b| b.parse().ok()),
            )
            .with_contact(req.contact_phone, req.address)
            .with_emergency(req.emergency_contact, req.emergency_phone);

        self.ctx.patient_repo().save_profile(&pid, &profile).await?;

        Ok(PatientProfileResponse {
            date_of_birth: profile.date_of_birth,
            gender: profile.gender.map(|g| g.to_string()),
            blood_type: profile.blood_type.map(|b| b.to_string()),
            contact_phone: profile.contact_phone,
            address: profile.address,
            emergency_contact: profile.emergency_contact,
            emergency_phone: profile.emergency_phone,
            medical_id: profile.medical_id,
            allergies: profile.allergies,
            medical_history: profile.medical_history,
            notes: profile.notes,
            tags: profile.tags,
        })
    }

    /// 获取档案
    pub async fn get_profile(&self, patient_id: Uuid) -> AppResult<Option<PatientProfileResponse>> {
        let pid = PatientId::from_uuid(patient_id);
        let profile = self.ctx.patient_repo().find_profile(&pid).await?;

        Ok(profile.map(|p| PatientProfileResponse {
            date_of_birth: p.date_of_birth,
            gender: p.gender.map(|g| g.to_string()),
            blood_type: p.blood_type.map(|b| b.to_string()),
            contact_phone: p.contact_phone,
            address: p.address,
            emergency_contact: p.emergency_contact,
            emergency_phone: p.emergency_phone,
            medical_id: p.medical_id,
            allergies: p.allergies,
            medical_history: p.medical_history,
            notes: p.notes,
            tags: p.tags,
        }))
    }
}

fn to_response(patient: &Patient) -> PatientResponse {
    PatientResponse {
        id: patient.id().as_uuid(),
        name: patient.name().to_string(),
        external_id: patient.external_id().map(|s| s.to_string()),
        created_at: patient.created_at(),
        updated_at: patient.updated_at(),
    }
}
