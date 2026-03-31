use crate::core::entity::{AuditLogQuery, NewAuditLog, NewRole, Permission, Role, UpdateRole};
use crate::dto::response::{AuditLogListResponse, AuditLogResponse, PermissionResponse, RoleListResponse, RolePermissionResponse, RoleResponse};
use crate::errors::{AppError, AppResult};
use crate::repository::{AuditLogRepository, PermissionRepository, RoleRepository};
use sqlx::PgPool;
use uuid::Uuid;

pub struct AdminService<'a> {
    role_repo: RoleRepository<'a>,
    permission_repo: PermissionRepository<'a>,
    audit_log_repo: AuditLogRepository<'a>,
}

impl<'a> AdminService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            role_repo: RoleRepository::new(pool),
            permission_repo: PermissionRepository::new(pool),
            audit_log_repo: AuditLogRepository::new(pool),
        }
    }

    // ===== 角色管理 =====

    pub async fn list_roles(&self) -> AppResult<RoleListResponse> {
        let roles = self.role_repo.list_all().await?;
        let total = roles.len() as i64;
        
        Ok(RoleListResponse {
            roles: roles.into_iter().map(|r| r.into()).collect(),
            total,
        })
    }

    pub async fn get_role(&self, id: &Uuid) -> AppResult<RoleResponse> {
        let role = self.role_repo.find_by_id(id).await?;
        match role {
            Some(role) => Ok(role.into()),
            None => Err(AppError::NotFound(format!("角色: {}", id))),
        }
    }

    pub async fn create_role(
        &self,
        name: String,
        description: Option<String>,
    ) -> AppResult<RoleResponse> {
        // 检查角色名是否已存在
        if let Some(_) = self.role_repo.find_by_name(&name).await? {
            return Err(AppError::ValidationError("角色名称已存在".into()));
        }

        let role = self
            .role_repo
            .create(&NewRole { name, description })
            .await?;

        Ok(role.into())
    }

    pub async fn update_role(
        &self,
        id: &Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> AppResult<RoleResponse> {
        // 检查角色是否存在
        let existing = self.role_repo.find_by_id(id).await?;
        if existing.is_none() {
            return Err(AppError::NotFound(format!("角色: {}", id)));
        }

        // 检查是否是系统角色
        if let Some(ref role) = existing {
            if role.is_system {
                return Err(AppError::ValidationError("不能修改系统角色".into()));
            }
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

        Ok(role.into())
    }

    pub async fn delete_role(&self, id: &Uuid) -> AppResult<()> {
        // 检查角色是否存在
        let existing = self.role_repo.find_by_id(id).await?;
        if existing.is_none() {
            return Err(AppError::NotFound(format!("角色: {}", id)));
        }

        // 检查是否是系统角色
        if let Some(ref role) = existing {
            if role.is_system {
                return Err(AppError::ValidationError("不能删除系统角色".into()));
            }
        }

        self.role_repo.delete(id).await?;
        Ok(())
    }

    // ===== 权限管理 =====

    pub async fn list_permissions(&self) -> AppResult<Vec<PermissionResponse>> {
        let permissions = self.permission_repo.list_all().await?;
        Ok(permissions.into_iter().map(|p| p.into()).collect())
    }

    pub async fn get_role_permissions(
        &self,
        role_id: &Uuid,
    ) -> AppResult<RolePermissionResponse> {
        // 检查角色是否存在
        if self.role_repo.find_by_id(role_id).await?.is_none() {
            return Err(AppError::NotFound(format!("角色: {}", role_id)));
        }

        let permissions = self.role_repo.get_permissions(role_id).await?;
        Ok(RolePermissionResponse {
            role_id: *role_id,
            permissions: permissions.into_iter().map(|p| p.into()).collect(),
        })
    }

    pub async fn assign_permission(
        &self,
        role_id: &Uuid,
        permission_id: &Uuid,
    ) -> AppResult<()> {
        // 检查角色是否存在
        if self.role_repo.find_by_id(role_id).await?.is_none() {
            return Err(AppError::NotFound(format!("角色: {}", role_id)));
        }

        // 检查权限是否存在
        if self.permission_repo.find_by_id(permission_id).await?.is_none() {
            return Err(AppError::NotFound(format!("权限: {}", permission_id)));
        }

        self.role_repo
            .assign_permission(role_id, permission_id)
            .await?;
        Ok(())
    }

    pub async fn revoke_permission(
        &self,
        role_id: &Uuid,
        permission_id: &Uuid,
    ) -> AppResult<()> {
        // 检查角色是否存在
        if self.role_repo.find_by_id(role_id).await?.is_none() {
            return Err(AppError::NotFound(format!("角色: {}", role_id)));
        }

        self.role_repo
            .revoke_permission(role_id, permission_id)
            .await?;
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
            logs: logs.into_iter().map(|l| l.into()).collect(),
            total,
            page,
            page_size,
        })
    }

    pub async fn get_audit_log(&self,
        id: &Uuid
    ) -> AppResult<AuditLogResponse> {
        let log = self.audit_log_repo.find_by_id(id).await?;
        match log {
            Some(log) => Ok(log.into()),
            None => Err(AppError::NotFound(format!("审计日志: {}", id))),
        }
    }

    pub async fn create_audit_log(
        &self,
        log: NewAuditLog,
    ) -> AppResult<AuditLogResponse> {
        let log = self.audit_log_repo.create(&log).await?;
        Ok(log.into())
    }
}

// 转换实现
impl From<Role> for RoleResponse {
    fn from(role: Role) -> Self {
        Self {
            id: role.id,
            name: role.name,
            description: role.description,
            is_system: role.is_system,
            created_at: role.created_at,
            updated_at: role.updated_at,
        }
    }
}

impl From<Permission> for PermissionResponse {
    fn from(perm: Permission) -> Self {
        Self {
            id: perm.id,
            resource: perm.resource,
            action: perm.action,
            description: perm.description,
            created_at: perm.created_at,
        }
    }
}

impl From<crate::core::entity::AuditLog> for AuditLogResponse {
    fn from(log: crate::core::entity::AuditLog) -> Self {
        Self {
            id: log.id,
            user_id: log.user_id,
            action: log.action,
            resource: log.resource,
            resource_id: log.resource_id,
            details: log.details,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            status: log.status,
            error_message: log.error_message,
            duration_ms: log.duration_ms,
            created_at: log.created_at,
        }
    }
}
