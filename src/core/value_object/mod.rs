//! 核心值对象

use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt;
use strum::{Display, EnumString};

/// 设备类型标识符（支持插件化扩展）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct DeviceTypeId(String);

impl Default for DeviceTypeId {
    fn default() -> Self {
        Self::new("unknown")
    }
}

impl DeviceTypeId {
    pub const HEART_RATE_MONITOR: &'static str = "heart_rate_monitor";
    pub const FALL_DETECTOR: &'static str = "fall_detector";
    pub const SMART_MATTRESS: &'static str = "smart_mattress";

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 设备状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum DeviceStatus {
    Inactive,
    Active,
    Maintenance,
}

impl DeviceStatus {
    pub fn can_transition_to(&self, new: Self) -> bool {
        !matches!((*self, new), (Self::Maintenance, Self::Inactive))
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self::Inactive
    }
}

/// 用户角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl UserRole {
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl Default for UserRole {
    fn default() -> Self {
        Self::User
    }
}

/// 用户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Inactive,
    Locked,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// 性别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    Other,
    Unknown,
}

impl Default for Gender {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 血型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum BloodType {
    APositive,
    ANegative,
    BPositive,
    BNegative,
    ABPositive,
    ABNegative,
    OPositive,
    ONegative,
    Unknown,
}

impl Default for BloodType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 用户与患者关系
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum RelationType {
    #[strum(serialize = "self")]
    Self_,
    Parent,
    Child,
    Caregiver,
    Other,
}

/// 数据来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum DataSource {
    Mqtt,
    Http,
    Tcp,
    Websocket,
}

impl Default for DataSource {
    fn default() -> Self {
        Self::Mqtt
    }
}
