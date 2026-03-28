//! 健康数据应用服务

use std::sync::Arc;

use crate::core::domain::healthdata::{
    DataSource, DataType, HealthData, HealthDataQuery, HealthDataRepository,
};
use crate::core::domain::shared::{DeviceId, PatientId};
use crate::errors::{AppError, AppResult};
use crate::infrastructure::persistence::SqlxHealthDataRepository;

/// 健康数据应用服务
pub struct HealthDataAppService<'a> {
    repo: SqlxHealthDataRepository<'a>,
}

impl<'a> HealthDataAppService<'a> {
    pub fn new(pool: &'a sqlx::PgPool) -> Self {
        Self {
            repo: SqlxHealthDataRepository::new(pool),
        }
    }

    /// 保存健康数据（数据接入层使用）
    pub async fn ingest_data(
        &self,
        device_id: DeviceId,
        data_type: DataType,
        payload: serde_json::Value,
        source: DataSource,
    ) -> AppResult<()> {
        let data = HealthData::create(
            chrono::Utc::now(),
            device_id,
            None, // subject_id 由绑定关系后续关联
            data_type,
            payload,
            source,
        );

        self.repo
            .save(&data)
            .await
            .map_err(|e| AppError::InternalError)?;

        Ok(())
    }

    /// 批量保存（高性能场景）
    pub async fn ingest_batch(
        &self,
        items: Vec<(DeviceId, DataType, serde_json::Value, DataSource)>,
    ) -> AppResult<usize> {
        let data_list: Vec<HealthData> = items
            .into_iter()
            .map(|(device_id, data_type, payload, source)| {
                HealthData::create(
                    chrono::Utc::now(),
                    device_id,
                    None,
                    data_type,
                    payload,
                    source,
                )
            })
            .collect();

        self.repo
            .save_batch(&data_list)
            .await
            .map_err(|e| AppError::InternalError)?;

        Ok(data_list.len())
    }

    /// 查询设备最新数据
    pub async fn get_latest_by_device(
        &self,
        device_id: &DeviceId,
        data_type: Option<&DataType>,
    ) -> AppResult<Option<HealthData>> {
        self.repo
            .find_latest_by_device(device_id, data_type)
            .await
            .map_err(|e| AppError::InternalError)
    }

    /// 查询患者最新数据
    pub async fn get_latest_by_patient(
        &self,
        patient_id: &PatientId,
        data_type: Option<&DataType>,
    ) -> AppResult<Option<HealthData>> {
        self.repo
            .find_latest_by_subject(patient_id, data_type)
            .await
            .map_err(|e| AppError::InternalError)
    }

    /// 查询数据列表
    pub async fn query_data(
        &self,
        query: &HealthDataQuery,
    ) -> AppResult<Vec<HealthData>> {
        self.repo
            .query(query)
            .await
            .map_err(|e| AppError::InternalError)
    }

    /// 查询数据数量
    pub async fn count_data(&self, query: &HealthDataQuery) -> AppResult<i64> {
        self.repo
            .count(query)
            .await
            .map_err(|e| AppError::InternalError)
    }
}
