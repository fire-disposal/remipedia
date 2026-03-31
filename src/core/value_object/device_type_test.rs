#[cfg(test)]
mod tests {
    use crate::core::value_object::DeviceType;
    use std::collections::HashSet;

    /// 测试 DeviceType 枚举值数量
    #[test]
    fn test_device_type_count() {
        let all_types = DeviceType::all();
        assert_eq!(all_types.len(), 3);
    }

    /// 测试所有 DeviceType 可以正确序列化和反序列化
    #[test]
    fn test_all_device_type_roundtrip() {
        for device_type in DeviceType::all() {
            let json = serde_json::to_string(device_type).unwrap();
            let decoded: DeviceType = serde_json::from_str(&json).unwrap();
            assert_eq!(*device_type, decoded);
        }
    }

    /// 测试 DeviceType 字符串表示一致性
    #[test]
    fn test_device_type_str_consistency() {
        for device_type in DeviceType::all() {
            let str_repr = device_type.as_str();
            let parsed = DeviceType::from_str(str_repr);
            assert_eq!(
                parsed,
                Some(*device_type),
                "as_str and from_str should be consistent for {:?}",
                device_type
            );
        }
    }

    /// 测试 DeviceType 反序列化大小写敏感性
    #[test]
    fn test_device_type_case_sensitivity() {
        // 正确的蛇形命名
        let valid_cases = vec![
            ("heart_rate_monitor", DeviceType::HeartRateMonitor),
            ("fall_detector", DeviceType::FallDetector),
            ("smart_mattress", DeviceType::SmartMattress),
        ];

        for (input, expected) in valid_cases {
            let parsed = DeviceType::from_str(input);
            assert_eq!(parsed, Some(expected));
        }

        // 错误的大小写
        let invalid_cases = vec![
            "HeartRateMonitor",
            "FALL_DETECTOR",
            "SmartMattress",
            "heart-rate-monitor",
            "fall.detector",
        ];

        for input in invalid_cases {
            assert_eq!(DeviceType::from_str(input), None);
        }
    }

    /// 测试 DeviceType 在 HashSet 中的使用
    #[test]
    fn test_device_type_in_hash_set() {
        let mut supported_types = HashSet::new();
        supported_types.insert(DeviceType::SmartMattress);
        supported_types.insert(DeviceType::HeartRateMonitor);

        assert!(supported_types.contains(&DeviceType::SmartMattress));
        assert!(supported_types.contains(&DeviceType::HeartRateMonitor));
        assert!(!supported_types.contains(&DeviceType::FallDetector));
    }

    /// 测试 DeviceType 的 Copy trait
    #[test]
    fn test_device_type_copy() {
        let original = DeviceType::FallDetector;
        let copied = original;

        // 使用 original 后，copied 仍然有效
        assert_eq!(original, DeviceType::FallDetector);
        assert_eq!(copied, DeviceType::FallDetector);
    }
}
