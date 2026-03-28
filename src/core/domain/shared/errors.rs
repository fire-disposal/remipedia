//! 领域层错误

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("实体未找到: {0}")]
    NotFound(String),

    #[error("无效的状态流转: 从 {from} 到 {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("验证失败: {0}")]
    Validation(String),

    #[error("设备未激活")]
    DeviceNotActive,

    #[error("绑定冲突: {0}")]
    BindingConflict(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
