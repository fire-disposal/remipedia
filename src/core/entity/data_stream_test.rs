#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;
    use crate::core::entity::{DataStream, DataStreamType, Observation, NewObservation};

    // ==============================
    // DataStreamType
    // ==============================

    #[test]
    fn test_data_stream_type_display() {
        assert_eq!(DataStreamType::Metric.to_string(), "metric");
        assert_eq!(DataStreamType::Event.to_string(), "event");
    }

    #[test]
    fn test_data_stream_type_from_str() {
        assert_eq!("metric".parse::<DataStreamType>().unwrap(), DataStreamType::Metric);
        assert_eq!("event".parse::<DataStreamType>().unwrap(), DataStreamType::Event);
        assert!("unknown".parse::<DataStreamType>().is_err());
    }

    #[test]
    fn test_data_stream_type_roundtrip() {
        for variant in [DataStreamType::Metric, DataStreamType::Event] {
            let s = variant.to_string();
            let parsed: DataStreamType = s.parse().unwrap();
            assert_eq!(variant, parsed);
        }
    }

    // ==============================
    // DataStream
    // ==============================

    #[test]
    fn test_data_stream_new_metric() {
        let device_id = Uuid::new_v4();
        let patient_id = Uuid::new_v4();
        let stream = DataStream::new(
            "心率".into(),
            DataStreamType::Metric,
            "heart_rate".into(),
            Some(device_id),
            Some(patient_id),
        );

        assert_eq!(stream.name, "心率");
        assert_eq!(stream.stream_type, "metric");
        assert_eq!(stream.data_type, "heart_rate");
        assert_eq!(stream.device_id, Some(device_id));
        assert_eq!(stream.patient_id, Some(patient_id));
        assert!(stream.is_active);
        assert!(stream.is_metric());
        assert!(!stream.is_event());
        assert_eq!(stream.stream_type_enum(), Some(DataStreamType::Metric));
    }

    #[test]
    fn test_data_stream_new_event() {
        let stream = DataStream::new(
            "跌倒检测".into(),
            DataStreamType::Event,
            "fall_detection".into(),
            None,
            None,
        );

        assert_eq!(stream.stream_type, "event");
        assert!(stream.is_event());
        assert!(!stream.is_metric());
        assert_eq!(stream.stream_type_enum(), Some(DataStreamType::Event));
        assert!(stream.device_id.is_none());
        assert!(stream.patient_id.is_none());
    }

    #[test]
    fn test_data_stream_default_active() {
        let stream = DataStream::new(
            "test".into(),
            DataStreamType::Metric,
            "test".into(),
            None,
            None,
        );
        assert!(stream.is_active);
        assert_eq!(stream.metadata, serde_json::json!({}));
    }

    // ==============================
    // Observation
    // ==============================

    #[test]
    fn test_observation_fields() {
        let stream_id = Uuid::new_v4();
        let patient_id = Uuid::new_v4();
        let now = Utc::now();

        let obs = Observation {
            id: Uuid::new_v4(),
            stream_id,
            patient_id,
            value_numeric: Some(rust_decimal::Decimal::new(75, 1)), // 7.5
            value_text: Some("正常".into()),
            metadata: serde_json::json!({"unit": "bpm"}),
            recorded_at: now,
        };

        assert_eq!(obs.stream_id, stream_id);
        assert_eq!(obs.patient_id, patient_id);
        assert_eq!(obs.value_text.as_deref(), Some("正常"));
        assert_eq!(obs.metadata["unit"], "bpm");
    }

    // ==============================
    // NewObservation
    // ==============================

    #[test]
    fn test_new_observation_creation() {
        let stream_id = Uuid::new_v4();
        let patient_id = Uuid::new_v4();
        let now = Utc::now();

        let new_obs = NewObservation {
            stream_id,
            patient_id,
            value_numeric: Some(rust_decimal::Decimal::new(98, 1)), // 9.8
            value_text: None,
            metadata: serde_json::json!({}),
            recorded_at: now,
        };

        assert_eq!(new_obs.stream_id, stream_id);
        assert!(new_obs.value_text.is_none());
        assert!(new_obs.value_numeric.is_some());
    }
}
