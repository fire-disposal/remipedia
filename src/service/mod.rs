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

use uuid::Uuid;

use crate::errors::{AppError, AppResult};

/// 实体存在检查 — 将 `Option<T>` 转换为 `AppResult<T>`，不存在时返回 `NotFound`
pub fn ensure_found<T>(entity: Option<T>, label: &str, id: &Uuid) -> AppResult<T> {
    entity.ok_or_else(|| AppError::NotFound(format!("{} {} not found", label, id)))
}

/// 简单分页结果（替代每个 Service 手写分页逻辑）
pub struct PageResult<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}
