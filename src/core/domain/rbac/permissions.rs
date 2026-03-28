//! 系统权限常量定义

use std::collections::HashSet;

/// 权限域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionDomain {
    User,
    Patient,
    Device,
    Binding,
    Data,
    System,
}

impl PermissionDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Patient => "patient",
            Self::Device => "device",
            Self::Binding => "binding",
            Self::Data => "data",
            Self::System => "system",
        }
    }
}

/// 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionAction {
    Create,
    Read,
    Update,
    Delete,
    Manage, // 包含所有操作
}

impl PermissionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Read => "read",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Manage => "manage",
        }
    }
}

/// 系统内置权限
pub struct SystemPermissions;

impl SystemPermissions {
    // User 权限
    pub const USER_CREATE: &'static str = "user:create";
    pub const USER_READ: &'static str = "user:read";
    pub const USER_UPDATE: &'static str = "user:update";
    pub const USER_DELETE: &'static str = "user:delete";
    pub const USER_MANAGE: &'static str = "user:manage";

    // Patient 权限
    pub const PATIENT_CREATE: &'static str = "patient:create";
    pub const PATIENT_READ: &'static str = "patient:read";
    pub const PATIENT_UPDATE: &'static str = "patient:update";
    pub const PATIENT_DELETE: &'static str = "patient:delete";
    pub const PATIENT_MANAGE: &'static str = "patient:manage";

    // Device 权限
    pub const DEVICE_CREATE: &'static str = "device:create";
    pub const DEVICE_READ: &'static str = "device:read";
    pub const DEVICE_UPDATE: &'static str = "device:update";
    pub const DEVICE_DELETE: &'static str = "device:delete";
    pub const DEVICE_MANAGE: &'static str = "device:manage";

    // Binding 权限
    pub const BINDING_CREATE: &'static str = "binding:create";
    pub const BINDING_READ: &'static str = "binding:read";
    pub const BINDING_UPDATE: &'static str = "binding:update";
    pub const BINDING_DELETE: &'static str = "binding:delete";
    pub const BINDING_MANAGE: &'static str = "binding:manage";

    // Data 权限
    pub const DATA_READ: &'static str = "data:read";
    pub const DATA_EXPORT: &'static str = "data:export";
    pub const DATA_MANAGE: &'static str = "data:manage";

    // System 权限
    pub const SYSTEM_ADMIN: &'static str = "system:admin";
    pub const SYSTEM_CONFIG: &'static str = "system:config";
    pub const SYSTEM_AUDIT: &'static str = "system:audit";

    /// 构建权限字符串
    pub fn build(domain: PermissionDomain, action: PermissionAction) -> String {
        format!("{}:{}", domain.as_str(), action.as_str())
    }

    /// 获取管理员所有权限
    pub fn admin_permissions() -> HashSet<String> {
        [
            Self::USER_MANAGE,
            Self::PATIENT_MANAGE,
            Self::DEVICE_MANAGE,
            Self::BINDING_MANAGE,
            Self::DATA_MANAGE,
            Self::SYSTEM_ADMIN,
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect()
    }

    /// 获取普通用户权限
    pub fn user_permissions() -> HashSet<String> {
        [
            Self::PATIENT_READ,
            Self::PATIENT_UPDATE,
            Self::DEVICE_READ,
            Self::BINDING_READ,
            Self::DATA_READ,
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect()
    }

    /// 解析权限字符串
    pub fn parse(permission: &str) -> Option<(PermissionDomain, PermissionAction)> {
        let parts: Vec<&str> = permission.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let domain = match parts[0] {
            "user" => PermissionDomain::User,
            "patient" => PermissionDomain::Patient,
            "device" => PermissionDomain::Device,
            "binding" => PermissionDomain::Binding,
            "data" => PermissionDomain::Data,
            "system" => PermissionDomain::System,
            _ => return None,
        };

        let action = match parts[1] {
            "create" => PermissionAction::Create,
            "read" => PermissionAction::Read,
            "update" => PermissionAction::Update,
            "delete" => PermissionAction::Delete,
            "manage" => PermissionAction::Manage,
            _ => return None,
        };

        Some((domain, action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_permission() {
        assert_eq!(
            SystemPermissions::build(PermissionDomain::User, PermissionAction::Create),
            "user:create"
        );
    }

    #[test]
    fn test_parse_permission() {
        let (domain, action) = SystemPermissions::parse("device:read").unwrap();
        assert_eq!(domain, PermissionDomain::Device);
        assert_eq!(action, PermissionAction::Read);

        assert!(SystemPermissions::parse("invalid").is_none());
    }

    #[test]
    fn test_admin_permissions() {
        let perms = SystemPermissions::admin_permissions();
        assert!(perms.contains(SystemPermissions::USER_MANAGE));
        assert!(perms.contains(SystemPermissions::SYSTEM_ADMIN));
    }
}
