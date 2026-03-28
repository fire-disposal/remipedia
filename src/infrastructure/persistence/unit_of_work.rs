//! 工作单元模式（Unit of Work）

use sqlx::{PgPool, Postgres, Transaction};

use crate::errors::{AppError, AppResult};
use crate::infrastructure::persistence::device_repository::SqlxDeviceRepositoryTx;

/// 工作单元
///
/// 管理事务生命周期和仓储访问
pub struct UnitOfWork {
    tx: Option<Transaction<'static, Postgres>>,
}

impl UnitOfWork {
    /// 开始新事务
    pub async fn begin(pool: &PgPool) -> AppResult<Self> {
        let tx = pool.begin().await.map_err(AppError::DatabaseError)?;
        Ok(Self { tx: Some(tx) })
    }

    /// 获取设备仓储（基于当前事务）
    pub fn device_repository(&mut self) -> SqlxDeviceRepositoryTx<'_> {
        SqlxDeviceRepositoryTx::new(self.tx.as_mut().unwrap())
    }

    /// 提交事务
    pub async fn commit(mut self) -> AppResult<()> {
        if let Some(tx) = self.tx.take() {
            tx.commit().await.map_err(AppError::DatabaseError)?;
        }
        Ok(())
    }

    /// 回滚事务
    pub async fn rollback(mut self) -> AppResult<()> {
        if let Some(tx) = self.tx.take() {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
        }
        Ok(())
    }
}

impl Drop for UnitOfWork {
    fn drop(&mut self) {
        // 如果事务未提交/回滚，自动回滚
        if self.tx.is_some() {
            log::warn!("UnitOfWork dropped without commit/rollback, auto-rollback");
        }
    }
}
