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

impl AuthenticatedUser {
    /// 检查当前用户是否为系统角色，否则返回 403
    pub fn check_system_role(&self) -> Result<(), AppError> {
        if self.is_system_role {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    /// 检查当前用户是否有权访问指定模块
    pub fn can_access_module(&self, module: &Module) -> bool {
        self.is_system_role || self.accessible_modules.contains(&module.as_str().to_string())
    }
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
                        AppError::InternalError("JWT 配置缺失".into()),
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

/// 显式模块守卫（指定模块）
///
/// 用法示例：
/// ```ignore
/// let user = AuthenticatedUser::from_request(request).await;
/// if !ExplicitModuleGuard::check(&user, Module::Patients) {
///     return Err(AppError::Forbidden);
/// }
/// ```
#[derive(Clone)]
pub struct ExplicitModuleGuard;

impl ExplicitModuleGuard {
    /// 检查用户是否有权访问指定模块
    pub fn check(user: &AuthenticatedUser, module: Module) -> bool {
        user.can_access_module(&module)
    }
}
