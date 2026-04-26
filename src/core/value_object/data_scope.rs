/// 数据范围枚举
///
/// 控制角色可以访问的数据范围：
/// - `All` — 查看所有数据（系统管理员）
/// - `Self_` — 仅查看与自己绑定的数据（普通操作员）
/// - `Department` — 查看同科室数据（预留）
#[derive(Debug, Clone, PartialEq)]
pub enum DataScope {
    All,
    Self_,
    Department,
}

impl DataScope {
    /// 从字符串解析 DataScope
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "self" => Some(Self::Self_),
            "department" => Some(Self::Department),
            _ => None,
        }
    }

    /// 转为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Self_ => "self",
            Self::Department => "department",
        }
    }

    /// 返回所有可能的 DataScope 值
    pub fn all() -> [Self; 3] {
        [Self::All, Self::Self_, Self::Department]
    }
}

impl std::fmt::Display for DataScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod data_scope_tests {
    use super::*;

    #[test]
    fn test_data_scope_from_str() {
        assert_eq!(DataScope::from_str("all"), Some(DataScope::All));
        assert_eq!(DataScope::from_str("self"), Some(DataScope::Self_));
        assert_eq!(DataScope::from_str("department"), Some(DataScope::Department));
        assert_eq!(DataScope::from_str("invalid"), None);
    }

    #[test]
    fn test_data_scope_as_str() {
        assert_eq!(DataScope::All.as_str(), "all");
        assert_eq!(DataScope::Self_.as_str(), "self");
        assert_eq!(DataScope::Department.as_str(), "department");
    }

    #[test]
    fn test_data_scope_display() {
        assert_eq!(format!("{}", DataScope::All), "all");
        assert_eq!(format!("{}", DataScope::Self_), "self");
    }

    #[test]
    fn test_data_scope_all_contains_all_variants() {
        let all = DataScope::all();
        assert!(all.contains(&DataScope::All));
        assert!(all.contains(&DataScope::Self_));
        assert!(all.contains(&DataScope::Department));
    }
}
