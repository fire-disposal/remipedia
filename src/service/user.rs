use log::info;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::core::entity::NewUser;
use crate::dto::request::{CreateUserRequest, UpdateUserRequest, UserQuery};
use crate::dto::response::{Pagination, UserListResponse, UserResponse};
use crate::errors::{AppError, AppResult};
use crate::repository::{RoleRepository, UserRepository};
use crate::service::AuthService;

pub struct UserService<'a> {
    user_repo: UserRepository<'a>,
    role_repo: RoleRepository<'a>,
}

impl<'a> UserService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            user_repo: UserRepository::new(pool),
            role_repo: RoleRepository::new(pool),
        }
    }

    /// 将 User 实体转换为 UserResponse（替代 User::to_response 的跨层调用）
    async fn to_user_response(&self, user: crate::core::entity::User) -> AppResult<UserResponse> {
        let role_name = match self.role_repo.find_by_id(&user.role_id).await? {
            Some(role) => role.name,
            None => "unknown".to_string(),
        };

        Ok(UserResponse {
            id: user.id,
            username: user.username,
            role_id: user.role_id,
            role_name,
            phone: user.phone,
            email: user.email,
            avatar_url: user.avatar_url,
            status: user.status,
            created_at: user.created_at,
            last_login_at: user.last_login_at,
        })
    }

    /// 创建用户
    pub async fn create(&self, req: CreateUserRequest) -> AppResult<UserResponse> {
        // 验证角色存在
        let role_id = Uuid::parse_str(&req.role_id)
            .map_err(|_| AppError::ValidationError("无效的角色ID".into()))?;

        let role = self.role_repo.find_by_id(&role_id).await?;
        if role.is_none() {
            return Err(AppError::ValidationError("角色不存在".into()));
        }

        // 检查用户名是否存在
        if self.user_repo.exists_by_username(&req.username).await? {
            return Err(AppError::Conflict("用户名已存在".into()));
        }

        // 哈希密码
        let password_hash = AuthService::hash_password(&req.password)?;

        // 创建用户
        let user = self
            .user_repo
            .insert(&NewUser {
                username: req.username,
                password_hash,
                role_id,
                phone: req.phone,
                email: req.email,
            })
            .await?;

        info!(
            "用户创建成功: user_id={}, username={}",
            user.id, user.username
        );

        // 转换为UserResponse
        self.to_user_response(user).await
    }

    /// 获取用户
    pub async fn get_by_id(&self, id: &Uuid) -> AppResult<UserResponse> {
        let user = self.user_repo.find_by_id(id).await?;
        self.to_user_response(user).await
    }

    /// 更新用户
    pub async fn update(
        &self, id: &Uuid, req: UpdateUserRequest
    ) -> AppResult<UserResponse> {
        if let Some(status) = req.status.as_deref() {
            let is_valid = matches!(status, "active" | "inactive" | "locked");
            if !is_valid {
                return Err(AppError::ValidationError("无效的状态值".into()));
            }
        }

        let user = self
            .user_repo
            .update_profile(
                id,
                req.phone.as_deref(),
                req.email.as_deref(),
                req.avatar_url.as_deref(),
                req.status.as_deref(),
            )
            .await?;

        info!("用户更新成功: user_id={}", user.id);

        self.to_user_response(user).await
    }

    /// 查询用户列表
    pub async fn query(
        &self, query: UserQuery) -> AppResult<UserListResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        let role_id = query.role_id.as_ref()
            .and_then(|r| Uuid::parse_str(r).ok());

        let users = self
            .user_repo
            .find_all(
                role_id.as_ref(),
                query.status.as_deref(),
                limit,
                offset,
            )
            .await?;

        let total = self
            .user_repo
            .count(role_id.as_ref(), query.status.as_deref())
            .await?;

        // 收集所有唯一的role_id进行批量查询（优化N+1问题）
        let role_ids: Vec<_> = users.iter().map(|u| u.role_id).collect();
        let mut role_names: HashMap<Uuid, String> = HashMap::new();
        for role_id in &role_ids {
            if let Ok(Some(role)) = self.role_repo.find_by_id(role_id).await {
                role_names.insert(*role_id, role.name);
            }
        }

        // 转换为响应
        let responses: Vec<UserResponse> = users
            .into_iter()
            .map(|user| {
                let role_name = role_names
                    .get(&user.role_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                UserResponse {
                    id: user.id,
                    username: user.username,
                    role_id: user.role_id,
                    role_name,
                    phone: user.phone,
                    email: user.email,
                    avatar_url: user.avatar_url,
                    status: user.status,
                    created_at: user.created_at,
                    last_login_at: user.last_login_at,
                }
            })
            .collect();

        Ok(UserListResponse {
            users: responses,
            pagination: Pagination::new(page, page_size, total),
        })
    }

    /// 删除用户
    pub async fn delete(
        &self, id: &Uuid) -> AppResult<()> {
        self.user_repo.delete(id).await?;
        info!("用户删除成功: user_id={}", id);
        Ok(())
    }
}
