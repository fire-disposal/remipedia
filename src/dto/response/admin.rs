use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// 角色响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 角色列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleListResponse {
    pub roles: Vec<RoleResponse>,
    pub total: i64,
}

/// 创建角色请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateRoleRequest {
    #[validate(length(min = 1, max = 50, message = "角色名称长度1-50"))]
    pub name: String,
    pub description: Option<String>,
}

/// 更新角色请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// 模块响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModuleResponse {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// 模块列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModuleListResponse {
    pub modules: Vec<ModuleResponse>,
}

/// 角色模块响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleModuleResponse {
    pub role_id: Uuid,
    pub modules: Vec<ModuleResponse>,
}

/// 分配模块请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignModuleRequest {
    pub module_id: Uuid,
}

/// 批量分配模块请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchAssignModulesRequest {
    pub module_ids: Vec<Uuid>,
}

/// 设置角色模块请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetRoleModulesRequest {
    pub module_ids: Vec<Uuid>,
}

/// 审计日志响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// 审计日志列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogListResponse {
    pub logs: Vec<AuditLogResponse>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// 审计日志查询参数
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogQueryParams {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub resource: Option<String>,
    pub status: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
