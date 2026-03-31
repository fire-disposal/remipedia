use uuid::Uuid;

/// 系统内置角色
///
/// 注意：只有 SuperAdmin 是硬编码的系统角色，
/// 其他角色全部通过数据库动态管理
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemRole;

impl SystemRole {
    /// 超级管理员角色的 UUID（硬编码）
    pub const SUPER_ADMIN_ID: Uuid = Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0001);

    /// 检查是否为超级管理员
    pub fn is_super_admin(role_id: &Uuid) -> bool {
        *role_id == Self::SUPER_ADMIN_ID
    }
}
