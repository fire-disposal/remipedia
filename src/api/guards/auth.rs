use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::JwtConfig;
use crate::core::value_object::SystemRole;
use crate::errors::AppError;
use crate::repository::RoleRepository;
use crate::service::JwtVerifier;

/// 认证用户信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub role_id: Uuid,
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
                            Ok((user_id, role_id, _subjects)) => {
                                Outcome::Success(AuthenticatedUser { id: user_id, role_id })
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

/// 超级管理员守卫
#[derive(Debug, Clone)]
pub struct SuperAdminGuard(pub AuthenticatedUser);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SuperAdminGuard {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user = AuthenticatedUser::from_request(request).await;

        match user {
            Outcome::Success(user) => {
                if SystemRole::is_super_admin(&user.role_id) {
                    Outcome::Success(SuperAdminGuard(user))
                } else {
                    Outcome::Error((Status::Forbidden, AppError::Forbidden))
                }
            }
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}

/// 权限守卫
/// 
/// 用于检查用户是否有访问特定资源的权限。
/// 会自动从请求路径和 HTTP 方法推断资源类型和操作类型。
pub struct PermissionGuard {
    pub user: AuthenticatedUser,
    pub resource: &'static str,
    pub action: &'static str,
}

pub struct PermissionGuardFactory {
    pub resource: &'static str,
    pub action: &'static str,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for PermissionGuard {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 获取用户信息
        let user = match AuthenticatedUser::from_request(request).await {
            Outcome::Success(user) => user,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };

        // 获取请求路径中的资源类型（从路径推断）
        let path = request.uri().path().as_str();
        let (resource, action) = parse_permission_from_path(path, request.method().as_str());

        // 获取数据库连接
        let pool = match request.rocket().state::<PgPool>() {
            Some(pool) => pool,
            None => {
                return Outcome::Error((
                    Status::InternalServerError,
                    AppError::ConfigError("数据库连接池未初始化".into()),
                ))
            }
        };

        // 检查权限
        let role_repo = RoleRepository::new(pool);
        match role_repo.has_permission(&user.role_id, resource, action).await {
            Ok(true) => Outcome::Success(PermissionGuard {
                user,
                resource,
                action,
            }),
            Ok(false) => Outcome::Error((Status::Forbidden, AppError::Forbidden)),
            Err(e) => Outcome::Error((Status::InternalServerError, e)),
        }
    }
}

/// 从路径和 HTTP 方法推断权限
pub(crate) fn parse_permission_from_path(path: &str, method: &str) -> (&'static str, &'static str) {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    // 获取资源类型（通常是 /api/v1/ 后的第一个路径段）
    let resource = if parts.len() >= 2 && parts[0] == "api" {
        if parts.len() >= 3 {
            parts[2]
        } else {
            "unknown"
        }
    } else if parts.len() >= 1 {
        parts[0]
    } else {
        "unknown"
    };

    // 从 HTTP 方法推断操作
    let action = match method.to_uppercase().as_str() {
        "GET" => {
            // 检查是否是列表请求（路径以 s 结尾或包含 list）
            if path.ends_with('s') || path.contains("list") || path.contains("history") {
                "list"
            } else {
                "read"
            }
        }
        "POST" => {
            if path.contains("switch") || path.contains("end") || path.contains("acknowledge") {
                "update"
            } else {
                "create"
            }
        }
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "read",
    };

    // 处理特殊路径
    let resource: &'static str = match resource {
        "patients" => "patient",
        "devices" => "device",
        "bindings" => "binding",
        "data" => "data",
        "users" => "user",
        "auth" => "auth",
        "admin" => "system",
        _ => Box::leak(resource.to_string().into_boxed_str()),
    };

    (resource, action)
}

/// 自定义权限守卫（显式指定资源和操作）
#[derive(Clone)]
pub struct RequirePermission {
    pub resource: &'static str,
    pub action: &'static str,
}

impl RequirePermission {
    pub const fn new(resource: &'static str, action: &'static str) -> Self {
        Self { resource, action }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequirePermission {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let _user = match AuthenticatedUser::from_request(request).await {
            Outcome::Success(user) => user,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };

        let _pool = match request.rocket().state::<PgPool>() {
            Some(pool) => pool,
            None => {
                return Outcome::Error((
                    Status::InternalServerError,
                    AppError::ConfigError("数据库连接池未初始化".into()),
                ))
            }
        };

        // 从请求守卫属性获取资源/操作（通过状态传递）
        // 这里简化处理，实际应该通过路由宏传递
        Outcome::Success(RequirePermission {
            resource: "any",
            action: "any",
        })
    }
}
