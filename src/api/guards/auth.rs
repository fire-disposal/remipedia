use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use uuid::Uuid;

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
                        // 验证 token
                        match verify_token(token, config) {
                            Ok(user) => Outcome::Success(user),
                            Err(e) => Outcome::Error((Status::Unauthorized, e)),
                        }
                    }
                    None => Outcome::Error((Status::InternalServerError, AppError::ConfigError("JWT 配置缺失".into()))),
                }
            }
            _ => Outcome::Error((Status::Unauthorized, AppError::Unauthorized("缺少认证信息".into()))),
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

/// 验证 Token
fn verify_token(token: &str, _config: &JwtConfig) -> Result<AuthenticatedUser, AppError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 2 {
        return Err(AppError::Unauthorized("无效的 Token".into()));
    }

    let payload = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, parts[0])
        .map_err(|_| AppError::Unauthorized("无效的 Token".into()))?;
    
    let payload_str = String::from_utf8(payload)
        .map_err(|_| AppError::Unauthorized("无效的 Token".into()))?;
    
    let parts: Vec<&str> = payload_str.split(':').collect();
    if parts.len() != 3 {
        return Err(AppError::Unauthorized("无效的 Token".into()));
    }

    let user_id = Uuid::parse_str(parts[0])
        .map_err(|_| AppError::Unauthorized("无效的 Token".into()))?;
    
    let role = UserRole::from_str(parts[1])
        .ok_or_else(|| AppError::Unauthorized("无效的 Token".into()))?;

    let expires_at: i64 = parts[2].parse()
        .map_err(|_| AppError::Unauthorized("无效的 Token".into()))?;
    
    if chrono::Utc::now().timestamp() > expires_at {
        return Err(AppError::Unauthorized("Token 已过期".into()));
    }

    Ok(AuthenticatedUser { id: user_id, role })
}