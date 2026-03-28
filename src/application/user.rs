//! User 应用服务

use uuid::Uuid;

use crate::application::AppContext;
use crate::core::domain::shared::UserId;
use crate::core::domain::user::{User, UserRepository};
use crate::core::value_object::UserRole;
use crate::dto::request::{CreateUserRequest, UpdateUserRequest, UserQuery};
use crate::dto::response::{Pagination, UserListResponse, UserResponse};
use crate::errors::AppResult;

pub struct UserAppService<'a> {
    ctx: AppContext<'a>,
}

impl<'a> UserAppService<'a> {
    pub fn new(ctx: AppContext<'a>) -> Self {
        Self { ctx }
    }

    /// 创建用户
    pub async fn create(&self, req: CreateUserRequest) -> AppResult<UserResponse> {
        // 检查用户名唯一性
        if self.ctx.user_repo().exists_by_username(&req.username).await? {
            return Err(crate::errors::AppError::UsernameExists);
        }

        let role: UserRole = req.role.parse()
            .map_err(|_| crate::errors::AppError::ValidationError("无效的角色".into()))?;

        // 哈希密码（使用应用层的 hash_password）
        let password_hash = crate::application::auth::hash_password(&req.password)?;

        let user = User::create(req.username, password_hash, role)?;
        self.ctx.user_repo().save(&user).await?;

        Ok(to_response(&user))
    }

    /// 获取用户
    pub async fn get_by_id(&self, id: Uuid) -> AppResult<UserResponse> {
        let user = self.ctx.user_repo()
            .find_by_id(&UserId::from_uuid(id))
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(format!("用户: {}", id)))?;

        Ok(to_response(&user))
    }

    /// 更新用户
    pub async fn update(&self, id: Uuid, req: UpdateUserRequest) -> AppResult<UserResponse> {
        let user_id = UserId::from_uuid(id);
        let mut user = self.ctx.user_repo()
            .find_by_id(&user_id)
            .await?
            .ok_or_else(|| crate::errors::AppError::NotFound(format!("用户: {}", id)))?;

        user.update_profile(req.phone, req.email, req.avatar_url);

        if let Some(status) = req.status {
            use std::str::FromStr;
            let status = crate::core::domain::user::UserStatus::from_str(&status)
                .map_err(|_| crate::errors::AppError::ValidationError("无效的状态".into()))?;
            user.set_status(status);
        }

        self.ctx.user_repo().save(&user).await?;
        Ok(to_response(&user))
    }

    /// 删除用户
    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        self.ctx.user_repo()
            .delete(&UserId::from_uuid(id))
            .await?;
        Ok(())
    }

    /// 查询用户列表
    pub async fn query(&self, query: UserQuery) -> AppResult<UserListResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let users = self.ctx.user_repo()
            .find_all(query.role.as_deref(), query.status.as_deref(), limit, offset)
            .await?;

        let data: Vec<UserResponse> = users.iter().map(to_response).collect();

        Ok(UserListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total: 0,
                total_pages: 0,
            },
        })
    }
}

fn to_response(user: &User) -> UserResponse {
    UserResponse {
        id: user.id().as_uuid(),
        username: user.username().to_string(),
        role: user.role().to_string(),
        phone: user.phone().map(|s| s.to_string()),
        email: user.email().map(|s| s.to_string()),
        avatar_url: user.avatar_url().map(|s| s.to_string()),
        status: user.status().to_string(),
        last_login_at: user.last_login_at(),
        created_at: user.created_at(),
    }
}
