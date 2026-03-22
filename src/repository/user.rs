use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{NewUser, User};
use crate::errors::{AppError, AppResult};

pub struct UserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"SELECT id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at
               FROM "user" WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("用户: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        sqlx::query_as::<_, User>(
            r#"SELECT id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at
               FROM "user" WHERE username = $1"#,
        )
        .bind(username)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn exists_by_username(&self, username: &str) -> AppResult<bool> {
        let result: Option<(i64,)> =
            sqlx::query_as(r#"SELECT 1 FROM "user" WHERE username = $1 LIMIT 1"#)
                .bind(username)
                .fetch_optional(self.pool)
                .await
                .map_err(AppError::DatabaseError)?;

        Ok(result.is_some())
    }

    pub async fn insert(&self, user: &NewUser) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"INSERT INTO "user" (username, password_hash, role, phone, email)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at"#,
        )
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(&user.phone)
        .bind(&user.email)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn update_last_login(&self, id: &Uuid) -> AppResult<()> {
        sqlx::query(r#"UPDATE "user" SET last_login_at = NOW() WHERE id = $1"#)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    pub async fn update_password(&self, id: &Uuid, password_hash: &str) -> AppResult<()> {
        sqlx::query(r#"UPDATE "user" SET password_hash = $2 WHERE id = $1"#)
            .bind(id)
            .bind(password_hash)
            .execute(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    pub async fn find_all(
        &self,
        role: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"SELECT id, username, password_hash, role, phone, email, avatar_url, status, last_login_at, created_at, updated_at
               FROM "user"
               WHERE ($1::text IS NULL OR role = $1)
                 AND ($2::text IS NULL OR status = $2)
               ORDER BY created_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(role)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(users)
    }

    pub async fn count(&self, role: Option<&str>, status: Option<&str>) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "user"
               WHERE ($1::text IS NULL OR role = $1)
                 AND ($2::text IS NULL OR status = $2)"#,
        )
        .bind(role)
        .bind(status)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.0)
    }

    pub async fn delete(&self, id: &Uuid) -> AppResult<()> {
        let result = sqlx::query(r#"DELETE FROM "user" WHERE id = $1"#)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("用户: {}", id)));
        }

        Ok(())
    }
}
