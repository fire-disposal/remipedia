use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use log::info;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::JwtConfig;
use crate::core::auth::Claims;
use crate::core::entity::NewRefreshToken;
use crate::core::value_object::UserRole;
use crate::dto::request::{ChangePasswordRequest, LoginRequest, RefreshTokenRequest};
use crate::dto::response::{LoginResponse, RefreshTokenResponse, UserInfo};
use crate::errors::{AppError, AppResult};
use crate::repository::{RefreshTokenRepository, UserRepository};

const JWT_ISSUER: &str = "remipedia";
const JWT_AUDIENCE: &str = "remipedia-api";

fn jwt_validation() -> Validation {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[JWT_ISSUER]);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation
}

/// JWT 验证器（仅用于验证 token，不需要数据库连接）
pub struct JwtVerifier<'a> {
    jwt_config: &'a JwtConfig,
}

impl<'a> JwtVerifier<'a> {
    pub fn new(jwt_config: &'a JwtConfig) -> Self {
        Self { jwt_config }
    }

    /// 验证 Access Token
    pub fn verify_access_token(&self, token: &str) -> AppResult<(Uuid, UserRole)> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_config.secret.as_bytes()),
            &jwt_validation(),
        )
        .map_err(|_| AppError::Unauthorized("无效的访问令牌".into()))?;

        let claims = token_data.claims;

        // 验证是否为 access token
        if !claims.is_access_token() {
            return Err(AppError::Unauthorized("无效的令牌类型".into()));
        }

        let user_id = claims.user_id()?;
        let role = UserRole::from_str(&claims.role)
            .ok_or_else(|| AppError::Unauthorized("无效的角色".into()))?;

        Ok((user_id, role))
    }
}

pub struct AuthService<'a> {
    user_repo: UserRepository<'a>,
    refresh_token_repo: RefreshTokenRepository<'a>,
    jwt_config: &'a JwtConfig,
}

impl<'a> AuthService<'a> {
    pub fn new(pool: &'a PgPool, jwt_config: &'a JwtConfig) -> Self {
        Self {
            user_repo: UserRepository::new(pool),
            refresh_token_repo: RefreshTokenRepository::new(pool),
            jwt_config,
        }
    }

    /// 用户登录
    pub async fn login(&self, req: LoginRequest) -> AppResult<LoginResponse> {
        // 查找用户
        let user = self
            .user_repo
            .find_by_username(&req.username)
            .await?
            .ok_or(AppError::Unauthorized("用户名或密码错误".into()))?;

        // 验证密码
        let parsed_hash =
            PasswordHash::new(&user.password_hash).map_err(|_| AppError::InternalError)?;

        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::InvalidPassword)?;

        // 检查用户状态
        if user.status != "active" {
            return Err(AppError::Unauthorized("用户已被禁用".into()));
        }

        // 更新最后登录时间
        self.user_repo.update_last_login(&user.id).await?;

        // 生成令牌
        let (access_token, expires_at) = self.generate_access_token(&user.id, &user.role)?;
        let refresh_token = self.generate_refresh_token(&user.id).await?;

        info!("用户登录成功: user_id={}", user.id);

        Ok(LoginResponse {
            success: true,
            access_token,
            refresh_token,
            user: UserInfo {
                id: user.id.to_string(),
                username: user.username,
                role: user.role,
            },
            expires_at,
        })
    }

    /// 刷新令牌
    pub async fn refresh_token(&self, req: RefreshTokenRequest) -> AppResult<RefreshTokenResponse> {
        // 验证 refresh token
        let claims = self.verify_refresh_token(&req.refresh_token)?;
        let token_hash = Self::hash_token(&req.refresh_token);

        // 校验 refresh token 是否存在且未撤销
        if !self.refresh_token_repo.is_valid(&token_hash).await? {
            return Err(AppError::Unauthorized("无效的刷新令牌".into()));
        }

        // 获取用户信息
        let user = self.user_repo.find_by_id(&claims.user_id()?).await?;

        // 检查用户状态
        if user.status != "active" {
            return Err(AppError::Unauthorized("用户已被禁用".into()));
        }

        // 撤销旧的 refresh token
        self.refresh_token_repo.revoke(&token_hash).await?;

        // 生成新的令牌
        let (access_token, expires_at) = self.generate_access_token(&user.id, &user.role)?;
        let refresh_token = self.generate_refresh_token(&user.id).await?;

        info!("令牌刷新成功: user_id={}", user.id);

        Ok(RefreshTokenResponse {
            success: true,
            access_token,
            refresh_token,
            expires_at,
        })
    }

    /// 修改密码
    pub async fn change_password(
        &self,
        user_id: &Uuid,
        req: ChangePasswordRequest,
    ) -> AppResult<()> {
        if req.old_password == req.new_password {
            return Err(AppError::ValidationError("新密码不能与旧密码相同".into()));
        }

        let user = self.user_repo.find_by_id(user_id).await?;

        // 验证旧密码
        let parsed_hash =
            PasswordHash::new(&user.password_hash).map_err(|_| AppError::InternalError)?;

        Argon2::default()
            .verify_password(req.old_password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::InvalidPassword)?;

        // 哈希新密码
        let new_hash = Self::hash_password(&req.new_password)?;

        // 更新密码
        self.user_repo.update_password(user_id, &new_hash).await?;

        // 撤销所有 refresh token（强制重新登录）
        self.refresh_token_repo.revoke_all_for_user(user_id).await?;

        info!("密码修改成功: user_id={}", user_id);

        Ok(())
    }

    /// 登出（撤销 refresh token）
    pub async fn logout(&self, refresh_token: &str) -> AppResult<()> {
        let token_hash = Self::hash_token(refresh_token);
        self.refresh_token_repo.revoke(&token_hash).await?;
        Ok(())
    }

    /// 哈希密码
    pub fn hash_password(password: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AppError::InternalError)
    }

    /// 生成 Access Token
    fn generate_access_token(
        &self,
        user_id: &Uuid,
        role: &str,
    ) -> AppResult<(String, chrono::DateTime<Utc>)> {
        let expires_at = Utc::now() + Duration::hours(self.jwt_config.expiration_hours as i64);
        let claims = Claims::new_access(user_id, role, expires_at, JWT_ISSUER);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_config.secret.as_bytes()),
        )
        .map_err(|_| AppError::InternalError)?;

        Ok((token, expires_at))
    }

    /// 生成 Refresh Token
    async fn generate_refresh_token(&self, user_id: &Uuid) -> AppResult<String> {
        let expires_at =
            Utc::now() + Duration::days(self.jwt_config.refresh_expiration_days as i64);
        let claims = Claims::new_refresh(user_id, expires_at, JWT_ISSUER);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_config.secret.as_bytes()),
        )
        .map_err(|_| AppError::InternalError)?;

        // 存储 refresh token 的哈希值
        let token_hash = Self::hash_token(&token);
        self.refresh_token_repo
            .create(&NewRefreshToken {
                user_id: *user_id,
                token_hash,
                expires_at,
            })
            .await?;

        Ok(token)
    }

    /// 验证 Refresh Token
    fn verify_refresh_token(&self, token: &str) -> AppResult<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_config.secret.as_bytes()),
            &jwt_validation(),
        )
        .map_err(|_| AppError::Unauthorized("无效的刷新令牌".into()))?;

        let claims = token_data.claims;

        // 验证是否为 refresh token
        if !claims.is_refresh_token() {
            return Err(AppError::Unauthorized("无效的令牌类型".into()));
        }

        Ok(claims)
    }

    /// 对 token 进行哈希（用于存储）
    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
