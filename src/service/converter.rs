//! Service转换工具
//!
//! 提供DTO转换的依赖实现

use crate::errors::AppResult;
use crate::repository::RoleRepository;
use sqlx::PgPool;
use std::collections::HashMap;

/// Service转换器
///
/// 封装转换所需的Repository
pub struct ServiceConverter<'a> {
    role_repo: RoleRepository<'a>,
}

impl<'a> ServiceConverter<'a> {
    /// 创建新的转换器实例
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            role_repo: RoleRepository::new(pool),
        }
    }

    /// 获取角色Repository的引用
    pub fn role_repo(&self) -> &RoleRepository<'a> {
        &self.role_repo
    }

    /// 批量获取角色名称
    ///
    /// 优化：先收集所有唯一的role_id，然后批量查询
    pub async fn get_role_names(
        &self,
        role_ids: &[uuid::Uuid],
    ) -> AppResult<HashMap<uuid::Uuid, String>> {
        let mut names = HashMap::new();
        for role_id in role_ids {
            if let Ok(Some(role)) = self.role_repo.find_by_id(role_id).await {
                names.insert(*role_id, role.name);
            }
        }
        Ok(names)
    }

    /// 获取单个角色名称
    pub async fn get_role_name(
        &self,
        role_id: &uuid::Uuid,
    ) -> AppResult<String> {
        match self.role_repo.find_by_id(role_id).await? {
            Some(role) => Ok(role.name),
            None => Ok("unknown".to_string()),
        }
    }
}