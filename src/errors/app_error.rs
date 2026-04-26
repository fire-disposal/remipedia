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
        Self::InternalError(format!("IO 错误: {}", e))
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        let (status, error_msg) = match &self {
            Self::NotFound(_) => (Status::NotFound, self.to_string()),
            Self::ValidationError(_) => (Status::BadRequest, self.to_string()),
            Self::Unauthorized(_) => (Status::Unauthorized, self.to_string()),
            Self::Forbidden => (Status::Forbidden, self.to_string()),
            Self::Conflict(_) => (Status::Conflict, self.to_string()),
            Self::DatabaseError(e) => {
                log::error!("数据库错误: {:?}", e);
                (Status::InternalServerError, "内部服务错误".into())
            }
            Self::InternalError(e) => {
                log::error!("内部错误: {}", e);
                (Status::InternalServerError, "内部服务错误".into())
            }
        };

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
