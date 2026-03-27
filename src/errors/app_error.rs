use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use serde_json::json;
use thiserror::Error;

/// 应用统一错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("实体未找到: {0}")]
    NotFound(String),

    #[error("验证失败: {0}")]
    ValidationError(String),

    #[error("设备未绑定")]
    DeviceNotBound,

    #[error("绑定已存在")]
    BindingAlreadyExists,

    #[error("认证失败: {0}")]
    Unauthorized(String),

    #[error("权限不足")]
    Forbidden,

    #[error("密码错误")]
    InvalidPassword,

    #[error("用户名已存在")]
    UsernameExists,

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("UUID 解析错误")]
    UuidError,

    #[error("资源耗尽: {0}")]
    ResourceExhausted(String),

    #[error("内部错误")]
    InternalError,
}

impl From<uuid::Error> for AppError {
    fn from(_: uuid::Error) -> Self {
        AppError::UuidError
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        let status = match &self {
            AppError::NotFound(_) => Status::NotFound,
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::DeviceNotBound => Status::BadRequest,
            AppError::BindingAlreadyExists => Status::Conflict,
            AppError::Unauthorized(_) => Status::Unauthorized,
            AppError::Forbidden => Status::Forbidden,
            AppError::InvalidPassword => Status::Unauthorized,
            AppError::UsernameExists => Status::Conflict,
            AppError::ConfigError(_) => Status::InternalServerError,
            AppError::UuidError => Status::BadRequest,
            AppError::ResourceExhausted(_) => Status::ServiceUnavailable,
            AppError::DatabaseError(_) => Status::InternalServerError,
            AppError::InternalError => Status::InternalServerError,
        };

        let body = json!({
            "success": false,
            "error": self.to_string(),
            "code": status.code,
        });

        response::Response::build_from(body.respond_to(req)?)
            .status(status)
            .ok()
    }
}
