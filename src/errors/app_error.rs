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

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("内部错误")]
    InternalError,
}

impl From<uuid::Error> for AppError {
    fn from(_: uuid::Error) -> Self {
        Self::UuidError
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        let status = match &self {
            Self::NotFound(_) => Status::NotFound,
            Self::ValidationError(_) => Status::BadRequest,
            Self::DeviceNotBound => Status::BadRequest,
            Self::BindingAlreadyExists => Status::Conflict,
            Self::Unauthorized(_) => Status::Unauthorized,
            Self::Forbidden => Status::Forbidden,
            Self::InvalidPassword => Status::Unauthorized,
            Self::UsernameExists => Status::Conflict,
            Self::ConfigError(_) => Status::InternalServerError,
            Self::UuidError => Status::BadRequest,
            Self::ResourceExhausted(_) => Status::ServiceUnavailable,
            Self::IoError(_) => Status::InternalServerError,
            Self::DatabaseError(_) => Status::InternalServerError,
            Self::InternalError => Status::InternalServerError,
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
