use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// 角色实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    /// 数据范围：all(全部), self(仅自己), department(科室)
    pub data_scope: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建角色的数据
#[derive(Debug, Clone)]
pub struct NewRole {
    pub name: String,
    pub description: Option<String>,
    /// 数据范围，不提供则默认为 'all'
    pub data_scope: Option<String>,
}

impl NewRole {
    /// 创建一个基本的 NewRole（不含 data_scope）
    pub fn new(name: String, description: Option<String>) -> Self {
        Self {
            name,
            description,
            data_scope: None,
        }
    }
}

/// 更新角色的数据
#[derive(Debug, Clone, Default)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub description: Option<String>,
    /// 数据范围
    pub data_scope: Option<String>,
}
