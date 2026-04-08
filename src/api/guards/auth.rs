use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use uuid::Uuid;

use crate::config::JwtConfig;
use crate::core::value_object::Module;
use crate::errors::AppError;
use crate::service::JwtVerifier;

/// 认证用户信息（基础守卫）
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub role_id: Uuid,
    /// 是否为系统角色（拥有通配权限）
    pub is_system_role: bool,
    /// 可访问模块列表
    pub accessible_modules: Vec<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = request.headers().get_one("Authorization");

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header[7..];
                let jwt_config = request.rocket().state::<JwtConfig>();

                match jwt_config {
                    Some(config) => {
                        let verifier = JwtVerifier::new(config);
                        match verifier.verify_access_token(token) {
                            Ok((user_id, role_id, is_system_role, modules)) => {
                                Outcome::Success(Self {
                                    id: user_id,
                                    role_id,
                                    is_system_role,
                                    accessible_modules: modules,
                                })
                            }
                            Err(e) => Outcome::Error((Status::Unauthorized, e)),
                        }
                    }
                    None => Outcome::Error((
                        Status::InternalServerError,
                        AppError::ConfigError("JWT 配置缺失".into()),
                    )),
                }
            }
            _ => Outcome::Error((
                Status::Unauthorized,
                AppError::Unauthorized("缺少认证信息".into()),
            )),
        }
    }
}

/// 系统角色守卫（拥有通配权限）
/// 
/// 用于管理功能，如角色管理、审计日志等
#[derive(Debug, Clone)]
pub struct SystemRoleGuard(pub AuthenticatedUser);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SystemRoleGuard {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user = AuthenticatedUser::from_request(request).await;

        match user {
            Outcome::Success(user) => {
                if user.is_system_role {
                    Outcome::Success(Self(user))
                } else {
                    Outcome::Error((Status::Forbidden, AppError::Forbidden))
                }
            }
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}

/// 模块权限守卫
/// 
/// 检查用户是否有访问特定模块的权限
#[derive(Debug, Clone)]
pub struct ModuleGuard {
    pub user: AuthenticatedUser,
    pub module: Module,
}

impl ModuleGuard {
    /// 检查用户是否有权限访问指定模块
    pub fn can_access(&self, module: Module) -> bool {
        // 系统角色拥有所有权限
        if self.user.is_system_role {
            return true;
        }
        // 检查模块是否在可访问列表中
        self.user.accessible_modules.contains(&module.as_str().to_string())
    }
}

/// 模块守卫工厂（用于从请求推断模块）
pub struct ModuleGuardFactory {
    pub module: Module,
}

impl ModuleGuardFactory {
    pub const fn new(module: Module) -> Self {
        Self { module }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ModuleGuard {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 获取用户信息
        let user = match AuthenticatedUser::from_request(request).await {
            Outcome::Success(user) => user,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };

        // 系统角色直接通过，不检查具体模块
        if user.is_system_role {
            return Outcome::Success(Self {
                user,
                module: Module::Dashboard, // 占位，实际不限制
            });
        }

        // 从请求路径推断模块
        let path = request.uri().path().as_str();
        let module = parse_module_from_path(path);

        // 检查是否有权限
        if user.accessible_modules.contains(&module.as_str().to_string()) {
            Outcome::Success(Self { user, module })
        } else {
            Outcome::Error((Status::Forbidden, AppError::Forbidden))
        }
    }
}

/// 从路径推断模块
fn parse_module_from_path(path: &str) -> Module {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    // 获取路径中的模块标识
    let module_code = if parts.len() >= 2 && parts[0] == "api" {
        if parts.len() >= 3 {
            parts[2]
        } else {
            "dashboard"
        }
    } else if !parts.is_empty() {
        parts[0]
    } else {
        "dashboard"
    };

    // 映射到模块枚举
    match module_code {
        "patients" => Module::Patients,
        "devices" => Module::Devices,
        "bindings" => Module::Bindings,
        "data" => Module::Data,
        "users" => Module::Users,
        "admin" => {
            // admin 路径需要进一步判断
            if parts.len() >= 4 {
                match parts[3] {
                    "roles" | "permissions" => Module::Roles,
                    "audit-logs" => Module::AuditLogs,
                    _ => Module::Settings,
                }
            } else {
                Module::Settings
            }
        }
        "settings" => Module::Settings,
        "pressure-ulcer" => Module::PressureUlcer,
        _ => Module::Dashboard,
    }
}

/// 显式模块守卫（指定模块）
/// 
/// 用法示例：
/// ```rust
/// #[get("/patients")]
/// async fn list_patients(
///     _guard: ExplicitModuleGuard<{ Module::Patients }>,
/// ) -> Json<...>
/// ```
#[derive(Clone)]
pub struct ExplicitModuleGuard {
    pub user: AuthenticatedUser,
    pub module: Module,
}

impl ExplicitModuleGuard {
    pub fn new(user: AuthenticatedUser, module: Module) -> Self {
        Self { user, module }
    }
    
    /// 创建守卫检查函数（用于路由宏）
    pub fn check(user: &AuthenticatedUser, module: Module) -> bool {
        if user.is_system_role {
            return true;
        }
        user.accessible_modules.contains(&module.as_str().to_string())
    }
}

// 保留旧守卫以兼容（标记为 deprecated）
/// 超级管理员守卫（已废弃，请使用 SystemRoleGuard）
#[deprecated(since = "0.2.0", note = "请使用 SystemRoleGuard 替代")]
pub type SuperAdminGuard = SystemRoleGuard;

/// 权限守卫（已废弃，请使用 ModuleGuard）
#[deprecated(since = "0.2.0", note = "请使用 ModuleGuard 替代")]
pub type PermissionGuard = ModuleGuard;

/// 自定义权限守卫（已废弃）
#[deprecated(since = "0.2.0", note = "功能已移除")]
#[derive(Clone)]
pub struct RequirePermission {
    pub resource: &'static str,
    pub action: &'static str,
}

#[allow(deprecated)]
impl RequirePermission {
    pub const fn new(resource: &'static str, action: &'static str) -> Self {
        Self { resource, action }
    }
}
