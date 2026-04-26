//! Repository 错误处理工具
//!
//! 提供 sqlx 错误到 AppError 的映射工具函数及 Option 扩展。

use crate::errors::{AppError, AppResult};
use uuid::Uuid;

/// Repository 错误处理工具
pub struct RepositoryHelper;

impl RepositoryHelper {
    /// 将 sqlx `RowNotFound` 错误映射为 [`AppError::NotFound`]。
    pub fn map_not_found_error(e: sqlx::Error, entity_name: &str, id: &Uuid) -> AppError {
        match e {
            sqlx::Error::RowNotFound => AppError::not_found(format!("{}: {}", entity_name, id)),
            other => AppError::DatabaseError(other),
        }
    }

    /// 将 sqlx 写入错误映射为 [`AppError`]，自动识别唯一约束冲突（23505）。
    pub fn map_write_error(e: sqlx::Error, duplicate_msg: &str) -> AppError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::validation(duplicate_msg);
            }
        }
        AppError::DatabaseError(e)
    }

    /// 检查删除结果，若影响行数为 0 则返回 [`AppError::NotFound`]。
    pub fn check_delete_result(
        result: sqlx::postgres::PgQueryResult,
        entity_name: &str,
        id: &Uuid,
    ) -> AppResult<()> {
        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("{}: {}", entity_name, id)));
        }
        Ok(())
    }
}

/// Option<T> 扩展 trait，提供 ensure_found 方法
///
/// 用于将 `Option<T>` 转换为 `T`，当值为 `None` 时自动返回 `AppError::NotFound`。
///
/// # 示例
///
/// ```ignore
/// let role = self.role_repo.find_by_id(id).await?.ensure_found("角色", id)?;
/// ```
pub trait EnsureFound<T> {
    fn ensure_found(self, entity_name: &str, id: &Uuid) -> AppResult<T>;
}

impl<T> EnsureFound<T> for Option<T> {
    fn ensure_found(self, entity_name: &str, id: &Uuid) -> AppResult<T> {
        self.ok_or_else(|| AppError::not_found(format!("{}: {}", entity_name, id)))
    }
}
