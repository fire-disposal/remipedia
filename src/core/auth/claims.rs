use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims 结构（Module-Based）
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
    /// 角色 ID
    pub role_id: String,
    /// 是否为系统角色（拥有所有模块权限）
    pub is_system_role: bool,
    /// 可访问模块列表（空列表表示通配权限，由 is_system_role 控制）
    pub modules: Vec<String>,
    /// 令牌类型：access 或 refresh
    pub token_type: String,
}

impl Claims {
    /// 创建新的 Access Token Claims
    pub fn new_access(
        user_id: &Uuid,
        role_id: &Uuid,
        is_system_role: bool,
        modules: Vec<String>,
        expires_at: DateTime<Utc>,
        issuer: &str,
    ) -> Self {
        let now = Utc::now();

        Self {
            iss: issuer.to_string(),
            aud: "remipedia-api".to_string(),
            exp: expires_at.timestamp(),
            nbf: now.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::now_v7().to_string(),
            sub: user_id.to_string(),
            role_id: role_id.to_string(),
            is_system_role,
            modules,
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
            role_id: String::new(), // refresh token 不需要角色
            is_system_role: false,
            modules: Vec::new(),
            token_type: "refresh".to_string(),
        }
    }

    /// 获取用户 ID
    pub fn user_id(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.sub)
    }

    /// 获取角色 ID
    pub fn role_id(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.role_id)
    }

    /// 检查是否拥有指定模块权限
    pub fn can_access_module(&self, module: &str) -> bool {
        // 系统角色拥有所有权限
        if self.is_system_role {
            return true;
        }
        // 检查模块列表
        self.modules.contains(&module.to_string())
    }

    /// 获取可访问模块列表
    pub fn accessible_modules(&self) -> &[String] {
        &self.modules
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
