//! User 仓储 SQLx 实现

use async_trait::async_trait;
use sqlx::PgPool;

use std::str::FromStr;

use crate::core::domain::shared::{DomainError, DomainResult, UserId};
use crate::core::domain::user::{User, UserRepository, UserStatus};
use crate::core::entity::User as UserRow;
use crate::core::value_object::UserRole;

/// SQLx 实现的 User 仓储
pub struct SqlxUserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SqlxUserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    fn to_entity(row: UserRow) -> DomainResult<User> {
        let status = match row.status.as_str() {
            "active" => UserStatus::Active,
            "inactive" => UserStatus::Inactive,
            "locked" => UserStatus::Locked,
            _ => UserStatus::Active,
        };

        // row.role 已经是 UserRole 类型（sqlx::Type 自动映射）
        let role = row.role;

        Ok(User::reconstruct(
            UserId::from_uuid(row.id),
            row.username,
            row.password_hash,
            role,
            row.phone,
            row.email,
            row.avatar_url,
            status,
            row.last_login_at,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[async_trait]
impl<'a> UserRepository for SqlxUserRepository<'a> {
    async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"SELECT id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at
               FROM "user" WHERE id = $1"#
        )
        .bind(id.as_uuid())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn find_by_username(&self, username: &str) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"SELECT id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at
               FROM "user" WHERE username = $1"#
        )
        .bind(username)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(row.map(Self::to_entity).transpose()?)
    }

    async fn exists_by_username(&self, username: &str) -> DomainResult<bool> {
        let result: Option<(i64,)> = sqlx::query_as(
            r#"SELECT 1 FROM "user" WHERE username = $1 LIMIT 1"#
        )
        .bind(username)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(result.is_some())
    }

    async fn save(&self, user: &User) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO "user" (id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               ON CONFLICT (id) DO UPDATE SET
                   username = EXCLUDED.username,
                   password_hash = EXCLUDED.password_hash,
                   role = EXCLUDED.role,
                   phone = EXCLUDED.phone,
                   email = EXCLUDED.email,
                   avatar_url = EXCLUDED.avatar_url,
                   status = EXCLUDED.status,
                   last_login_at = EXCLUDED.last_login_at,
                   updated_at = EXCLUDED.updated_at"#
        )
        .bind(user.id().as_uuid())
        .bind(user.username())
        .bind(user.password_hash())
        .bind(user.role().to_string())
        .bind(user.phone())
        .bind(user.email())
        .bind(user.avatar_url())
        .bind(user.status().as_str())
        .bind(user.last_login_at())
        .bind(user.created_at())
        .bind(user.updated_at())
        .execute(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &UserId) -> DomainResult<()> {
        sqlx::query(r#"DELETE FROM "user" WHERE id = $1"#)
            .bind(id.as_uuid())
            .execute(self.pool)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn find_all(&self, role: Option<&str>, status: Option<&str>, limit: i64, offset: i64) -> DomainResult<Vec<User>> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"SELECT id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at
               FROM "user"
               WHERE ($1::text IS NULL OR role = $1)
                 AND ($2::text IS NULL OR status = $2)
               ORDER BY created_at DESC
               LIMIT $3 OFFSET $4"#
        )
        .bind(role)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        rows.into_iter().map(Self::to_entity).collect()
    }

    async fn exists_admin(&self) -> DomainResult<bool> {
        let result: Option<(i64,)> = sqlx::query_as(
            r#"SELECT 1 FROM "user" WHERE role = 'admin' LIMIT 1"#
        )
        .fetch_optional(self.pool)
        .await
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(result.is_some())
    }
}
