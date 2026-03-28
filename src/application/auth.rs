//! 认证应用服务

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sqlx::PgPool;

use crate::config::JwtConfig;
use crate::core::value_object::UserRole;
use crate::errors::{AppError, AppResult};

const JWT_ISSUER: &str = "remipedia";
const JWT_AUDIENCE: &str = "remipedia-api";

/// JWT Claims
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Claims {
    sub: String,      // user_id
    role: String,
    exp: i64,         // expiration
    iat: i64,         // issued at
    iss: String,
    aud: String,
    #[serde(rename = "type")]
    token_type: String, // "access" or "refresh"
}

impl Claims {
    fn new_access(user_id: &str, role: &str, expires_at: DateTime<Utc>, issuer: &str) -> Self {
        Self {
            sub: user_id.to_string(),
            role: role.to_string(),
            exp: expires_at.timestamp(),
            iat: Utc::now().timestamp(),
            iss: issuer.to_string(),
            aud: JWT_AUDIENCE.to_string(),
            token_type: "access".to_string(),
        }
    }

    fn new_refresh(user_id: &str, expires_at: DateTime<Utc>, issuer: &str) -> Self {
        Self {
            sub: user_id.to_string(),
            role: "user".to_string(),
            exp: expires_at.timestamp(),
            iat: Utc::now().timestamp(),
            iss: issuer.to_string(),
            aud: JWT_AUDIENCE.to_string(),
            token_type: "refresh".to_string(),
        }
    }
}

/// JWT验证器
pub struct JwtVerifier<'a> {
    jwt_config: &'a JwtConfig,
}

impl<'a> JwtVerifier<'a> {
    pub fn new(jwt_config: &'a JwtConfig) -> Self {
        Self { jwt_config }
    }

    pub fn verify_access_token(&self, token: &str) -> AppResult<(uuid::Uuid, UserRole)> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_audience(&[JWT_AUDIENCE]);

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_config.secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AppError::Unauthorized("无效的访问令牌".into()))?;

        let claims = token_data.claims;

        if claims.token_type != "access" {
            return Err(AppError::Unauthorized("无效的令牌类型".into()));
        }

        let user_id = uuid::Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("无效的用户ID".into()))?;

        let role: UserRole = claims.role.parse()
            .map_err(|_| AppError::Unauthorized("无效的角色".into()))?;

        Ok((user_id, role))
    }
}

/// 认证应用服务
pub struct AuthAppService<'a> {
    pool: &'a PgPool,
    jwt_config: &'a JwtConfig,
}

impl<'a> AuthAppService<'a> {
    pub fn new(pool: &'a PgPool, jwt_config: &'a JwtConfig) -> Self {
        Self { pool, jwt_config }
    }

    /// 用户登录
    pub async fn login(&self, username: &str, password: &str) -> AppResult<(String, String)> {
        // 查询用户
        let user: (uuid::Uuid, String, String, String) = sqlx::query_as(
            r#"SELECT id, username, password_hash, role FROM "user" WHERE username = $1"#
        )
        .bind(username)
        .fetch_one(self.pool)
        .await
        .map_err(|_| AppError::Unauthorized("用户名或密码错误".into()))?;

        let (user_id, _, password_hash, role) = user;

        // 验证密码
        let parsed_hash = PasswordHash::new(&password_hash)
            .map_err(|_| AppError::InternalError)?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized("用户名或密码错误".into()))?;

        // 检查状态
        let status: String = sqlx::query_scalar(
            r#"SELECT status FROM "user" WHERE id = $1"#
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;

        if status != "active" {
            return Err(AppError::Unauthorized("用户已被禁用".into()));
        }

        // 生成令牌
        let access_token = self.generate_access_token(&user_id.to_string(), &role)?;
        let refresh_token = self.generate_refresh_token(&user_id.to_string()).await?;

        // 更新最后登录时间
        sqlx::query(r#"UPDATE "user" SET last_login_at = NOW() WHERE id = $1"#)
            .bind(user_id)
            .execute(self.pool)
            .await?;

        Ok((access_token, refresh_token))
    }

    /// 刷新令牌
    pub async fn refresh_token(&self, refresh_token: &str) -> AppResult<(String, String)> {
        // 验证refresh token
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_audience(&[JWT_AUDIENCE]);

        let token_data = decode::<Claims>(
            refresh_token,
            &DecodingKey::from_secret(self.jwt_config.secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AppError::Unauthorized("无效的刷新令牌".into()))?;

        let claims = token_data.claims;

        if claims.token_type != "refresh" {
            return Err(AppError::Unauthorized("无效的令牌类型".into()));
        }

        let user_id = uuid::Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("无效的用户ID".into()))?;

        // 检查用户状态
        let status: String = sqlx::query_scalar(
            r#"SELECT status FROM "user" WHERE id = $1"#
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;

        if status != "active" {
            return Err(AppError::Unauthorized("用户已被禁用".into()));
        }

        // 获取角色
        let role: String = sqlx::query_scalar(
            r#"SELECT role FROM "user" WHERE id = $1"#
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;

        // 生成新令牌
        let new_access = self.generate_access_token(&user_id.to_string(), &role)?;
        let new_refresh = self.generate_refresh_token(&user_id.to_string()).await?;

        Ok((new_access, new_refresh))
    }

    fn generate_access_token(&self, user_id: &str, role: &str) -> AppResult<String> {
        let expires_at = Utc::now() + Duration::hours(self.jwt_config.expiration_hours as i64);
        let claims = Claims::new_access(user_id, role, expires_at, JWT_ISSUER);

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_config.secret.as_bytes()),
        )
        .map_err(|_| AppError::InternalError)
    }

    async fn generate_refresh_token(&self, user_id: &str) -> AppResult<String> {
        let expires_at = Utc::now() + Duration::days(self.jwt_config.refresh_expiration_days as i64);
        let claims = Claims::new_refresh(user_id, expires_at, JWT_ISSUER);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_config.secret.as_bytes()),
        )
        .map_err(|_| AppError::InternalError)?;

        // 存储refresh token hash（简化实现）
        let token_hash = sha256_hash(&token);
        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)"
        )
        .bind(uuid::Uuid::parse_str(user_id).map_err(|_| AppError::InternalError)?)
        .bind(token_hash)
        .bind(expires_at)
        .execute(self.pool)
        .await?;

        Ok(token)
    }
}

fn sha256_hash(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 哈希密码（公共函数）
pub fn hash_password(password: &str) -> crate::errors::AppResult<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| crate::errors::AppError::InternalError)
}
