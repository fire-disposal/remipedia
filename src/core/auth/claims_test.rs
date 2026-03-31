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

    /// 测试 JWT Claims 生成 - 包含 subjects
    #[test]
    fn test_claims_generation_with_subjects() {
        let user_id = Uuid::now_v7();
        let role_id = Uuid::now_v7();
        let subjects = vec![Uuid::now_v7(), Uuid::now_v7()];
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let claims = Claims::new_access(
            &user_id,
            &role_id,
            subjects.clone(),
            expires_at,
            "test_issuer",
        );

        // 验证基本字段
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.role_id, role_id.to_string());
        assert_eq!(claims.iss, "test_issuer");
        assert_eq!(claims.token_type, "access");

        // 验证 subjects
        assert!(claims.subjects.is_some());

        // 验证 accessible_subjects 方法
        let accessible = claims.accessible_subjects();
        assert_eq!(accessible.len(), 2);

        // 验证 subjects 内容
        let claim_subjects = claims.subjects.clone().unwrap();
        assert_eq!(claim_subjects.len(), 2);
    }

    /// 测试 JWT Claims 生成 - 空 subjects
    #[test]
    fn test_claims_generation_without_subjects() {
        let user_id = Uuid::now_v7();
        let role_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let claims = Claims::new_access(
            &user_id,
            &role_id,
            vec![], // 空 subjects
            expires_at,
            "test_issuer",
        );

        // 空 subjects 应该为 None
        assert!(claims.subjects.is_none());
        assert!(claims.accessible_subjects().is_empty());
    }

    /// 测试资源访问检查
    #[test]
    fn test_can_access_subject() {
        let user_id = Uuid::now_v7();
        let role_id = Uuid::now_v7();
        let subject1 = Uuid::now_v7();
        let subject2 = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let claims = Claims::new_access(
            &user_id,
            &role_id,
            vec![subject1, subject2],
            expires_at,
            "test_issuer",
        );

        // 应该能访问列表中的 subject
        assert!(claims.can_access_subject(&subject1));
        assert!(claims.can_access_subject(&subject2));

        // 不应该能访问不在列表中的 subject
        let other_subject = Uuid::now_v7();
        assert!(!claims.can_access_subject(&other_subject));
    }

    /// 测试资源访问检查 - 无限制时
    #[test]
    fn test_can_access_subject_no_restriction() {
        let user_id = Uuid::now_v7();
        let role_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let claims = Claims::new_access(
            &user_id,
            &role_id,
            vec![], // 空列表
            expires_at,
            "test_issuer",
        );

        // subjects 为 None 时应该允许访问所有
        let any_subject = Uuid::now_v7();
        assert!(claims.can_access_subject(&any_subject));
    }

    /// 测试 refresh token claims
    #[test]
    fn test_refresh_token_claims() {
        let user_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::days(7);

        let claims = Claims::new_refresh(&user_id, expires_at, "test_issuer");

        assert_eq!(claims.sub, user_id.to_string());
        assert!(claims.role_id.is_empty());
        assert!(claims.subjects.is_none());
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
            Claims::new_access(&user_id, &role_id, vec![], expires_at, "test_issuer");

        assert!(access_claims.is_access_token());
        assert!(!access_claims.is_refresh_token());
    }
}
