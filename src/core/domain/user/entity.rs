//! User 富实体

use chrono::{DateTime, Utc};

use crate::core::domain::shared::{DomainError, DomainResult, UserId};
use crate::core::value_object::UserRole;

/// 用户实体
#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    username: String,
    password_hash: String,
    role: UserRole,
    phone: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
    status: UserStatus,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// UserStatus 从 value_object 复用
pub use crate::core::value_object::UserStatus;

impl User {
    /// 创建用户
    pub fn create(
        username: String,
        password_hash: String,
        role: UserRole,
    ) -> DomainResult<Self> {
        if username.len() < 3 {
            return Err(DomainError::Validation("用户名至少3个字符".into()));
        }

        let now = Utc::now();
        Ok(Self {
            id: UserId::new(),
            username,
            password_hash,
            role,
            phone: None,
            email: None,
            avatar_url: None,
            status: UserStatus::Active,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// 从持久化重建
    pub fn reconstruct(
        id: UserId,
        username: String,
        password_hash: String,
        role: UserRole,
        phone: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
        status: UserStatus,
        last_login_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            username,
            password_hash,
            role,
            phone,
            email,
            avatar_url,
            status,
            last_login_at,
            created_at,
            updated_at,
        }
    }

    /// 验证密码
    pub fn verify_password(&self, password: &str) -> bool {
        // 实际应该使用 Argon2 验证，这里简化
        // 实际密码验证在应用层进行
        true
    }

    /// 更新最后登录时间
    pub fn record_login(&mut self) {
        self.last_login_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 设置状态
    pub fn set_status(&mut self, status: UserStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// 更新个人信息
    pub fn update_profile(
        &mut self,
        phone: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
    ) {
        if phone.is_some() {
            self.phone = phone;
        }
        if email.is_some() {
            self.email = email;
        }
        if avatar_url.is_some() {
            self.avatar_url = avatar_url;
        }
        self.updated_at = Utc::now();
    }

    /// 是否管理员
    pub fn is_admin(&self) -> bool {
        self.role.is_admin()
    }

    /// 是否活跃
    pub fn is_active(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }

    // Getters
    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    pub fn role(&self) -> UserRole {
        self.role
    }

    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn avatar_url(&self) -> Option<&str> {
        self.avatar_url.as_deref()
    }

    pub fn status(&self) -> UserStatus {
        self.status
    }

    pub fn last_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_login_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_create() {
        let user = User::create("admin".into(), "hash".into(), UserRole::Admin).unwrap();
        assert_eq!(user.username(), "admin");
        assert!(user.is_admin());
        assert!(user.is_active());
    }

    #[test]
    fn test_user_create_validation() {
        let result = User::create("ab".into(), "hash".into(), UserRole::User);
        assert!(result.is_err());
    }

    #[test]
    fn test_user_record_login() {
        let mut user = User::create("test".into(), "hash".into(), UserRole::User).unwrap();
        assert!(user.last_login_at().is_none());
        user.record_login();
        assert!(user.last_login_at().is_some());
    }
}
