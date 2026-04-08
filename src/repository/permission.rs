use crate::errors::{AppError, AppResult};
use sqlx::PgPool;

pub struct PermissionRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PermissionRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}
