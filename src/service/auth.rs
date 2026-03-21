use argon2::{password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::config::JwtConfig;
use crate::core::value_object::UserRole;
use crate::dto::request::{ChangePasswordRequest, LoginRequest};
use crate::dto::response::{LoginResponse, UserInfo};
use crate::errors::{AppError, AppResult};
use crate::repository::UserRepository;

pub struct AuthService<'a> {
    user_repo: UserRepository<'a>,
    jwt_config: &'a JwtConfig,
}

impl<'a> AuthService<'a> {
    pub fn new(pool: &'a PgPool, jwt_config: &'a JwtConfig) -> Self {
        Self {
            user_repo: UserRepository::new(pool),
            jwt_config,
        }
    }

    /// 用户登录
    #[instrument(skip(self), fields(username = %req.username))]
    pub async fn login(&self, req: LoginRequest) -> AppResult<LoginResponse> {
        // 查找用户
        let user = self.user_repo.find_by_username(&req.username).await?
            .ok_or(AppError::Unauthorized("用户名或密码错误".into()))?;

        // 验证密码
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| AppError::InternalError)?;
        
        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::InvalidPassword)?;

        // 检查用户状态
        if user.status != "active" {
            return Err(AppError::Unauthorized("用户已被禁用".into()));
        }

        // 更新最后登录时间
        self.user_repo.update_last_login(&user.id).await?;

        // 生成 JWT
        let expires_at = Utc::now() + Duration::hours(self.jwt_config.expiration_hours as i64);
        let token = self.generate_token(&user.id, &user.role, expires_at)?;

        info!(user_id = %user.id, "用户登录成功");

        Ok(LoginResponse {
            success: true,
            token,
            user: UserInfo {
                id: user.id.to_string(),
                username: user.username,
                role: user.role,
            },
            expires_at,
        })
    }

    /// 修改密码
    #[instrument(skip(self))]
    pub async fn change_password(&self, user_id: &Uuid, req: ChangePasswordRequest) -> AppResult<()> {
        let user = self.user_repo.find_by_id(user_id).await?;

        // 验证旧密码
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| AppError::InternalError)?;
        
        Argon2::default()
            .verify_password(req.old_password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::InvalidPassword)?;

        // 哈希新密码
        let new_hash = Self::hash_password(&req.new_password)?;
        
        // 更新密码
        self.user_repo.update_password(user_id, &new_hash).await?;

        info!(user_id = %user_id, "密码修改成功");

        Ok(())
    }

    /// 哈希密码
    pub fn hash_password(password: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        argon2.hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AppError::InternalError)
    }

    /// 生成 JWT Token
    fn generate_token(&self, user_id: &Uuid, role: &str, expires_at: chrono::DateTime<Utc>) -> AppResult<String> {
        // 简单实现：使用 base64 编码（生产环境应使用真正的 JWT 库如 jsonwebtoken）
        let payload = format!("{}:{}:{}", user_id, role, expires_at.timestamp());
        
        // 使用 HMAC-SHA256 签名
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        payload.hash(&mut hasher);
        let signature = hasher.finish();
        
        Ok(format!("{}.{}", 
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &payload),
            signature
        ))
    }

    /// 验证 Token
    pub fn verify_token(&self, token: &str) -> AppResult<(Uuid, UserRole)> {
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
        
        if Utc::now().timestamp() > expires_at {
            return Err(AppError::Unauthorized("Token 已过期".into()));
        }

        Ok((user_id, role))
    }
}