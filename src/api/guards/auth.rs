use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use uuid::Uuid;

use crate::core::value_object::UserRole;
use crate::errors::AppError;

/// 认证用户信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub role: UserRole,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AppError;

    async fn from_request(_request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 简化：返回一个默认用户，用于测试
        // 实际应该验证JWT
        Outcome::Success(AuthenticatedUser {
            id: Uuid::now_v7(),
            role: UserRole::Admin,
        })
    }
}

/// 管理员用户守卫
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthenticatedUser);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminUser {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user = AuthenticatedUser::from_request(request).await;

        match user {
            Outcome::Success(user) => {
                if user.role.is_admin() {
                    Outcome::Success(AdminUser(user))
                } else {
                    Outcome::Error((Status::Forbidden, AppError::Forbidden))
                }
            }
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}
