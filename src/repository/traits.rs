//! Repository Trait 定义
//!
//! 定义数据访问接口，使 Service 层可脱离具体数据库实现进行单元测试。
//! 遵循 DDD Repository 模式：聚合根通过 Repository 接口访问持久化。

use async_trait::async_trait;
use uuid::Uuid;

use crate::core::entity::{
    AlertEvent, DataStream, DataStreamType, NewAlertEvent,
    NewObservation, Observation,
};
use crate::errors::AppResult;

// ============================================
// DataStream 聚合
// ============================================

#[async_trait]
pub trait DataStreamRepository: Send + Sync {
    /// 获取患者的所有数据流
    async fn find_by_patient(&self, patient_id: &Uuid) -> AppResult<Vec<DataStream>>;

    /// 按设备+类型查找或创建数据流
    async fn find_or_create(
        &self,
        device_id: &Uuid,
        data_type: &str,
        stream_type: &DataStreamType,
        patient_id: Option<Uuid>,
    ) -> AppResult<DataStream>;

    /// 创建数据流
    async fn create(&self, stream: &DataStream) -> AppResult<DataStream>;

    /// 根据 ID 查找数据流
    async fn find_by_id(&self, id: &Uuid) -> AppResult<Option<DataStream>>;

    /// 删除数据流
    async fn delete(&self, id: &Uuid) -> AppResult<()>;
}

#[async_trait]
pub trait ObservationRepository: Send + Sync {
    /// 插入观测数据
    async fn insert(&self, obs: &NewObservation) -> AppResult<Observation>;

    /// 查询观测数据
    async fn query(
        &self,
        patient_id: &Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Observation>>;

    /// 统计观测数据数量
    async fn count(&self, patient_id: &Uuid) -> AppResult<i64>;

    /// 获取患者最新的观测数据
    async fn find_latest_by_patient(&self, patient_id: &Uuid) -> AppResult<Option<Observation>>;
}

#[async_trait]
pub trait AlertEventRepository: Send + Sync {
    /// 插入告警事件
    async fn insert(&self, alert: &NewAlertEvent) -> AppResult<AlertEvent>;

    /// 查询活跃告警
    async fn query_active(&self, patient_id: &Uuid) -> AppResult<Vec<AlertEvent>>;

    /// 确认告警
    async fn acknowledge(&self, id: &Uuid, by: &Uuid) -> AppResult<AlertEvent>;

    /// 解决告警
    async fn resolve(&self, id: &Uuid, by: &Uuid) -> AppResult<AlertEvent>;

    /// 获取告警统计
    async fn get_stats(&self, patient_id: &Uuid) -> AppResult<AlertStats>;
}

// ============================================
// 共享类型
// ============================================

/// 告警统计
#[derive(Debug, Clone, Default)]
pub struct AlertStats {
    pub total_active: i64,
    pub total_acknowledged: i64,
    pub total_resolved: i64,
    pub by_severity: std::collections::HashMap<String, i64>,
}
