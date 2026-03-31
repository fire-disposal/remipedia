#[cfg(test)]
mod tests {
    use crate::api::guards::auth::parse_permission_from_path;

    /// 测试从路径解析 GET 请求权限
    #[test]
    fn test_parse_get_permissions() {
        // 列表请求
        let (resource, action) = parse_permission_from_path("/api/v1/patients", "GET");
        assert_eq!(resource, "patient");
        assert_eq!(action, "list");

        // 单个资源请求
        let (resource, action) = parse_permission_from_path("/api/v1/patients/123", "GET");
        assert_eq!(resource, "patient");
        assert_eq!(action, "read");

        // 历史请求
        let (resource, action) = parse_permission_from_path("/api/v1/bindings/history", "GET");
        assert_eq!(resource, "binding");
        assert_eq!(action, "list");
    }

    /// 测试从路径解析 POST 请求权限
    #[test]
    fn test_parse_post_permissions() {
        // 创建请求
        let (resource, action) = parse_permission_from_path("/api/v1/patients", "POST");
        assert_eq!(resource, "patient");
        assert_eq!(action, "create");

        // 切换绑定
        let (resource, action) = parse_permission_from_path("/api/v1/bindings/switch", "POST");
        assert_eq!(resource, "binding");
        assert_eq!(action, "update");

        // 结束绑定
        let (resource, action) = parse_permission_from_path("/api/v1/bindings/123/end", "POST");
        assert_eq!(resource, "binding");
        assert_eq!(action, "update");
    }

    /// 测试从路径解析 PUT/PATCH 请求权限
    #[test]
    fn test_parse_put_permissions() {
        let (resource, action) = parse_permission_from_path("/api/v1/patients/123", "PUT");
        assert_eq!(resource, "patient");
        assert_eq!(action, "update");

        let (resource, action) = parse_permission_from_path("/api/v1/devices/456", "PATCH");
        assert_eq!(resource, "device");
        assert_eq!(action, "update");
    }

    /// 测试从路径解析 DELETE 请求权限
    #[test]
    fn test_parse_delete_permissions() {
        let (resource, action) = parse_permission_from_path("/api/v1/patients/123", "DELETE");
        assert_eq!(resource, "patient");
        assert_eq!(action, "delete");

        let (resource, action) = parse_permission_from_path("/api/v1/users/789", "DELETE");
        assert_eq!(resource, "user");
        assert_eq!(action, "delete");
    }

    /// 测试不同资源类型的路径解析
    #[test]
    fn test_parse_different_resources() {
        let test_cases = vec![
            ("/api/v1/patients", "GET", "patient", "list"),
            ("/api/v1/devices", "GET", "device", "list"),
            ("/api/v1/bindings", "GET", "binding", "list"),
            ("/api/v1/data", "GET", "data", "read"),
            ("/api/v1/users", "GET", "user", "list"),
        ];

        for (path, method, expected_resource, expected_action) in test_cases {
            let (resource, action) = parse_permission_from_path(path, method);
            assert_eq!(
                resource, expected_resource,
                "Failed for path: {}, expected resource: {}",
                path, expected_resource
            );
            assert_eq!(
                action, expected_action,
                "Failed for path: {}, expected action: {}",
                path, expected_action
            );
        }
    }

    /// 测试非标准路径
    #[test]
    fn test_parse_non_standard_paths() {
        // 直接路径（无 /api/v1 前缀）
        let (_resource, action) = parse_permission_from_path("/health", "GET");
        assert_eq!(action, "read");

        // 认证路径
        let (resource, action) = parse_permission_from_path("/api/v1/auth/login", "POST");
        assert_eq!(resource, "auth");
        assert_eq!(action, "create");
    }
}
