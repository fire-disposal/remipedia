use crate::core::entity::{AuditLog, AuditLogQuery, NewAuditLog};
use crate::errors::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

pub struct AuditLogRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> AuditLogRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 创建审计日志
    pub async fn create(&self, log: &NewAuditLog) -> AppResult<AuditLog> {
        let audit_log = sqlx::query_as::<_, AuditLog>(
            r#"INSERT INTO audit_logs 
               (user_id, action, resource, resource_id, details, ip_address, user_agent, status, error_message, duration_ms)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING *"#,
        )
        .bind(log.user_id)
        .bind(&log.action)
        .bind(&log.resource)
        .bind(&log.resource_id)
        .bind(&log.details)
        .bind(log.ip_address.clone())
        .bind(&log.user_agent)
        .bind(&log.status)
        .bind(&log.error_message)
        .bind(log.duration_ms)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(audit_log)
    }

    /// 查询审计日志
    pub async fn query(&self, query: &AuditLogQuery) -> AppResult<(Vec<AuditLog>, i64)> {
        let offset = ((query.page - 1) * query.page_size) as i64;
        let limit = query.page_size as i64;

        let mut sql = String::from(
            "SELECT * FROM audit_logs WHERE 1=1"
        );
        let mut count_sql = String::from(
            "SELECT COUNT(*) FROM audit_logs WHERE 1=1"
        );

        if query.user_id.is_some() {
            sql.push_str(" AND user_id = $1");
            count_sql.push_str(" AND user_id = $1");
        }
        if query.action.is_some() {
            sql.push_str(" AND action = $2");
            count_sql.push_str(" AND action = $2");
        }
        if query.resource.is_some() {
            sql.push_str(" AND resource = $3");
            count_sql.push_str(" AND resource = $3");
        }
        if query.status.is_some() {
            sql.push_str(" AND status = $4");
            count_sql.push_str(" AND status = $4");
        }
        if query.start_time.is_some() {
            sql.push_str(" AND created_at >= $5");
            count_sql.push_str(" AND created_at >= $5");
        }
        if query.end_time.is_some() {
            sql.push_str(" AND created_at <= $6");
            count_sql.push_str(" AND created_at <= $6");
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT $7 OFFSET $8");

        let mut query_builder = sqlx::query_as::<_, AuditLog>(&sql);
        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_sql);

        // Bind parameters
        if let Some(user_id) = query.user_id {
            query_builder = query_builder.bind(user_id);
            count_builder = count_builder.bind(user_id);
        }
        if let Some(ref action) = query.action {
            query_builder = query_builder.bind(action);
            count_builder = count_builder.bind(action);
        }
        if let Some(ref resource) = query.resource {
            query_builder = query_builder.bind(resource);
            count_builder = count_builder.bind(resource);
        }
        if let Some(ref status) = query.status {
            query_builder = query_builder.bind(status);
            count_builder = count_builder.bind(status);
        }
        if let Some(start_time) = query.start_time {
            query_builder = query_builder.bind(start_time);
            count_builder = count_builder.bind(start_time);
        }
        if let Some(end_time) = query.end_time {
            query_builder = query_builder.bind(end_time);
            count_builder = count_builder.bind(end_time);
        }

        query_builder = query_builder.bind(limit).bind(offset);

        let logs = query_builder.fetch_all(self.pool).await.map_err(AppError::DatabaseError)?;
        let total = count_builder.fetch_one(self.pool).await.map_err(AppError::DatabaseError)?;

        Ok((logs, total))
    }

    /// 获取单个审计日志
    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Option<AuditLog>> {
        let log = sqlx::query_as::<_, AuditLog>("SELECT * FROM audit_logs WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(log)
    }
}
