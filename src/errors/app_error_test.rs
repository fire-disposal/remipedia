#[cfg(test)]
mod tests {
    use crate::errors::AppError;

    /// 测试 AppError 的 Display 实现
    #[test]
    fn test_app_error_display() {
        let errors = vec![
            (AppError::NotFound("user".to_string()), "未找到: user"),
            (
                AppError::ValidationError("invalid input".to_string()),
                "请求无效: invalid input",
            ),
            (
                AppError::Unauthorized("token expired".to_string()),
                "认证失败: token expired",
            ),
            (AppError::Forbidden, "权限不足"),
            (
                AppError::Conflict("用户名已存在".to_string()),
                "资源冲突: 用户名已存在",
            ),
            (
                AppError::InternalError("内部错误".to_string()),
                "内部错误: 内部错误",
            ),
        ];

        for (error, expected) in errors {
            let display = format!("{}", error);
            assert_eq!(display, expected);
        }
    }

    /// 测试从 uuid::Error 转换
    #[test]
    fn test_uuid_error_conversion() {
        let result: Result<uuid::Uuid, _> = "not-a-uuid".parse();
        assert!(result.is_err());

        let uuid_error = result.unwrap_err();
        let app_error: AppError = uuid_error.into();

        match app_error {
            AppError::ValidationError(_) => (), // 合并后为 ValidationError
            _ => panic!("Expected ValidationError"),
        }
    }

    /// 测试从 sqlx::Error 转换
    #[test]
    fn test_sqlx_error_conversion() {
        let sqlx_error = sqlx::Error::RowNotFound;
        let app_error: AppError = sqlx_error.into();

        match app_error {
            AppError::DatabaseError(_) => (),
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
        assert!(matches!(
            AppError::InternalError("err".to_string()),
            AppError::InternalError(_)
        ));
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
                vec!["请求无效", "field required"],
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

    /// 测试辅助构造方法
    #[test]
    fn test_error_helper_methods() {
        assert!(matches!(AppError::not_found("用户"), AppError::NotFound(_)));
        assert!(matches!(AppError::validation("无效参数"), AppError::ValidationError(_)));
        assert!(matches!(AppError::unauthorized("过期"), AppError::Unauthorized(_)));
        assert!(matches!(AppError::conflict("重复"), AppError::Conflict(_)));
        assert!(matches!(AppError::internal("失败"), AppError::InternalError(_)));

        let msg = format!("{}", AppError::not_found("用户: 123"));
        assert!(msg.contains("用户: 123"), "消息应包含具体内容: {}", msg);
    }

    /// 测试 From<anyhow::Error>
    #[test]
    fn test_anyhow_error_conversion() {
        let any_err = anyhow::anyhow!("自定义错误");
        let app_err: AppError = any_err.into();
        assert!(matches!(app_err, AppError::InternalError(_)));
        assert!(format!("{}", app_err).contains("自定义错误"));
    }
}
