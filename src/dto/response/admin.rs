use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::core::entity::{AuditLog, Role};
use crate::core::value_object::Module;

/// 模块信息（API 响应用，从 Module 枚举派生）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModuleInfo {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
}

impl From<&Module> for ModuleInfo {
    fn from(m: &Module) -> Self {
        Self {
            code: m.as_str().to_string(),
            name: m.display_name().to_string(),
            description: None, // 枚举不含 description，设为 None
            category: m.category().to_string(),
        }
    }
}

/// 角色列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleListResponse {
    pub roles: Vec<Role>,
    pub total: i64,
}

/// 创建角色请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateRoleRequest {
    #[validate(length(min = 1, max = 50, message = "角色名称长度1-50"))]
    pub name: String,
    pub description: Option<String>,
    /// 数据范围：all(全部), self(仅自己), department(科室)
    pub data_scope: Option<String>,
}

/// 更新角色请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    /// 数据范围：all(全部), self(仅自己), department(科室)
    pub data_scope: Option<String>,
}

/// 模块列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModuleListResponse {
    pub modules: Vec<ModuleInfo>,
}

/// 角色模块响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleModuleResponse {
    pub role_id: Uuid,
    pub modules: Vec<ModuleInfo>,
}

/// 分配模块请求（使用 module_code）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignModuleRequest {
    pub module_code: String,
}

/// 批量分配模块请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchAssignModulesRequest {
    pub module_codes: Vec<String>,
}

/// 设置角色模块请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetRoleModulesRequest {
    pub module_codes: Vec<String>,
}

/// 审计日志列表响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogListResponse {
    pub logs: Vec<AuditLog>,
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
