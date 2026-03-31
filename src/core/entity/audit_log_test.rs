#[cfg(test)]
mod tests {
    use crate::core::entity::{AuditLog, AuditLogQuery, NewAuditLog};
    use chrono::Utc;
    use uuid::Uuid;

    /// 测试审计日志创建 - 成功场景
    #[test]
    fn test_audit_log_success_creation() {
        let user_id = Uuid::now_v7();
        let log = NewAuditLog::success(
            Some(user_id),
            "create",
            "patient",
            Some("patient-123".to_string()),
        );

        assert_eq!(log.action, "create");
        assert_eq!(log.resource, "patient");
        assert_eq!(log.resource_id, Some("patient-123".to_string()));
        assert_eq!(log.status, "success");
        assert!(log.error_message.is_none());
        assert_eq!(log.user_id, Some(user_id));
    }

    /// 测试审计日志创建 - 失败场景
    #[test]
    fn test_audit_log_failure_creation() {
        let log = NewAuditLog::failure(None, "login", "auth", "Invalid credentials");

        assert_eq!(log.action, "login");
        assert_eq!(log.resource, "auth");
        assert_eq!(log.status, "failure");
        assert_eq!(log.error_message, Some("Invalid credentials".to_string()));
        assert!(log.resource_id.is_none());
        assert!(log.user_id.is_none());
    }

    /// 测试审计日志 builder 方法
    #[test]
    fn test_audit_log_builder() {
        let user_id = Uuid::now_v7();
        let log = NewAuditLog::success(
            Some(user_id),
            "update",
            "device",
            Some("device-456".to_string()),
        )
        .with_details(serde_json::json!({
            "old_value": "active",
            "new_value": "inactive"
        }))
        .with_ip("192.168.1.1".to_string())
        .with_user_agent("Mozilla/5.0".to_string())
        .with_duration(150);

        assert_eq!(log.status, "success");
        assert_eq!(log.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(log.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(log.duration_ms, Some(150));

        let details = log.details;
        assert_eq!(details["old_value"], "active");
        assert_eq!(details["new_value"], "inactive");
    }

    /// 测试审计日志查询参数
    #[test]
    fn test_audit_log_query() {
        let user_id = Uuid::now_v7();
        let start_time = Utc::now() - chrono::Duration::days(7);
        let end_time = Utc::now();

        let query = AuditLogQuery {
            user_id: Some(user_id),
            action: Some("delete".to_string()),
            resource: Some("patient".to_string()),
            status: Some("success".to_string()),
            start_time: Some(start_time),
            end_time: Some(end_time),
            page: 1,
            page_size: 20,
        };

        assert_eq!(query.user_id, Some(user_id));
        assert_eq!(query.action, Some("delete".to_string()));
        assert_eq!(query.resource, Some("patient".to_string()));
        assert_eq!(query.status, Some("success".to_string()));
        assert_eq!(query.page, 1);
        assert_eq!(query.page_size, 20);
    }

    /// 测试审计日志查询默认参数
    #[test]
    fn test_audit_log_query_default() {
        let query = AuditLogQuery::default();

        assert!(query.user_id.is_none());
        assert!(query.action.is_none());
        assert!(query.resource.is_none());
        assert!(query.status.is_none());
        assert!(query.start_time.is_none());
        assert!(query.end_time.is_none());
        assert_eq!(query.page, 0); // Default for u32
        assert_eq!(query.page_size, 0);
    }

    /// 测试审计日志实体结构
    #[test]
    fn test_audit_log_entity() {
        let log = AuditLog {
            id: Uuid::now_v7(),
            user_id: Some(Uuid::now_v7()),
            action: "export".to_string(),
            resource: "data".to_string(),
            resource_id: Some("export-789".to_string()),
            details: serde_json::json!({"format": "csv", "records": 1000}),
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            status: "success".to_string(),
            error_message: None,
            duration_ms: Some(2500),
            created_at: Utc::now(),
        };

        assert_eq!(log.action, "export");
        assert_eq!(log.resource, "data");
        assert_eq!(log.status, "success");
        assert_eq!(log.duration_ms, Some(2500));
        assert!(log.error_message.is_none());
    }

    /// 测试各种操作类型的审计日志
    #[test]
    fn test_audit_log_action_types() {
        let actions = vec![
            ("create", "patient"),
            ("read", "device"),
            ("update", "binding"),
            ("delete", "user"),
            ("login", "auth"),
            ("logout", "auth"),
            ("export", "data"),
        ];

        for (action, resource) in actions {
            let log = NewAuditLog::success(None, action, resource, None);

            assert_eq!(log.action, action);
            assert_eq!(log.resource, resource);
            assert_eq!(log.status, "success");
        }
    }
}
