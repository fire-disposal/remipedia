//! 健康数据仓储接口

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::core::domain::healthdata::{DataType, HealthData};
use crate::core::domain::shared::{DeviceId, DomainResult, PatientId};

/// 健康数据查询条件
#[derive(Debug, Clone, Default)]
pub struct HealthDataQuery {
    pub device_id: Option<Uuid>,
    pub subject_id: Option<Uuid>,
    pub data_type: Option<DataType>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub min_quality: Option<super::DataQuality>,
    pub limit: i64,
    pub offset: i64,
}

impl HealthDataQuery {
    pub fn new() -> Self {
        Self {
            limit: 100,
            offset: 0,
            ..Default::default()
        }
    }

    pub fn with_device(mut self, device_id: Uuid) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn with_subject(mut self, subject_id: Uuid) -> Self {
        self.subject_id = Some(subject_id);
        self
    }

    pub fn with_type(mut self, data_type: DataType) -> Self {
        self.data_type = Some(data_type);
        self
    }

    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_pagination(mut self, limit: i64, offset: i64) -> Self {
        self.limit = limit;
        self.offset = offset;
        self
    }
}

/// 健康数据仓储接口
#[async_trait]
pub trait HealthDataRepository: Send + Sync {
    /// 保存健康数据
    async fn save(&self, data: &HealthData) -> DomainResult<()>;

    /// 批量保存
    async fn save_batch(&self, data_list: &[HealthData]) -> DomainResult<()>;

    /// 根据ID查询
    async fn find_by_id(&self, id: &Uuid) -> DomainResult<Option<HealthData>>;

    /// 查询列表
    async fn query(&self, query: &HealthDataQuery) -> DomainResult<Vec<HealthData>>;

    /// 查询数量
    async fn count(&self, query: &HealthDataQuery) -> DomainResult<i64>;

    /// 获取设备最新数据
    async fn find_latest_by_device(
        &self,
        device_id: &DeviceId,
        data_type: Option<&DataType>,
    ) -> DomainResult<Option<HealthData>>;

    /// 获取患者最新数据
    async fn find_latest_by_subject(
        &self,
        subject_id: &PatientId,
        data_type: Option<&DataType>,
    ) -> DomainResult<Option<HealthData>>;

    /// 聚合查询（按时间分组统计）
    async fn aggregate_by_hour(
        &self,
        device_id: &DeviceId,
        data_type: &DataType,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> DomainResult<Vec<HourlyAggregation>>;

    /// 删除过期数据
    async fn delete_before(&self, before: DateTime<Utc>) -> DomainResult<u64>;
}

/// 小时级聚合结果
#[derive(Debug, Clone)]
pub struct HourlyAggregation {
    pub hour: DateTime<Utc>,
    pub count: i64,
    pub avg_value: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}
