//! DTO转换trait
//!
//! 提供从Entity到Response DTO的统一转换机制

use crate::errors::AppResult;
use std::future::Future;

/// 实体转换为响应DTO的trait
///
/// 为实体类型实现此trait，提供统一的转换接口
pub trait IntoResponse: Send {
    /// 目标响应类型
    type Response: Send;

    /// 将实体转换为响应DTO
    fn into_response(
        self,
        role_repo: &crate::repository::RoleRepository<'_>,
    ) -> impl Future<Output = AppResult<Self::Response>> + Send;
}
