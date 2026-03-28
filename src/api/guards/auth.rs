use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use uuid::Uuid;

use crate::application::auth::JwtVerifier;
use crate::config::JwtConfig;
use crate::core::value_object::UserRole;
use crate::errors::AppError;

/// 认证用户信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub role: UserRole,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 获取 Authorization header
        let auth_header = request.headers().get_one("Authorization");

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header[7..];

                // 获取 JWT 配置
                let jwt_config = request.rocket().state::<JwtConfig>();

                match jwt_config {
                    Some(config) => {
                        // 使用 JwtVerifier 验证 token
                        let verifier = JwtVerifier::new(config);
                        match verifier.verify_access_token(token) {
                            Ok((user_id, role)) => {
                                Outcome::Success(AuthenticatedUser { id: user_id, role })
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

/// 管理员用户守卫
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthenticatedUser);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminUser {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user = AuthenticatedUser::from_request(request).await;

        match user {
            Outcome::Success(user) => {
                if user.role.is_admin() {
                    Outcome::Success(AdminUser(user))
                } else {
                    Outcome::Error((Status::Forbidden, AppError::Forbidden))
                }
            }
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}

/// 基于权限的守卫
///
/// 用法：
/// ```rust
/// #[get("/devices")]
/// async fn list_devices(
///     guard: PermissionGuard,
///     user: AuthenticatedUser,
/// ) -> Result<Json<DeviceList>, AppError> {
///     guard.require("device:read")?;
///     // ...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PermissionGuard {
    user: AuthenticatedUser,
}

impl PermissionGuard {
    /// 检查用户是否有指定权限
    ///
    /// 注意：当前实现仅基于角色做简化检查。
    /// 完整实现需要从数据库查询用户的角色权限。
    pub fn require(&self, permission: &str) -> Result<(), AppError> {
        // 管理员拥有所有权限
        if self.user.role.is_admin() {
            return Ok(());
        }

        // 普通用户的基本读权限
        let allowed_permissions = ["device:read", "patient:read", "binding:read", "data:read"];
        
        if allowed_permissions.contains(&permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    /// 检查是否有任一权限
    pub fn require_any(&self, permissions: &[&str]) -> Result<(), AppError> {
        for &permission in permissions {
            if self.require(permission).is_ok() {
                return Ok(());
            }
        }
        Err(AppError::Forbidden)
    }

    /// 检查是否有所有权限
    pub fn require_all(&self, permissions: &[&str]) -> Result<(), AppError> {
        for &permission in permissions {
            self.require(permission)?;
        }
        Ok(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for PermissionGuard {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user = AuthenticatedUser::from_request(request).await;

        match user {
            Outcome::Success(user) => Outcome::Success(PermissionGuard { user }),
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}
