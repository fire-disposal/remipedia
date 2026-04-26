/// 权限标识（用于测试兼容，旧版细粒度权限的遗留类型）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionKey {
    pub resource: String,
    pub action: String,
}

impl PermissionKey {
    pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

impl From<(String, String)> for PermissionKey {
    fn from((resource, action): (String, String)) -> Self {
        Self { resource, action }
    }
}
