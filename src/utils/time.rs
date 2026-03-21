use chrono::{DateTime, Utc};

/// 格式化时间为 ISO 8601 字符串
pub fn format_iso8601(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// 解析 ISO 8601 字符串为时间
pub fn parse_iso8601(s: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// 获取当前时间
pub fn now() -> DateTime<Utc> {
    Utc::now()
}