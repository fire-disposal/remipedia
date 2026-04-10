#[cfg(test)]
mod tests {
    use crate::core::entity::{NewRole, PermissionKey, Role, UpdateRole};
    use chrono::Utc;
    use uuid::Uuid;

    /// 测试角色创建
    #[test]
    fn test_role_creation() {
        let role = Role {
            id: Uuid::now_v7(),
            name: "test_role".to_string(),
            description: Some("Test description".to_string()),
            is_system: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(role.name, "test_role");
        assert_eq!(role.description, Some("Test description".to_string()));
        assert!(!role.is_system);
    }

    /// 测试新角色数据结构
    #[test]
    fn test_new_role() {
        let new_role = NewRole {
            name: "doctor".to_string(),
            description: Some("Doctor role".to_string()),
        };

        assert_eq!(new_role.name, "doctor");
        assert_eq!(new_role.description, Some("Doctor role".to_string()));
    }

    /// 测试角色更新数据结构
    #[test]
    fn test_update_role() {
        let update = UpdateRole {
            name: Some("updated_role".to_string()),
            description: Some("Updated description".to_string()),
        };

        assert_eq!(update.name, Some("updated_role".to_string()));
        assert_eq!(update.description, Some("Updated description".to_string()));

        // 测试默认（部分更新）
        let partial_update = UpdateRole {
            name: None,
            description: Some("Only update description".to_string()),
        };

        assert!(partial_update.name.is_none());
        assert!(partial_update.description.is_some());
    }

    /// 测试权限键（PermissionKey）
    #[test]
    fn test_permission_key() {
        let key = PermissionKey::new("patient", "read");

        assert_eq!(key.resource, "patient");
        assert_eq!(key.action, "read");
        assert_eq!(key.to_string(), "patient:read");
    }

    /// 测试从元组创建 PermissionKey
    #[test]
    fn test_permission_key_from_tuple() {
        let tuple = ("device".to_string(), "create".to_string());
        let key: PermissionKey = tuple.into();

        assert_eq!(key.resource, "device");
        assert_eq!(key.action, "create");
    }

    /// 测试系统角色标记
    #[test]
    fn test_system_role_flag() {
        let system_role = Role {
            id: Uuid::now_v7(),
            name: "super_admin".to_string(),
            description: Some("System admin".to_string()),
            is_system: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let custom_role = Role {
            id: Uuid::now_v7(),
            name: "custom_role".to_string(),
            description: None,
            is_system: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(system_role.is_system);
        assert!(!custom_role.is_system);
    }

    /// 测试权限资源操作组合
    #[test]
    fn test_permission_combinations() {
        let resources = vec!["patient", "device", "binding", "data", "user"];
        let actions = vec!["create", "read", "update", "delete", "list"];

        for resource in &resources {
            for action in &actions {
                let key = PermissionKey::new(*resource, *action);
                assert_eq!(key.to_string(), format!("{}:{}", resource, action));
            }
        }
    }
}
