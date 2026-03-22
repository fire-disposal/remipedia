use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 令牌签发者
    pub iss: String,
    /// 令牌接收者
    pub aud: String,
    /// 令牌过期时间（Unix 时间戳）
    pub exp: i64,
    /// 令牌生效时间（Unix 时间戳）
    pub nbf: i64,
    /// 令牌签发时间（Unix 时间戳）
    pub iat: i64,
    /// JWT ID（唯一标识）
    pub jti: String,
    /// 用户 ID
    pub sub: String,
    /// 用户角色
    pub role: String,
    /// 令牌类型：access 或 refresh
    pub token_type: String,
}

impl Claims {
    /// 创建新的 Access Token Claims
    pub fn new_access(user_id: &Uuid, role: &str, expires_at: DateTime<Utc>, issuer: &str) -> Self {
        let now = Utc::now();
        Self {
            iss: issuer.to_string(),
            aud: "remipedia-api".to_string(),
            exp: expires_at.timestamp(),
            nbf: now.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::now_v7().to_string(),
            sub: user_id.to_string(),
            role: role.to_string(),
            token_type: "access".to_string(),
        }
    }

    /// 创建新的 Refresh Token Claims
    pub fn new_refresh(user_id: &Uuid, expires_at: DateTime<Utc>, issuer: &str) -> Self {
        let now = Utc::now();
        Self {
            iss: issuer.to_string(),
            aud: "remipedia-api".to_string(),
            exp: expires_at.timestamp(),
            nbf: now.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::now_v7().to_string(),
            sub: user_id.to_string(),
            role: String::new(), // refresh token 不需要角色
            token_type: "refresh".to_string(),
        }
    }

    /// 获取用户 ID
    pub fn user_id(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.sub)
    }

    /// 检查是否为 access token
    pub fn is_access_token(&self) -> bool {
        self.token_type == "access"
    }

    /// 检查是否为 refresh token
    pub fn is_refresh_token(&self) -> bool {
        self.token_type == "refresh"
    }
}
