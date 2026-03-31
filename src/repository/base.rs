//! Repository基础trait和工具
//!
//! 提供通用的CRUD操作和错误处理，减少Repository层的重复代码

use crate::errors::{AppError, AppResult};
use uuid::Uuid;

/// Repository基础trait
///
/// 为所有Repository提供通用的CRUD操作模式
/// 注意：实际SQL查询仍需各Repository自行实现
pub trait BaseRepository<E, NewE> {
    /// 查找实体（根据ID）
    fn find_by_id(&self, id: &Uuid) -> impl std::future::Future<Output = AppResult<E>> + Send;

    /// 查找所有实体
    fn find_all(
        &self,
        limit: i64,
        offset: i64,
    ) -> impl std::future::Future<Output = AppResult<Vec<E>>> + Send;

    /// 创建实体
    fn insert(&self, entity: &NewE) -> impl std::future::Future<Output = AppResult<E>> + Send;

    /// 删除实体
    fn delete(&self, id: &Uuid) -> impl std::future::Future<Output = AppResult<()>> + Send;
}

/// Repository错误处理工具
pub struct RepositoryHelper;

impl RepositoryHelper {
    /// 将sqlx错误映射为AppError
    ///
    /// 特别处理RowNotFound错误
    pub fn map_not_found_error(e: sqlx::Error, entity_name: &str, id: &Uuid) -> AppError {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("{}: {}", entity_name, id)),
            other => AppError::DatabaseError(other),
        }
    }

    /// 映射写入错误（处理唯一约束冲突）
    pub fn map_write_error(e: sqlx::Error, duplicate_msg: &str) -> AppError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::ValidationError(duplicate_msg.into());
            }
        }
        AppError::DatabaseError(e)
    }

    /// 检查删除结果
    pub fn check_delete_result(
        result: sqlx::postgres::PgQueryResult,
        entity_name: &str,
        id: &Uuid,
    ) -> AppResult<()> {
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("{}: {}", entity_name, id)));
        }
        Ok(())
    }
}

/// Repository构造宏
///
/// 自动生成标准的Repository结构和方法
///
/// # 使用示例
/// ```rust
/// use remipedia::repository::define_repository;
///
/// define_repository! {
///     pub struct UserRepository;
///     entity = crate::core::entity::User;
///     new_entity = crate::core::entity::NewUser;
///     table_name = "user";
/// }
/// ```
#[macro_export]
macro_rules! define_repository {
    (
        $(#[$meta:meta])*
        pub struct $name:ident;
        entity = $entity:ty;
        new_entity = $new_entity:ty;
        table_name = $table:literal;
    ) => {
        $(#[$meta])*
        pub struct $name<'a> {
            pool: &'a ::sqlx::PgPool,
        }

        impl<'a> $name<'a> {
            /// 创建新的Repository实例
            pub fn new(pool: &'a ::sqlx::PgPool) -> Self {
                Self { pool }
            }

            /// 获取数据库连接池
            pub fn pool(&self) -> &::sqlx::PgPool {
                self.pool
            }
        }
    };
}

/// 条件查询构建器
///
/// 用于构建带有可选条件的SQL查询
pub struct QueryBuilder {
    conditions: Vec<String>,
    params: Vec<Box<dyn std::any::Any + Send>>,
}

impl QueryBuilder {
    /// 创建新的查询构建器
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            params: Vec::new(),
        }
    }

    /// 添加可选条件
    pub fn add_optional_condition<T>(&mut self, field: &str, value: Option<T>, param_index: usize)
    where
        T: std::any::Any + Send + 'static,
    {
        if value.is_some() {
            self.conditions
                .push(format!("AND {} = ${}", field, param_index));
            self.params.push(Box::new(value.unwrap()));
        }
    }

    /// 添加文本模糊匹配条件
    pub fn add_text_search(&mut self, field: &str, value: Option<&str>, param_index: usize) {
        if let Some(_v) = value {
            self.conditions.push(format!(
                "AND {} ILIKE '%' || ${} || '%'",
                field, param_index
            ));
        }
    }

    /// 构建WHERE子句
    pub fn build_where_clause(&self) -> String {
        if self.conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE 1=1 {}", self.conditions.join(" "))
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
