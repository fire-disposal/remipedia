use rocket::http::Status;
use crate::dto::response::{ModuleListResponse, ModuleResponse, BatchAssignModulesRequest, SetRoleModulesRequest};
use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put};
use sqlx::PgPool;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::api::guards::SystemRoleGuard;
use crate::dto::response::{
    AssignModuleRequest, AuditLogListResponse, AuditLogResponse,
    CreateRoleRequest, RoleListResponse, RoleModuleResponse,
    RoleResponse, UpdateRoleRequest,
};
use crate::errors::{AppError, AppResult};
use crate::service::AdminService;

/// 列出所有角色
#[utoipa::path(
    get,
    path = "/admin/roles",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "查询成功", body = RoleListResponse),
        (status = 403, description = "无权限"),
    )
)]
#[get("/admin/roles")]
pub async fn list_roles(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
) -> AppResult<Json<RoleListResponse>> {
    let service = AdminService::new(pool);
    let response = service.list_roles().await?;
    Ok(Json(response))
}

/// 获取角色详情
#[utoipa::path(
    get,
    path = "/admin/roles/{id}",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID")
    ),
    responses(
        (status = 200, description = "获取成功", body = RoleResponse),
        (status = 404, description = "角色不存在"),
        (status = 403, description = "无权限"),
    )
)]
#[get("/admin/roles/<id>")]
pub async fn get_role(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
) -> AppResult<Json<RoleResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let service = AdminService::new(pool);
    let response = service.get_role(&id).await?;
    Ok(Json(response))
}

/// 创建角色
#[utoipa::path(
    post,
    path = "/admin/roles",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "创建成功", body = RoleResponse),
        (status = 400, description = "验证失败"),
        (status = 403, description = "无权限"),
    )
)]
#[post("/admin/roles", data = "<req>")]
pub async fn create_role(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    req: Json<CreateRoleRequest>,
) -> AppResult<(Status, Json<RoleResponse>)> {
    let service = AdminService::new(pool);
    let response = service.create_role(req.name.clone(), req.description.clone()).await?;
    Ok((Status::Created, Json(response)))
}

/// 更新角色
#[utoipa::path(
    put,
    path = "/admin/roles/{id}",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID")
    ),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "更新成功", body = RoleResponse),
        (status = 404, description = "角色不存在"),
        (status = 400, description = "验证失败"),
        (status = 403, description = "无权限"),
    )
)]
#[put("/admin/roles/<id>", data = "<req>")]
pub async fn update_role(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
    req: Json<UpdateRoleRequest>,
) -> AppResult<Json<RoleResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let service = AdminService::new(pool);
    let response = service.update_role(&id, req.name.clone(), req.description.clone()).await?;
    Ok(Json(response))
}

/// 删除角色
#[utoipa::path(
    delete,
    path = "/admin/roles/{id}",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID")
    ),
    responses(
        (status = 204, description = "删除成功"),
        (status = 404, description = "角色不存在"),
        (status = 400, description = "不能删除系统角色"),
        (status = 403, description = "无权限"),
    )
)]
#[delete("/admin/roles/<id>")]
pub async fn delete_role(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
) -> AppResult<Status> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let service = AdminService::new(pool);
    service.delete_role(&id).await?;
    Ok(Status::NoContent)
}

/// 获取角色模块权限
#[utoipa::path(
    get,
    path = "/admin/roles/{id}/modules",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID")
    ),
    responses(
        (status = 200, description = "获取成功", body = RoleModuleResponse),
        (status = 404, description = "角色不存在"),
        (status = 403, description = "无权限"),
    )
)]
#[get("/admin/roles/<id>/modules")]
pub async fn get_role_modules(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
) -> AppResult<Json<RoleModuleResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let service = AdminService::new(pool);
    let response = service.get_role_modules(&id).await?;
    Ok(Json(response))
}

/// 为角色分配模块权限
#[utoipa::path(
    post,
    path = "/admin/roles/{id}/modules",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID")
    ),
    request_body = AssignModuleRequest,
    responses(
        (status = 204, description = "分配成功"),
        (status = 404, description = "角色或模块不存在"),
        (status = 403, description = "无权限"),
    )
)]
#[post("/admin/roles/<id>/modules", data = "<req>")]
pub async fn assign_module(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
    req: Json<AssignModuleRequest>,
) -> AppResult<Status> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let service = AdminService::new(pool);
    service.assign_module(&id, &req.module_id).await?;
    Ok(Status::NoContent)
}

/// 移除角色模块权限
#[utoipa::path(
    delete,
    path = "/admin/roles/{id}/modules/{module_id}",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID"),
        ("module_id" = String, Path, description = "模块ID")
    ),
    responses(
        (status = 204, description = "移除成功"),
        (status = 404, description = "角色不存在"),
        (status = 403, description = "无权限"),
    )
)]
#[delete("/admin/roles/<id>/modules/<module_id>")]
pub async fn revoke_module(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
    module_id: &str,
) -> AppResult<Status> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let module_id = Uuid::parse_str(module_id)
        .map_err(|_| AppError::ValidationError("无效的模块ID".into()))?;
    let service = AdminService::new(pool);
    service.revoke_module(&id, &module_id).await?;
    Ok(Status::NoContent)
}

/// 批量分配模块权限
#[utoipa::path(
    post,
    path = "/admin/roles/{id}/modules/batch",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID")
    ),
    request_body = BatchAssignModulesRequest,
    responses(
        (status = 204, description = "分配成功"),
        (status = 404, description = "角色不存在"),
        (status = 403, description = "无权限"),
    )
)]
#[post("/admin/roles/<id>/modules/batch", data = "<req>")]
pub async fn batch_assign_modules(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
    req: Json<BatchAssignModulesRequest>,
) -> AppResult<Status> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let service = AdminService::new(pool);
    service.batch_assign_modules(&id, &req.module_ids).await?;
    Ok(Status::NoContent)
}

/// 设置角色模块权限（替换）
#[utoipa::path(
    put,
    path = "/admin/roles/{id}/modules",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "角色ID")
    ),
    request_body = SetRoleModulesRequest,
    responses(
        (status = 204, description = "设置成功"),
        (status = 404, description = "角色不存在"),
        (status = 403, description = "无权限"),
    )
)]
#[put("/admin/roles/<id>/modules", data = "<req>")]
pub async fn set_role_modules(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
    req: Json<SetRoleModulesRequest>,
) -> AppResult<Status> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;
    let service = AdminService::new(pool);
    service.set_role_modules(&id, &req.module_ids).await?;
    Ok(Status::NoContent)
}

/// 列出所有模块
#[utoipa::path(
    get,
    path = "/admin/modules",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "查询成功", body = ModuleListResponse),
        (status = 403, description = "无权限"),
    )
)]
#[get("/admin/modules")]
pub async fn list_modules(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
) -> AppResult<Json<ModuleListResponse>> {
    let service = AdminService::new(pool);
    let response = service.list_modules().await?;
    Ok(Json(response))
}

/// 查询审计日志
#[utoipa::path(
    get,
    path = "/admin/audit-logs",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("user_id" = Option<String>, Query, description = "用户ID"),
        ("action" = Option<String>, Query, description = "操作类型"),
        ("resource" = Option<String>, Query, description = "资源类型"),
        ("status" = Option<String>, Query, description = "状态"),
        ("start_time" = Option<String>, Query, description = "开始时间"),
        ("end_time" = Option<String>, Query, description = "结束时间"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "查询成功", body = AuditLogListResponse),
        (status = 403, description = "无权限"),
    )
)]
#[get("/admin/audit-logs?<user_id>&<action>&<resource>&<status>&<start_time>&<end_time>&<page>&<page_size>")]
pub async fn list_audit_logs(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    user_id: Option<String>,
    action: Option<String>,
    resource: Option<String>,
    status: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<AuditLogListResponse>> {
    let service = AdminService::new(pool);
    
    let user_id = user_id.and_then(|id| Uuid::parse_str(&id).ok());
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    
    // 解析时间字符串
    let start_time = start_time.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });
    let end_time = end_time.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });
    
    let response = service
        .query_audit_logs(user_id, action, resource, status, start_time, end_time, page, page_size)
        .await?;
    Ok(Json(response))
}

/// 获取审计日志详情
#[utoipa::path(
    get,
    path = "/admin/audit-logs/{id}",
    tag = "admin",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "审计日志ID")
    ),
    responses(
        (status = 200, description = "获取成功", body = AuditLogResponse),
        (status = 404, description = "审计日志不存在"),
        (status = 403, description = "无权限"),
    )
)]
#[get("/admin/audit-logs/<id>")]
pub async fn get_audit_log(
    pool: &State<PgPool>,
    _guard: SystemRoleGuard,
    id: &str,
) -> AppResult<Json<AuditLogResponse>> {
    let id = Uuid::parse_str(id).map_err(|_| AppError::ValidationError("无效的日志ID".into()))?;
    let service = AdminService::new(pool);
    let response = service.get_audit_log(&id).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list_roles,
        get_role,
        create_role,
        update_role,
        delete_role,
        get_role_modules,
        set_role_modules,
        assign_module,
        batch_assign_modules,
        revoke_module,
        list_modules,
        list_audit_logs,
        get_audit_log,
    ]
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_roles,
        get_role,
        create_role,
        update_role,
        delete_role,
        get_role_modules,
        set_role_modules,
        assign_module,
        batch_assign_modules,
        revoke_module,
        list_modules,
        list_audit_logs,
        get_audit_log,
    ),
    components(
        schemas(
            RoleResponse,
            RoleListResponse,
            CreateRoleRequest,
            UpdateRoleRequest,
            ModuleResponse,
            ModuleListResponse,
            RoleModuleResponse,
            AssignModuleRequest,
            BatchAssignModulesRequest,
            SetRoleModulesRequest,
            AuditLogResponse,
            AuditLogListResponse,
        )
    )
)]
pub struct AdminApiDoc;
