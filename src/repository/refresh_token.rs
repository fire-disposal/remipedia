use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{NewRefreshToken, RefreshToken};
use crate::errors::{AppError, AppResult};

pub struct RefreshTokenRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> RefreshTokenRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 创建刷新令牌
    pub async fn create(&self, token: &NewRefreshToken) -> AppResult<RefreshToken> {
        sqlx::query_as::<_, RefreshToken>(
            r#"INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
               VALUES ($1, $2, $3)
               RETURNING id, user_id, token_hash, expires_at, created_at, revoked_at"#,
        )
        .bind(token.user_id)
        .bind(&token.token_hash)
        .bind(token.expires_at)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    /// 根据 token hash 查找刷新令牌
    pub async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<RefreshToken>> {
        sqlx::query_as::<_, RefreshToken>(
            r#"SELECT id, user_id, token_hash, expires_at, created_at, revoked_at
               FROM refresh_tokens WHERE token_hash = $1"#,
        )
        .bind(token_hash)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    /// 撤销刷新令牌
    pub async fn revoke(&self, token_hash: &str) -> AppResult<()> {
        let result = sqlx::query(
            r#"UPDATE refresh_tokens SET revoked_at = $1 WHERE token_hash = $2 AND revoked_at IS NULL"#,
        )
        .bind(Utc::now())
        .bind(token_hash)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("刷新令牌不存在或已撤销".into()));
        }

        Ok(())
    }

    /// 撤销用户所有刷新令牌
    pub async fn revoke_all_for_user(&self, user_id: &Uuid) -> AppResult<()> {
        sqlx::query(
            r#"UPDATE refresh_tokens SET revoked_at = $1 WHERE user_id = $2 AND revoked_at IS NULL"#,
        )
        .bind(Utc::now())
        .bind(user_id)
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    /// 删除过期的刷新令牌
    pub async fn delete_expired(&self) -> AppResult<u64> {
        let result = sqlx::query(
            r#"DELETE FROM refresh_tokens WHERE expires_at < $1 OR revoked_at IS NOT NULL"#,
        )
        .bind(Utc::now())
        .execute(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// 检查刷新令牌是否有效
    pub async fn is_valid(&self, token_hash: &str) -> AppResult<bool> {
        let token = self.find_by_hash(token_hash).await?;

        match token {
            Some(t) => {
                // 检查是否已撤销
                if t.revoked_at.is_some() {
                    return Ok(false);
                }
                // 检查是否过期
                if t.expires_at < Utc::now() {
                    return Ok(false);
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
