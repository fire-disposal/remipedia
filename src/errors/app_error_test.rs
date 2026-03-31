#[cfg(test)]
mod tests {
    use crate::errors::AppError;

    /// 测试 AppError 的 Display 实现
    #[test]
    fn test_app_error_display() {
        let errors = vec![
            (AppError::NotFound("user".to_string()), "实体未找到: user"),
            (
                AppError::ValidationError("invalid input".to_string()),
                "验证失败: invalid input",
            ),
            (
                AppError::Unauthorized("token expired".to_string()),
                "认证失败: token expired",
            ),
            (AppError::Forbidden, "权限不足"),
            (AppError::InvalidPassword, "密码错误"),
            (AppError::UsernameExists, "用户名已存在"),
            (AppError::DeviceNotBound, "设备未绑定"),
            (AppError::BindingAlreadyExists, "绑定已存在"),
            (AppError::UuidError, "UUID 解析错误"),
            (AppError::InternalError, "内部错误"),
        ];

        for (error, expected) in errors {
            let display = format!("{}", error);
            assert_eq!(display, expected);
        }
    }

    /// 测试从 uuid::Error 转换
    #[test]
    fn test_uuid_error_conversion() {
        // 创建一个无效的 UUID 字符串来触发解析错误
        let result: Result<uuid::Uuid, _> = "not-a-uuid".parse();
        assert!(result.is_err());

        let uuid_error = result.unwrap_err();
        let app_error: AppError = uuid_error.into();

        match app_error {
            AppError::UuidError => (), // 期望的结果
            _ => panic!("Expected UuidError"),
        }
    }

    /// 测试从 sqlx::Error 转换
    #[test]
    fn test_sqlx_error_conversion() {
        let sqlx_error = sqlx::Error::RowNotFound;
        let app_error: AppError = sqlx_error.into();

        match app_error {
            AppError::DatabaseError(_) => (), // 期望的结果
            _ => panic!("Expected DatabaseError"),
        }
    }

    /// 测试错误类型匹配
    #[test]
    fn test_error_type_matching() {
        assert!(matches!(
            AppError::NotFound("test".to_string()),
            AppError::NotFound(_)
        ));
        assert!(matches!(AppError::Forbidden, AppError::Forbidden));
        assert!(matches!(AppError::InternalError, AppError::InternalError));
    }

    /// 测试错误消息包含预期内容
    #[test]
    fn test_error_messages_contain_keywords() {
        let test_cases = vec![
            (
                AppError::NotFound("resource".to_string()),
                vec!["未找到", "resource"],
            ),
            (
                AppError::ValidationError("field required".to_string()),
                vec!["验证失败", "field required"],
            ),
            (
                AppError::Unauthorized("expired".to_string()),
                vec!["认证失败", "expired"],
            ),
        ];

        for (error, keywords) in test_cases {
            let message = format!("{}", error);
            for keyword in keywords {
                assert!(
                    message.contains(keyword),
                    "Error message '{}' should contain '{}'",
                    message,
                    keyword
                );
            }
        }
    }

    /// 测试 AppError 的 Debug 实现
    #[test]
    fn test_app_error_debug() {
        let error = AppError::ValidationError("test error".to_string());
        let debug_str = format!("{:?}", error);

        assert!(debug_str.contains("ValidationError"));
        assert!(debug_str.contains("test error"));
    }
}
