mod admin;
mod auth;
mod binding;
mod data;
mod device;
mod ingest_raw;
mod patient;
mod user;

pub use admin::*;
pub use auth::*;
pub use binding::*;
pub use data::*;
pub use device::*;
pub use ingest_raw::*;
pub use patient::*;
pub use user::*;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::NewAuditLog;
use crate::errors::{AppError, AppResult};
use crate::repository::AuditLogRepository;

/// 实体存在检查 — 将 `Option<T>` 转换为 `AppResult<T>`，不存在时返回 `NotFound`
pub fn ensure_found<T>(entity: Option<T>, label: &str, id: &Uuid) -> AppResult<T> {
    entity.ok_or_else(|| AppError::not_found(format!("{} {} not found", label, id)))
}

/// 简单分页结果（替代每个 Service 手写分页逻辑）
pub struct PageResult<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

// ============================================
// 审计日志辅助函数
// ============================================

/// 记录成功审计日志
pub(crate) async fn log_audit_success(
    pool: &PgPool,
    user_id: Option<Uuid>,
    action: &str,
    resource: &str,
    resource_id: Option<String>,
    details: Option<Value>,
) -> AppResult<()> {
    let mut log = NewAuditLog::success(user_id, action, resource, resource_id);
    if let Some(d) = details {
        log = log.with_details(d);
    }
    AuditLogRepository::new(pool).create(&log).await?;
    Ok(())
}

/// 记录失败审计日志
#[allow(dead_code)]
pub(crate) async fn log_audit_failure(
    pool: &PgPool,
    user_id: Option<Uuid>,
    action: &str,
    resource: &str,
    error: &str,
    details: Option<Value>,
) -> AppResult<()> {
    let mut log = NewAuditLog::failure(user_id, action, resource, error);
    if let Some(d) = details {
        log = log.with_details(d);
    }
    AuditLogRepository::new(pool).create(&log).await?;
    Ok(())
}
