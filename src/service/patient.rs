use log::info;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{NewPatient, NewPatientProfile};
use crate::dto::request::{
    CreatePatientProfileRequest, CreatePatientRequest, PatientQuery, UpdatePatientRequest,
};
use crate::dto::response::{
    Pagination, PatientDetailResponse, PatientListResponse, PatientProfileResponse, PatientResponse,
};
use crate::errors::{AppError, AppResult};
use crate::repository::PatientRepository;

pub struct PatientService<'a> {
    patient_repo: PatientRepository<'a>,
}

impl<'a> PatientService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            patient_repo: PatientRepository::new(pool),
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

        info!(
            "患者创建成功: patient_id={}, name={}",
            patient.id, patient.name
        );

        Ok(patient.into())
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

    // ========== 患者档案 ==========

    /// 创建或更新患者档案
    pub async fn upsert_profile(
        &self,
        patient_id: &Uuid,
        req: CreatePatientProfileRequest,
    ) -> AppResult<PatientProfileResponse> {
        // 确保患者存在
        self.patient_repo.find_by_id(patient_id).await?;

        // 原子 upsert，避免先删后插导致的数据窗口和时间戳丢失
        let profile = self
            .patient_repo
            .upsert_profile(&NewPatientProfile {
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

        info!("患者档案写入成功: patient_id={}", patient_id);

        Ok(profile.into())
    }

    /// 获取患者档案
    pub async fn get_profile(
        &self,
        patient_id: &Uuid,
    ) -> AppResult<Option<PatientProfileResponse>> {
        // 确保患者存在，避免把不存在患者和“无档案”混为一谈
        self.patient_repo.find_by_id(patient_id).await?;

        let profile = self.patient_repo.find_profile(patient_id).await?;
        Ok(profile.map(|p| p.into()))
    }

    pub async fn delete_profile(&self, patient_id: &Uuid) -> AppResult<()> {
        // 确保患者存在
        self.patient_repo.find_by_id(patient_id).await?;
        self.patient_repo.delete_profile(patient_id).await?;
        info!("患者档案删除成功: patient_id={}", patient_id);
        Ok(())
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
        }
    }
}

impl From<crate::core::entity::PatientProfile> for PatientProfileResponse {
    fn from(profile: crate::core::entity::PatientProfile) -> Self {
        Self {
            patient_id: profile.patient_id,
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
            metadata: profile.metadata,
            created_at: profile.created_at,
            updated_at: profile.updated_at,
        }
    }
}
