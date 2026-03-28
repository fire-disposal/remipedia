//! 基于角色的访问控制 (RBAC) 模块
//!
//! 权限模型：
//! - Permission: 原子权限（如 user:create, device:read）
//! - Role: 角色，包含一组权限
//! - 用户通过角色获得权限

pub mod entity;
pub mod permissions;
pub mod repository;
pub mod service;

pub use entity::{Permission, Role, RoleAssignment};
pub use permissions::{PermissionAction, PermissionDomain, SystemPermissions};
pub use repository::RoleRepository;
pub use service::RbacService;
