#[cfg(test)]
mod tests {
    use crate::core::auth::Claims;

    use crate::core::value_object::SystemRole;
    use chrono::Utc;
    use uuid::Uuid;

    /// 测试超级管理员 UUID 常量是否正确
    #[test]
    fn test_super_admin_id_constant() {
        let expected = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        assert_eq!(SystemRole::SUPER_ADMIN_ID, expected);
    }

    /// 测试超级管理员检查功能
    #[test]
    fn test_is_super_admin() {
        // 超级管理员 ID 应该返回 true
        assert!(SystemRole::is_super_admin(&SystemRole::SUPER_ADMIN_ID));

        // 普通 UUID 应该返回 false
        let normal_id = Uuid::now_v7();
        assert!(!SystemRole::is_super_admin(&normal_id));
    }

    /// 测试 JWT Claims 生成 - 包含模块权限
    #[test]
    fn test_claims_generation_with_modules() {
        let user_id = Uuid::now_v7();
        let role_id = Uuid::now_v7();
        let modules = vec!["dashboard".to_string(), "patients".to_string()];
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let claims = Claims::new_access(
            &user_id,
            &role_id,
            false,
            modules.clone(),
            "all",
            expires_at,
            "test_issuer",
        );

        // 验证基本字段
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.role_id, role_id.to_string());
        assert_eq!(claims.iss, "test_issuer");
        assert_eq!(claims.token_type, "access");
        assert!(!claims.is_system_role);

        // 验证模块权限
        assert_eq!(claims.modules.len(), 2);
        assert!(claims.can_access_module("dashboard"));
        assert!(claims.can_access_module("patients"));
        assert!(!claims.can_access_module("devices"));
    }

    /// 测试系统角色（通配权限）
    #[test]
    fn test_system_role_claims() {
        let user_id = Uuid::now_v7();
        let role_id = SystemRole::SUPER_ADMIN_ID;
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let claims = Claims::new_access(
            &user_id,
            &role_id,
            true,   // is_system_role
            vec![], // 空模块列表，系统角色不需要
            "all",
            expires_at,
            "test_issuer",
        );

        // 系统角色应拥有所有模块权限
        assert!(claims.is_system_role);
        assert!(claims.can_access_module("dashboard"));
        assert!(claims.can_access_module("users"));
        assert!(claims.can_access_module("nonexistent"));
    }

    /// 测试角色无模块权限
    #[test]
    fn test_claims_without_modules() {
        let user_id = Uuid::now_v7();
        let role_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let claims = Claims::new_access(
            &user_id,
            &role_id,
            false,
            vec![],
            "all",
            expires_at,
            "test_issuer",
        );

        // 非系统角色且无模块时，所有模块都应拒绝
        assert!(!claims.is_system_role);
        assert!(claims.accessible_modules().is_empty());
        assert!(!claims.can_access_module("dashboard"));
    }

    /// 测试 refresh token claims
    #[test]
    fn test_refresh_token_claims() {
        let user_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::days(7);

        let claims = Claims::new_refresh(&user_id, expires_at, "test_issuer");

        assert_eq!(claims.sub, user_id.to_string());
        assert!(claims.role_id.is_empty());
        assert!(!claims.is_system_role);
        assert!(claims.modules.is_empty());
        assert_eq!(claims.token_type, "refresh");
        assert!(claims.is_refresh_token());
        assert!(!claims.is_access_token());
    }

    /// 测试 token 类型检查
    #[test]
    fn test_token_type_checks() {
        let user_id = Uuid::now_v7();
        let role_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let access_claims =
            Claims::new_access(&user_id, &role_id, false, vec![], "all", expires_at, "test_issuer");

        assert!(access_claims.is_access_token());
        assert!(!access_claims.is_refresh_token());
    }
}
