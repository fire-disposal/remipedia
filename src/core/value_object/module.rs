/// 系统模块枚举
///
/// 用于模块级权限控制，替代细粒度的 resource:action 权限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Module {
    /// 仪表板 - 系统首页
    Dashboard,
    /// 患者管理
    Patients,
    /// 设备管理
    Devices,
    /// 绑定关系
    Bindings,
    /// 数据查询
    Data,
    /// 用户管理
    Users,
    /// 角色管理
    Roles,
    /// 审计日志
    AuditLogs,
    /// 系统设置
    Settings,
    /// 压疮教学
    PressureUlcer,
}

impl Module {
    /// 获取模块代码
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Patients => "patients",
            Self::Devices => "devices",
            Self::Bindings => "bindings",
            Self::Data => "data",
            Self::Users => "users",
            Self::Roles => "roles",
            Self::AuditLogs => "audit_logs",
            Self::Settings => "settings",
            Self::PressureUlcer => "pressure_ulcer",
        }
    }

    /// 从字符串解析模块
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dashboard" => Some(Self::Dashboard),
            "patients" => Some(Self::Patients),
            "devices" => Some(Self::Devices),
            "bindings" => Some(Self::Bindings),
            "data" => Some(Self::Data),
            "users" => Some(Self::Users),
            "roles" => Some(Self::Roles),
            "audit_logs" => Some(Self::AuditLogs),
            "settings" => Some(Self::Settings),
            "pressure_ulcer" => Some(Self::PressureUlcer),
            _ => None,
        }
    }

    /// 获取所有模块列表
    pub fn all() -> Vec<Self> {
        vec![
            Self::Dashboard,
            Self::Patients,
            Self::Devices,
            Self::Bindings,
            Self::Data,
            Self::Users,
            Self::Roles,
            Self::AuditLogs,
            Self::Settings,
            Self::PressureUlcer,
        ]
    }

    /// 获取模块分类
    pub fn category(&self) -> &'static str {
        match self {
            Self::Dashboard | Self::Patients | Self::Devices | Self::Bindings | Self::Data => {
                "core"
            }
            Self::Users | Self::Roles | Self::AuditLogs | Self::Settings => "admin",
            Self::PressureUlcer => "feature",
        }
    }

    /// 获取模块显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dashboard => "仪表板",
            Self::Patients => "患者管理",
            Self::Devices => "设备管理",
            Self::Bindings => "绑定关系",
            Self::Data => "数据查询",
            Self::Users => "用户管理",
            Self::Roles => "角色管理",
            Self::AuditLogs => "审计日志",
            Self::Settings => "系统设置",
            Self::PressureUlcer => "压疮教学",
        }
    }
}

impl std::fmt::Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod module_tests {
    use super::*;

    #[test]
    fn test_module_as_str() {
        assert_eq!(Module::Dashboard.as_str(), "dashboard");
        assert_eq!(Module::Patients.as_str(), "patients");
        assert_eq!(Module::Users.as_str(), "users");
    }

    #[test]
    fn test_module_from_str() {
        assert_eq!(Module::from_str("dashboard"), Some(Module::Dashboard));
        assert_eq!(Module::from_str("unknown"), None);
    }

    #[test]
    fn test_module_category() {
        assert_eq!(Module::Patients.category(), "core");
        assert_eq!(Module::Users.category(), "admin");
        assert_eq!(Module::PressureUlcer.category(), "feature");
    }
}
