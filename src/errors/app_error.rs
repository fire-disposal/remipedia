use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use serde_json::json;
use thiserror::Error;

/// 应用统一错误类型
///
/// 精简为 7 个核心变体，消除语义重叠：
/// - `DeviceNotBound` → `ValidationError`
/// - `BindingAlreadyExists`/`UsernameExists` → `Conflict`
/// - `InvalidPassword` → `Unauthorized`
/// - `UuidError` → `ValidationError`
/// - `ConfigError`/`ResourceExhausted`/`IoError` → `InternalError`
#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("未找到: {0}")]
    NotFound(String),

    #[error("请求无效: {0}")]
    ValidationError(String),

    #[error("认证失败: {0}")]
    Unauthorized(String),

    #[error("权限不足")]
    Forbidden,

    #[error("资源冲突: {0}")]
    Conflict(String),

    #[error("内部错误: {0}")]
    InternalError(String),
}

impl From<uuid::Error> for AppError {
    fn from(e: uuid::Error) -> Self {
        Self::ValidationError(format!("UUID 格式错误: {}", e))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(format!("IO 错误: {}", e))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::internal(e.to_string())
    }
}

impl From<config::ConfigError> for AppError {
    fn from(e: config::ConfigError) -> Self {
        Self::internal(format!("配置错误: {}", e))
    }
}

impl From<rocket::Error> for AppError {
    fn from(e: rocket::Error) -> Self {
        Self::internal(format!("服务器错误: {}", e))
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        Self::internal(format!("数据库迁移错误: {}", e))
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    // -----------------------------------------------------------------------
    // 辅助构造方法 —— 简化调用方代码，消除重复的 format!/into 样板
    // -----------------------------------------------------------------------

    /// 构造「未找到」错误
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// 构造「请求无效」错误（参数校验失败、格式错误等）
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }

    /// 构造「认证失败」错误
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }

    /// 构造「资源冲突」错误
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    /// 构造「内部错误」
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalError(msg.into())
    }

    // -----------------------------------------------------------------------
    // 响应构建
    // -----------------------------------------------------------------------

    /// 将错误变体映射为 (HTTP 状态码, 客户端可见错误消息)
    fn to_response_parts(&self) -> (Status, String) {
        match self {
            Self::NotFound(msg) => (Status::NotFound, msg.clone()),
            Self::ValidationError(msg) => (Status::BadRequest, msg.clone()),
            Self::Unauthorized(msg) => (Status::Unauthorized, msg.clone()),
            Self::Forbidden => (Status::Forbidden, self.to_string()),
            Self::Conflict(msg) => (Status::Conflict, msg.clone()),
            Self::DatabaseError(e) => {
                tracing::error!("数据库错误: {:?}", e);
                (Status::InternalServerError, "内部服务错误".into())
            }
            Self::InternalError(msg) => {
                tracing::error!("内部错误: {}", msg);
                (Status::InternalServerError, "内部服务错误".into())
            }
        }
    }
}

impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        let (status, error_msg) = self.to_response_parts();
        let body = json!({
            "success": false,
            "error": error_msg,
            "code": status.code,
        });
        response::Response::build_from(body.respond_to(req)?)
            .status(status)
            .ok()
    }
}
