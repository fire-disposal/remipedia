use log::info;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::entity::NewUser;
use crate::core::value_object::UserRole;
use crate::dto::request::{CreateUserRequest, UpdateUserRequest, UserQuery};
use crate::dto::response::{Pagination, UserListResponse, UserResponse};
use crate::errors::{AppError, AppResult};
use crate::repository::UserRepository;
use crate::service::AuthService;

pub struct UserService<'a> {
    user_repo: UserRepository<'a>,
}

impl<'a> UserService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            user_repo: UserRepository::new(pool),
        }
    }

    /// 创建用户
    pub async fn create(&self, req: CreateUserRequest) -> AppResult<UserResponse> {
        // 验证角色
        UserRole::from_str(&req.role)
            .ok_or_else(|| AppError::ValidationError("无效的角色".into()))?;

        // 检查用户名是否存在
        if self.user_repo.exists_by_username(&req.username).await? {
            return Err(AppError::UsernameExists);
        }

        // 哈希密码
        let password_hash = AuthService::hash_password(&req.password)?;

        // 创建用户
        let user = self.user_repo.insert(&NewUser {
            username: req.username,
            password_hash,
            role: req.role,
            phone: req.phone,
            email: req.email,
        }).await?;

        info!("用户创建成功: user_id={}, username={}", user.id, user.username);

        Ok(user.into())
    }

    /// 获取用户
    pub async fn get_by_id(&self, id: &Uuid) -> AppResult<UserResponse> {
        let user = self.user_repo.find_by_id(id).await?;
        Ok(user.into())
    }

    /// 更新用户
    pub async fn update(&self, id: &Uuid, _req: UpdateUserRequest) -> AppResult<UserResponse> {
        let user = self.user_repo.find_by_id(id).await?;

        // 更新用户信息（需要扩展 repository）
        Ok(user.into())
    }

    /// 查询用户列表
    pub async fn query(&self, query: UserQuery) -> AppResult<UserListResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let users = self.user_repo.find_all(
            query.role.as_deref(),
            query.status.as_deref(),
            limit,
            offset,
        ).await?;

        let total = self.user_repo.count(
            query.role.as_deref(),
            query.status.as_deref(),
        ).await?;

        let data: Vec<UserResponse> = users.into_iter().map(|u| u.into()).collect();

        Ok(UserListResponse {
            data,
            pagination: Pagination {
                page,
                page_size,
                total,
                total_pages: (total + limit - 1) / limit,
            },
        })
    }

    /// 删除用户
    pub async fn delete(&self, id: &Uuid) -> AppResult<()> {
        self.user_repo.delete(id).await?;
        info!("用户删除成功: user_id={}", id);
        Ok(())
    }
}

// 实体到响应的转换
impl From<crate::core::entity::User> for UserResponse {
    fn from(user: crate::core::entity::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: user.role,
            phone: user.phone,
            email: user.email,
            avatar_url: user.avatar_url,
            status: user.status,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        }
    }
}