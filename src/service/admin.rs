use crate::core::entity::{AuditLog, AuditLogQuery, NewAuditLog, NewRole, Role, UpdateRole};
use crate::dto::response::{
    AuditLogListResponse, ModuleListResponse, RoleListResponse, RoleModuleResponse,
};
use crate::errors::{AppError, AppResult};
use crate::repository::{AuditLogRepository, EnsureFound, ModulePermissionRepository, RoleRepository};
use sqlx::PgPool;
use uuid::Uuid;

pub struct AdminService<'a> {
    role_repo: RoleRepository<'a>,
    module_perm_repo: ModulePermissionRepository<'a>,
    audit_log_repo: AuditLogRepository<'a>,
}

impl<'a> AdminService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            role_repo: RoleRepository::new(pool),
            module_perm_repo: ModulePermissionRepository::new(pool),
            audit_log_repo: AuditLogRepository::new(pool),
        }
    }

    // ===== 角色管理 =====

    pub async fn list_roles(&self) -> AppResult<RoleListResponse> {
        let roles = self.role_repo.list_all().await?;
        let total = roles.len() as i64;

        Ok(RoleListResponse { roles, total })
    }

    pub async fn get_role(&self, id: &Uuid) -> AppResult<Role> {
        let role = self.role_repo.find_by_id(id).await?.ensure_found("角色", id)?;
        Ok(role)
    }

    pub async fn create_role(
        &self,
        name: String,
        description: Option<String>,
    ) -> AppResult<Role> {
        // 检查角色名是否已存在
        if let Some(_) = self.role_repo.find_by_name(&name).await? {
            return Err(AppError::ValidationError("角色名称已存在".into()));
        }

        let role = self
            .role_repo
            .create(&NewRole { name, description })
            .await?;

        Ok(role)
    }

    pub async fn update_role(
        &self,
        id: &Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> AppResult<Role> {
        // 检查角色是否存在
        let role = self.role_repo.find_by_id(id).await?.ensure_found("角色", id)?;

        // 检查是否是系统角色
        if role.is_system {
            return Err(AppError::ValidationError("不能修改系统角色".into()));
        }

        // 如果更新名称，检查是否已存在
        if let Some(ref new_name) = name {
            if let Some(existing) = self.role_repo.find_by_name(new_name).await? {
                if existing.id != *id {
                    return Err(AppError::ValidationError("角色名称已存在".into()));
                }
            }
        }

        let role = self
            .role_repo
            .update(id, &UpdateRole { name, description })
            .await?;

        Ok(role)
    }

    pub async fn delete_role(&self, id: &Uuid) -> AppResult<()> {
        // 检查角色是否存在
        let role = self.role_repo.find_by_id(id).await?.ensure_found("角色", id)?;

        // 检查是否是系统角色
        if role.is_system {
            return Err(AppError::ValidationError("不能删除系统角色".into()));
        }

        self.role_repo.delete(id).await?;
        Ok(())
    }

    // ===== 模块管理 =====

    pub async fn list_modules(&self) -> AppResult<ModuleListResponse> {
        let modules = self.module_perm_repo.list_all_modules().await?;
        Ok(ModuleListResponse { modules })
    }

    pub async fn get_role_modules(&self, role_id: &Uuid) -> AppResult<RoleModuleResponse> {
        self.role_repo.find_by_id(role_id).await?.ensure_found("角色", role_id)?;

        let modules = self.module_perm_repo.get_role_modules(role_id).await?;
        Ok(RoleModuleResponse {
            role_id: *role_id,
            modules,
        })
    }

    pub async fn assign_module(&self, role_id: &Uuid, module_id: &Uuid) -> AppResult<()> {
        self.role_repo.find_by_id(role_id).await?.ensure_found("角色", role_id)?;

        if !self.module_perm_repo.module_exists(module_id).await? {
            return Err(AppError::NotFound(format!("模块: {}", module_id)));
        }

        self.module_perm_repo.assign_module(role_id, module_id).await?;
        Ok(())
    }

    pub async fn revoke_module(&self, role_id: &Uuid, module_id: &Uuid) -> AppResult<()> {
        self.role_repo.find_by_id(role_id).await?.ensure_found("角色", role_id)?;

        self.module_perm_repo.revoke_module(role_id, module_id).await?;
        Ok(())
    }

    pub async fn batch_assign_modules(&self, role_id: &Uuid, module_ids: &[Uuid]) -> AppResult<()> {
        self.role_repo.find_by_id(role_id).await?.ensure_found("角色", role_id)?;

        for module_id in module_ids {
            if self.module_perm_repo.module_exists(module_id).await? {
                self.module_perm_repo.assign_module(role_id, module_id).await?;
            }
        }

        Ok(())
    }

    pub async fn batch_revoke_modules(&self, role_id: &Uuid, module_ids: &[Uuid]) -> AppResult<()> {
        self.role_repo.find_by_id(role_id).await?.ensure_found("角色", role_id)?;

        for module_id in module_ids {
            self.module_perm_repo.revoke_module(role_id, module_id).await?;
        }

        Ok(())
    }

    pub async fn set_role_modules(&self, role_id: &Uuid, module_ids: &[Uuid]) -> AppResult<()> {
        self.role_repo.find_by_id(role_id).await?.ensure_found("角色", role_id)?;

        let current_modules = self.module_perm_repo.get_role_module_ids(role_id).await?;

        let current_set: std::collections::HashSet<Uuid> = current_modules.into_iter().collect();
        let new_set: std::collections::HashSet<Uuid> = module_ids.iter().cloned().collect();

        let to_add: Vec<Uuid> = new_set.difference(&current_set).cloned().collect();
        let to_remove: Vec<Uuid> = current_set.difference(&new_set).cloned().collect();

        for module_id in to_add {
            if self.module_perm_repo.module_exists(&module_id).await? {
                self.module_perm_repo.assign_module(role_id, &module_id).await?;
            }
        }

        for module_id in to_remove {
            self.module_perm_repo.revoke_module(role_id, &module_id).await?;
        }

        Ok(())
    }

    // ===== 审计日志 =====

    pub async fn query_audit_logs(
        &self,
        user_id: Option<Uuid>,
        action: Option<String>,
        resource: Option<String>,
        status: Option<String>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        page: u32,
        page_size: u32,
    ) -> AppResult<AuditLogListResponse> {
        let query = AuditLogQuery {
            user_id,
            action,
            resource,
            status,
            start_time,
            end_time,
            page,
            page_size,
        };

        let (logs, total) = self.audit_log_repo.query(&query).await?;

        Ok(AuditLogListResponse {
            logs,
            total,
            page,
            page_size,
        })
    }

    pub async fn get_audit_log(&self, id: &Uuid) -> AppResult<AuditLog> {
        let log = self.audit_log_repo.find_by_id(id).await?.ensure_found("审计日志", id)?;
        Ok(log)
    }

    pub async fn create_audit_log(&self, log: NewAuditLog) -> AppResult<AuditLog> {
        let log = self.audit_log_repo.create(&log).await?;
        Ok(log)
    }
}
