//! Binding富实体

use chrono::{DateTime, Utc};

use crate::core::domain::shared::{BindingId, DeviceId, DomainError, DomainResult, PatientId};

/// 设备-患者绑定
#[derive(Debug, Clone)]
pub struct Binding {
    id: BindingId,
    device_id: DeviceId,
    patient_id: PatientId,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
}

impl Binding {
    /// 创建新绑定（工厂方法）
    ///
    /// 前置条件：设备必须已激活
    pub fn create(
        device_id: DeviceId,
        patient_id: PatientId,
        notes: Option<String>,
    ) -> DomainResult<Self> {
        let now = Utc::now();
        Ok(Self {
            id: BindingId::new(),
            device_id,
            patient_id,
            started_at: now,
            ended_at: None,
            notes,
            created_at: now,
        })
    }

    /// 从持久化重建
    pub fn reconstruct(
        id: BindingId,
        device_id: DeviceId,
        patient_id: PatientId,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
        notes: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            device_id,
            patient_id,
            started_at,
            ended_at,
            notes,
            created_at,
        }
    }

    /// 结束绑定
    ///
    /// 规则：
    /// - 已结束的绑定不能再次结束
    pub fn end(&mut self, at: DateTime<Utc>) -> DomainResult<()> {
        if self.ended_at.is_some() {
            return Err(DomainError::Validation("绑定已结束".into()));
        }
        if at < self.started_at {
            return Err(DomainError::Validation(
                "结束时间不能早于开始时间".into(),
            ));
        }
        self.ended_at = Some(at);
        Ok(())
    }

    /// 是否有效（未结束）
    pub fn is_active(&self) -> bool {
        self.ended_at.is_none()
    }

    /// 获取绑定时长（如果已结束）
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.ended_at.map(|end| end - self.started_at)
    }

    // ========== Getters ==========

    pub fn id(&self) -> BindingId {
        self.id
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn patient_id(&self) -> PatientId {
        self.patient_id
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_create() {
        let binding = Binding::create(
            DeviceId::new(),
            PatientId::new(),
            Some("测试绑定".into()),
        )
        .unwrap();

        assert!(binding.is_active());
        assert_eq!(binding.notes(), Some("测试绑定"));
    }

    #[test]
    fn test_binding_end() {
        let mut binding = Binding::create(DeviceId::new(), PatientId::new(), None).unwrap();

        let end_time = Utc::now();
        binding.end(end_time).unwrap();

        assert!(!binding.is_active());
        assert!(binding.duration().is_some());
    }

    #[test]
    fn test_binding_end_twice_fails() {
        let mut binding = Binding::create(DeviceId::new(), PatientId::new(), None).unwrap();

        let end_time = Utc::now();
        binding.end(end_time).unwrap();

        // 再次结束应该失败
        let result = binding.end(end_time);
        assert!(result.is_err());
    }

    #[test]
    fn test_binding_end_before_start_fails() {
        let binding = Binding::create(DeviceId::new(), PatientId::new(), None).unwrap();
        let mut binding = binding;

        // 结束时间早于开始时间
        let end_time = binding.started_at() - chrono::Duration::hours(1);
        let result = binding.end(end_time);
        assert!(result.is_err());
    }
}
