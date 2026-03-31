use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::{NewUser, User};
use crate::core::value_object::SystemRole;
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
            r#"SELECT id, username, password_hash, role_id, phone, email, avatar_url, status, last_login_at, created_at, updated_at
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
            r#"SELECT id, username, password_hash, role_id, phone, email, avatar_url, status, last_login_at, created_at, updated_at
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

    pub async fn exists_by_email(&self, email: &str) -> AppResult<bool> {
        let result: Option<(i64,)> =
            sqlx::query_as(r#"SELECT 1 FROM "user" WHERE email = $1 LIMIT 1"#)
                .bind(email)
                .fetch_optional(self.pool)
                .await
                .map_err(AppError::DatabaseError)?;

        Ok(result.is_some())
    }

    pub async fn exists_by_phone(&self, phone: &str) -> AppResult<bool> {
        let result: Option<(i64,)> =
            sqlx::query_as(r#"SELECT 1 FROM "user" WHERE phone = $1 LIMIT 1"#)
                .bind(phone)
                .fetch_optional(self.pool)
                .await
                .map_err(AppError::DatabaseError)?;

        Ok(result.is_some())
    }

    pub async fn insert(&self, user: &NewUser) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"INSERT INTO "user" (username, password_hash, role_id, phone, email)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, username, password_hash, role_id, phone, email, avatar_url, status, last_login_at, created_at, updated_at"#,
        )
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(user.role_id)
        .bind(&user.phone)
        .bind(&user.email)
        .fetch_one(self.pool)
        .await
        .map_err(Self::map_write_error)
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

    pub async fn update_profile(
        &self,
        id: &Uuid,
        phone: Option<&str>,
        email: Option<&str>,
        avatar_url: Option<&str>,
        status: Option<&str>,
    ) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"UPDATE "user"
               SET phone = COALESCE($2, phone),
                   email = COALESCE($3, email),
                   avatar_url = COALESCE($4, avatar_url),
                   status = COALESCE($5, status)
               WHERE id = $1
               RETURNING id, username, password_hash, role_id, phone, email, avatar_url, status, last_login_at, created_at, updated_at"#,
        )
        .bind(id)
        .bind(phone)
        .bind(email)
        .bind(avatar_url)
        .bind(status)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("用户: {}", id)),
            other => Self::map_write_error(other),
        })
    }

    pub async fn update_role(&self, id: &Uuid, role_id: &Uuid) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"UPDATE "user" SET role_id = $2 WHERE id = $1
               RETURNING id, username, password_hash, role_id, phone, email, avatar_url, status, last_login_at, created_at, updated_at"#,
        )
        .bind(id)
        .bind(role_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("用户: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn find_all(
        &self,
        role_id: Option<&Uuid>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"SELECT id, username, password_hash, role_id, phone, email, avatar_url, status, last_login_at, created_at, updated_at
               FROM "user"
               WHERE ($1::uuid IS NULL OR role_id = $1)
                 AND ($2::text IS NULL OR status = $2)
               ORDER BY created_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(role_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(users)
    }

    pub async fn count(
        &self, role_id: Option<&Uuid>, status: Option<&str>
    ) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "user"
               WHERE ($1::uuid IS NULL OR role_id = $1)
                 AND ($2::text IS NULL OR status = $2)"#,
        )
        .bind(role_id)
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

    /// 检查是否存在超级管理员
    pub async fn exists_super_admin(&self) -> AppResult<bool> {
        let result: Option<(i32,)> = sqlx::query_as(
            r#"SELECT 1 FROM "user" WHERE role_id = $1 LIMIT 1"#,
        )
        .bind(SystemRole::SUPER_ADMIN_ID)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.is_some())
    }

    /// 创建初始超级管理员
    pub async fn create_super_admin(
        &self,
        username: &str,
        password_hash: &str,
    ) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"INSERT INTO "user" (username, password_hash, role_id, status)
               VALUES ($1, $2, $3, 'active')
               RETURNING id, username, password_hash, role_id, phone, email, avatar_url, status, last_login_at, created_at, updated_at"#,
        )
        .bind(username)
        .bind(password_hash)
        .bind(SystemRole::SUPER_ADMIN_ID)
        .fetch_one(self.pool)
        .await
        .map_err(Self::map_write_error)
    }

    /// 获取用户可访问的患者列表
    pub async fn get_accessible_subjects(&self,
        user_id: &Uuid,
    ) -> AppResult<Vec<Uuid>> {
        let subjects: Vec<(Uuid,)> = sqlx::query_as(
            r#"SELECT patient_id FROM user_patient_binding WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(subjects.into_iter().map(|s| s.0).collect())
    }

    fn map_write_error(e: sqlx::Error) -> AppError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::ValidationError("手机号或邮箱已被使用".into());
            }
        }
        AppError::DatabaseError(e)
    }
}
