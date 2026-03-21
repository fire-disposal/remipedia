# 📐 IoT Health Platform 架构与开发规范（v0.2）

## 1. 项目目标

本项目为一个实验性但具备工程可扩展性的 IoT 健康数据平台，核心能力包括：

* 多设备数据接入（IoT 节点）
* 用户与设备动态绑定
* 健康数据存储与分析（时间序列）
* 行为/事件检测（如跌倒）
* 后续支持实时流处理与扩展分析

---

## 2. 技术栈总览

### 2.1 核心框架

| 组件 | 技术选型 | 版本要求 | 说明 |
|------|---------|---------|------|
| Web 框架 | **Rocket** | 0.5.x | 异步、类型安全、内置 JSON 支持 |
| 数据库 | **PostgreSQL** | 16+ | 主数据存储 |
| 时序扩展 | **TimescaleDB** | 2.x | 时间序列数据优化（可选启用） |
| ORM/查询 | **SQLx** | 0.8.x | 编译时 SQL 检查，无 ORM 魔法 |
| 异步运行时 | **Tokio** | 1.x | Rocket 底层依赖 |

### 2.2 数据接入

| 组件 | 技术选型 | 版本要求 | 说明 |
|------|---------|---------|------|
| MQTT 客户端 | **rumqttc** | 0.24.x | 纯 Rust，异步，高性能 |
| 消息序列化 | **serde** | 1.x | JSON/MessagePack 支持 |

### 2.3 基础设施

| 组件 | 技术选型 | 版本要求 | 说明 |
|------|---------|---------|------|
| 配置管理 | **config** | 0.14.x | 多格式支持，环境变量覆盖 |
| 日志 | **log** + **env_logger** | 0.4.x / 0.11.x | 轻量级日志，环境变量控制 |
| 错误处理 | **thiserror** | 2.x | 派生错误类型 |
| 序列化 | **serde** + **serde_json** | 1.x | JSON 首选 |
| 时间处理 | **chrono** | 0.4.x | 时区感知时间 |
| UUID | **uuid** | 1.x | v7（时间排序）优先 |

### 2.4 认证与安全

| 组件 | 技术选型 | 版本要求 | 说明 |
|------|---------|---------|------|
| JWT | **jsonwebtoken** | 10.x | HS256 算法 |
| 密码哈希 | **argon2** | 0.5.x | 抗 GPU 破解 |
| 哈希算法 | **sha2** | 0.10.x | SHA-256 |

### 2.5 API 文档

| 组件 | 技术选型 | 版本要求 | 说明 |
|------|---------|---------|------|
| OpenAPI | **utoipa** | 5.x | 自动生成 OpenAPI 规范 |
| Swagger UI | **utoipa-swagger-ui** | 9.x | 交互式 API 文档 |

### 2.6 开发与测试

| 组件 | 技术选型 | 说明 |
|------|---------|------|
| Migration | **sqlx-cli** | 离线模式编译检查 |
| 单元测试 | 内置 `#[test]` | 每层独立测试 |
| 集成测试 | **sqlx::testing** | 数据库测试 |
| Mock | **mockall**（可选） | Service 层 mock |

---

## 3. Cargo.toml 依赖模板

```toml
[package]
name = "remipedia"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
# Web 框架
rocket = { version = "0.5", features = ["json"] }

# 数据库
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "chrono", "uuid", "json", "macros"] }

# 异步运行时
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"
base64 = "0.22"

# 时间
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1", features = ["v7", "serde"] }

# 配置
config = "0.14"

# 日志
log = "0.4"
env_logger = "0.11"

# 错误处理
thiserror = "2"
anyhow = "1"  # 仅用于应用启动阶段

# MQTT（数据接入）
rumqttc = "0.24"

# 密码哈希
argon2 = "0.5"

# JWT
jsonwebtoken = "10"

# 哈希
sha2 = "0.10"

# 验证
validator = { version = "0.18", features = ["derive"] }

# 异步 trait
async-trait = "0.1"

# OpenAPI / Swagger
utoipa = { version = "5", features = ["uuid", "chrono"] }
utoipa-swagger-ui = { version = "9", features = ["rocket", "vendored"] }

[dev-dependencies]
# 测试
tokio-test = "0.4"

[profile.release]
lto = "thin"
strip = true
```

---

## 4. 核心设计原则

### 4.1 MVP 优先

* 优先实现"数据流通"
* 避免过早抽象
* 不设计"未来可能用到"的复杂结构

---

### 4.2 分层架构（强约束）

系统采用固定分层：

```
API → Service → Repository → Database
```

#### 规则：

* API 层 ❌ 不允许访问数据库
* Service 层 ❌ 不处理 HTTP
* Repository 层 ❌ 不包含业务逻辑
* Database 层 ❌ 不暴露给上层

---

### 4.3 单向依赖

```
api → service → repository → db
```

禁止循环依赖或跨层调用。

---

## 5. 项目目录结构（标准）

```
src/
├── main.rs                 # 应用入口
├── lib.rs                  # 库入口，模块导出
│
├── api/                    # HTTP 接口层（Rocket）
│   ├── mod.rs
│   ├── openapi.rs          # OpenAPI 文档配置
│   ├── routes/             # 路由定义
│   │   ├── mod.rs
│   │   ├── auth.rs         # 认证相关路由
│   │   ├── health.rs       # 健康检查路由
│   │   ├── user.rs         # 用户相关路由
│   │   ├── patient.rs      # 患者相关路由
│   │   ├── device.rs       # 设备相关路由
│   │   ├── binding.rs      # 绑定相关路由
│   │   └── data.rs         # 数据上报/查询路由
│   └── guards/              # 请求守卫（认证等）
│       ├── mod.rs
│       └── auth.rs         # JWT 认证守卫
│
├── service/                # 业务逻辑层
│   ├── mod.rs
│   ├── auth.rs             # 认证服务（JWT + Refresh Token）
│   ├── user.rs             # 用户管理服务
│   ├── patient.rs          # 患者管理服务
│   ├── device.rs           # 设备管理服务
│   ├── binding.rs          # 绑定关系服务
│   └── data.rs             # 数据处理服务
│
├── repository/             # 数据访问层（SQLx）
│   ├── mod.rs
│   ├── user.rs             # 用户 CRUD
│   ├── refresh_token.rs    # 刷新令牌 CRUD
│   ├── device.rs           # 设备 CRUD
│   ├── patient.rs          # 患者 CRUD
│   ├── binding.rs          # 绑定关系 CRUD
│   └── data.rs             # 数据存储/查询
│
├── core/                   # 领域模型（纯 Rust，无外部依赖）
│   ├── mod.rs
│   ├── auth/               # 认证相关
│   │   ├── mod.rs
│   │   └── claims.rs       # JWT Claims 结构
│   ├── entity/             # 实体定义
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── refresh_token.rs
│   │   ├── patient.rs
│   │   ├── device.rs
│   │   ├── binding.rs
│   │   └── datasheet.rs
│   └── value_object/       # 值对象
│       ├── mod.rs
│       ├── user_role.rs
│       ├── device_type.rs
│       └── data_type.rs
│
├── dto/                    # 数据传输对象
│   ├── mod.rs
│   ├── request/            # 请求 DTO
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── user.rs
│   │   ├── patient.rs
│   │   ├── device.rs
│   │   └── data.rs
│   └── response/           # 响应 DTO
│       ├── mod.rs
│       ├── auth.rs
│       ├── user.rs
│       ├── patient.rs
│       ├── device.rs
│       └── data.rs
│
├── errors/                 # 统一错误定义
│   ├── mod.rs
│   └── app_error.rs        # AppError 枚举
│
├── config/                 # 配置管理
│   ├── mod.rs
│   └── settings.rs         # 配置结构体
│
├── ingest/                 # 数据接入（MQTT）
│   ├── mod.rs
│   ├── mqtt_client.rs      # MQTT 客户端封装
│   └── adapters/           # 设备适配器
│       ├── mod.rs
│       ├── adapter_trait.rs
│       ├── heart_rate.rs
│       ├── fall_detector.rs
│       └── spo2.rs
│
└── utils/                  # 工具函数
    ├── mod.rs
    └── time.rs             # 时间处理工具

migrations/                 # 数据库迁移文件
├── 20260321000000_init.up.sql
├── 20260321000000_init.down.sql
├── 20260321100000_refresh_token.up.sql
└── 20260321100000_refresh_token.down.sql
```

---

## 6. 数据库设计原则

### 6.1 Migration 强制

所有表结构变更必须通过 migration：

```bash
# 创建新迁移
sqlx migrate add <name>

# 执行迁移
sqlx migrate run

# 回滚
sqlx migrate revert
```

禁止：

* 手动修改数据库
* 不记录 schema 版本

---

### 6.2 核心表结构（初始版本）

#### device 表

```sql
CREATE TABLE device (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    serial_number   TEXT NOT NULL UNIQUE,
    device_type     TEXT NOT NULL,              -- 设备类型：heart_rate_monitor, fall_detector 等
    firmware_version TEXT,
    status          TEXT NOT NULL DEFAULT 'inactive',  -- active, inactive, maintenance
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_device_serial ON device(serial_number);
CREATE INDEX idx_device_status ON device(status);
```

#### patient 表

```sql
CREATE TABLE patient (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    date_of_birth   DATE,
    gender          TEXT,
    medical_id      TEXT UNIQUE,                -- 医疗编号
    contact_phone   TEXT,
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_patient_medical_id ON patient(medical_id);
```

#### binding 表（设备-患者绑定关系）

```sql
CREATE TABLE binding (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id       UUID NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    patient_id      UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at        TIMESTAMPTZ,                -- NULL 表示当前有效绑定
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_binding_device ON binding(device_id);
CREATE INDEX idx_binding_patient ON binding(patient_id);
CREATE INDEX idx_binding_active ON binding(device_id, patient_id) WHERE ended_at IS NULL;
```

#### datasheet 表（核心数据表）

```sql
CREATE TABLE datasheet (
    time            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    device_id       UUID NOT NULL REFERENCES device(id),
    data_type       TEXT NOT NULL,              -- 数据类型：heart_rate, fall_event, etc.
    payload         JSONB NOT NULL,              -- 灵活的负载数据
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- TimescaleDB 扩展（可选，后期启用）
-- SELECT create_hypertable('datasheet', 'time');

CREATE INDEX idx_datasheet_time ON datasheet(time DESC);
CREATE INDEX idx_datasheet_device ON datasheet(device_id, time DESC);
CREATE INDEX idx_datasheet_type ON datasheet(data_type, time DESC);
```

---

### 6.3 数据策略

* 初期采用 **半结构化（JSONB）**
* 后期按需结构化拆分
* 所有数据必须带时间字段（为时间序列优化准备）

---

## 7. SQL 使用规范（强约束）

### 7.1 必须使用参数绑定

✅ 正确：

```rust
sqlx::query_as!(
    Entity,
    "SELECT * FROM device WHERE id = $1",
    id
)
```

❌ 禁止：

```rust
sqlx::query_as!(
    Entity,
    &format!("SELECT * FROM device WHERE id = {}", id)
)
```

---

### 7.2 禁止动态 SQL 结构

用户输入不能控制：

* 表名
* 字段名
* ORDER BY（必须使用白名单映射）

---

### 7.3 SQL 归属规则

所有 SQL 必须只存在于：

```
repository/
```

---

### 7.4 查询优化规范

* 分页查询必须使用 `LIMIT` + `OFFSET` 或游标分页
* 大结果集必须使用流式查询（`fetch` 而非 `fetch_all`）
* 时间范围查询必须使用索引字段

---

## 8. 错误处理规范

### 8.1 统一错误类型

```rust
// src/errors/app_error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("实体未找到: {0}")]
    NotFound(String),

    #[error("验证失败: {0}")]
    ValidationError(String),

    #[error("设备未绑定")]
    DeviceNotBound,

    #[error("绑定已存在")]
    BindingAlreadyExists,

    #[error("内部错误")]
    InternalError,
}

pub type AppResult<T> = Result<T, AppError>;
```

### 8.2 Rocket 错误响应

```rust
// 实现 Rocket 的 Responder
impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &rocket::Request<'_>) -> rocket::response::Result<'r> {
        let status = match &self {
            AppError::NotFound(_) => Status::NotFound,
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::DeviceNotBound => Status::BadRequest,
            _ => Status::InternalServerError,
        };

        let json = json!({
            "error": self.to_string(),
            "code": status.code,
        });

        rocket::response::Response::build_from(json.respond_to(req)?)
            .status(status)
            .ok()
    }
}
```

### 8.3 错误处理规则

* 禁止使用 `unwrap()` 或 `expect()`
* 使用 `?` 运算符传播错误
* Service 层返回 `AppResult<T>`
* Repository 层将 `sqlx::Error` 转换为 `AppError`

---

## 9. 配置管理

### 9.1 配置文件结构

```rust
// src/config/settings.rs
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub mqtt: MqttConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MqttConfig {
    pub broker: String,
    pub port: u16,
    pub client_id: String,
    pub topic_prefix: String,
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::Environment::with_prefix("APP"))
            .build()?;

        config.try_deserialize()
    }
}
```

### 9.2 配置文件示例

```yaml
# config/default.yaml
database:
  url: "postgresql://localhost/remipedia"
  max_connections: 10
  min_connections: 2

server:
  host: "0.0.0.0"
  port: 8000

mqtt:
  broker: "localhost"
  port: 1883
  client_id: "remipedia-server"
  topic_prefix: "remipedia"
```

### 9.3 环境变量覆盖

```bash
# 环境变量优先级高于配置文件
APP_DATABASE__URL="postgresql://user:pass@host/db"
APP_SERVER__PORT=8080
```

---

## 10. API 设计规范

### 10.1 RESTful 端点设计

```
POST   /api/v1/devices              # 注册设备
GET    /api/v1/devices/:id          # 获取设备信息
PUT    /api/v1/devices/:id           # 更新设备信息
DELETE /api/v1/devices/:id           # 删除设备

POST   /api/v1/patients              # 创建患者
GET    /api/v1/patients/:id          # 获取患者信息
PUT    /api/v1/patients/:id          # 更新患者信息
DELETE /api/v1/patients/:id          # 删除患者

POST   /api/v1/bindings              # 创建绑定
GET    /api/v1/bindings              # 查询绑定列表
DELETE /api/v1/bindings/:id          # 解除绑定

POST   /api/v1/data                  # 上报数据
GET    /api/v1/data                  # 查询数据（支持时间范围、设备ID过滤）
```

### 10.2 请求/响应格式

```json
// 统一响应格式
{
  "success": true,
  "data": { ... },
  "timestamp": "2024-01-01T00:00:00Z"
}

// 错误响应格式
{
  "success": false,
  "error": "错误描述",
  "code": 400,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### 10.3 分页规范

```json
// 请求
GET /api/v1/data?page=1&page_size=20&device_id=xxx&start_time=xxx&end_time=xxx

// 响应
{
  "success": true,
  "data": [...],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total": 100,
    "total_pages": 5
  }
}
```

---

## 11. 日志规范

### 11.1 日志配置

```rust
// main.rs
use log::LevelFilter;

fn init_logging() {
    env_logger::Builder::from_env("RUST_LOG")
        .filter_module("remipedia", LevelFilter::Debug)
        .filter_module("sqlx", LevelFilter::Warn)
        .filter_module("rocket", LevelFilter::Info)
        .init();
}
```

### 11.2 日志级别使用

| 级别 | 使用场景 |
|------|---------|
| ERROR | 系统错误、异常状态 |
| WARN | 潜在问题、降级操作 |
| INFO | 关键业务事件（设备注册、绑定、数据上报） |
| DEBUG | 开发调试信息 |
| TRACE | 详细执行流程 |

### 11.3 日志使用示例

```rust
use log::{info, debug, error};

pub async fn register_device(pool: &PgPool, req: RegisterDeviceRequest) -> AppResult<Device> {
    info!(
        "注册新设备: serial={}, type={}",
        req.serial_number,
        req.device_type
    );
    // ...
}
```

### 11.4 运行时控制

```bash
# 设置日志级别
RUST_LOG=remipedia=debug,sqlx=warn cargo run

# 仅显示错误
RUST_LOG=error cargo run

# 详细调试
RUST_LOG=remipedia=trace cargo run
```

---

## 12. 测试规范

### 12.1 测试分层

| 层级 | 测试类型 | 工具 | 范围 |
|------|---------|------|------|
| Repository | 集成测试 | sqlx::testing | 数据库操作 |
| Service | 单元测试 | #[test] | 业务逻辑 |
| API | 集成测试 | Rocket local client | HTTP 接口 |

### 12.2 Repository 测试示例

```rust
#[sqlx::test]
async fn test_insert_device(pool: PgPool) {
    let repo = DeviceRepository::new(pool);

    let device = repo.insert(NewDevice {
        serial_number: "TEST-001".to_string(),
        device_type: "heart_rate_monitor".to_string(),
    }).await.unwrap();

    assert_eq!(device.serial_number, "TEST-001");
}
```

### 12.3 Service 测试示例

```rust
#[tokio::test]
async fn test_bind_device_to_patient() {
    // 使用 mock 或内存数据库
    let service = DeviceService::new(mock_repo);

    let result = service.bind_device("device-1", "patient-1").await;

    assert!(result.is_ok());
}
```

---

## 13. AI 代码使用规范（关键）

### 13.1 AI 禁止事项

AI 不允许：

* 修改项目结构
* 设计数据库 schema
* 定义系统架构
* 引入复杂抽象（泛型/宏）

---

### 13.2 AI 允许范围

AI 可用于：

* CRUD 实现
* SQL 查询编写
* DTO struct
* 测试代码

---

### 13.3 AI 输出约束

所有 AI 生成代码必须满足：

* 符合分层结构
* 不跨层调用
* 使用已有模板
* 不引入新范式

---

### 13.4 AI 使用模式（推荐）

❌ 错误：

> "帮我写一个系统"

✅ 正确：

> "实现这个 repository 函数，使用 SQLx，遵循现有结构"

---

## 14. Repository 层规范（模板）

### 14.1 基本模板

```rust
// src/repository/device.rs
use sqlx::PgPool;
use crate::core::entity::Device;
use crate::errors::{AppError, AppResult};

pub struct DeviceRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DeviceRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &uuid::Uuid) -> AppResult<Device> {
        sqlx::query_as!(
            Device,
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device WHERE id = $1"#,
            id
        )
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("设备: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }

    pub async fn insert(&self, device: &NewDevice) -> AppResult<Device> {
        sqlx::query_as!(
            Device,
            r#"INSERT INTO device (serial_number, device_type, firmware_version, status, metadata)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at"#,
            device.serial_number,
            device.device_type,
            device.firmware_version,
            device.status,
            device.metadata,
        )
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }
}
```

### 14.2 查询方法命名规范

| 方法名 | 用途 | 返回类型 |
|--------|------|---------|
| `find_by_id` | 按 ID 查询单条 | `AppResult<T>` |
| `find_all` | 查询所有 | `AppResult<Vec<T>>` |
| `find_by_xxx` | 按条件查询 | `AppResult<Vec<T>>` |
| `insert` | 插入 | `AppResult<T>` |
| `update` | 更新 | `AppResult<T>` |
| `delete` | 删除 | `AppResult<()>` |
| `count_by_xxx` | 计数 | `AppResult<i64>` |

---

## 15. Service 层规范

### 15.1 基本模板

```rust
// src/service/device.rs
use crate::repository::DeviceRepository;
use crate::dto::request::RegisterDeviceRequest;
use crate::dto::response::DeviceResponse;
use crate::errors::{AppError, AppResult};

pub struct DeviceService<'a> {
    device_repo: DeviceRepository<'a>,
}

impl<'a> DeviceService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            device_repo: DeviceRepository::new(pool),
        }
    }

    pub async fn register(&self, req: RegisterDeviceRequest) -> AppResult<DeviceResponse> {
        // 1. 业务校验
        if req.serial_number.is_empty() {
            return Err(AppError::ValidationError("序列号不能为空".into()));
        }

        // 2. 检查是否已存在
        if self.device_repo.exists_by_serial(&req.serial_number).await? {
            return Err(AppError::ValidationError("设备已存在".into()));
        }

        // 3. 创建实体
        let device = self.device_repo.insert(&req.into()).await?;

        // 4. 返回响应
        Ok(device.into())
    }
}
```

### 15.2 Service 层职责

* 业务规则校验
* 跨 Repository 协调
* 事务管理（必要时）
* DTO 转换

---

## 16. API 层规范

### 16.1 基本模板

```rust
// src/api/routes/device.rs
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;

use crate::dto::request::RegisterDeviceRequest;
use crate::dto::response::DeviceResponse;
use crate::service::DeviceService;
use crate::errors::AppResult;

#[post("/devices", data = "<req>")]
pub async fn register_device(
    pool: &State<PgPool>,
    req: Json<RegisterDeviceRequest>,
) -> AppResult<Json<DeviceResponse>> {
    let service = DeviceService::new(pool);
    let response = service.register(req.into_inner()).await?;
    Ok(Json(response))
}

#[get("/devices/<id>")]
pub async fn get_device(
    pool: &State<PgPool>,
    id: uuid::Uuid,
) -> AppResult<Json<DeviceResponse>> {
    let service = DeviceService::new(pool);
    let response = service.get_by_id(&id).await?;
    Ok(Json(response))
}
```

### 16.2 API 层职责

* HTTP 请求解析
* 调用 Service
* HTTP 响应构建

禁止：

* 写业务逻辑
* 写 SQL
* 直接访问 Repository

---

## 17. MQTT 数据接入规范

### 17.1 Topic 设计

```
remipedia/{device_id}/data        # 设备数据上报
remipedia/{device_id}/status      # 设备状态更新
remipedia/{device_id}/command     # 下发命令（预留）
```

### 17.2 消息格式

```json
{
  "device_id": "uuid",
  "timestamp": "2024-01-01T00:00:00Z",
  "data_type": "heart_rate",
  "payload": {
    "value": 72,
    "unit": "bpm"
  }
}
```

### 17.3 客户端实现

```rust
// src/ingest/mqtt_client.rs
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use crate::service::DataService;

pub struct MqttIngest {
    client: AsyncClient,
}

impl MqttIngest {
    pub async fn new(broker: &str, port: u16) -> Self {
        let mut options = MqttOptions::new("remipedia-server", broker, port);
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, mut eventloop) = AsyncClient::new(options, 10);

        // 启动事件循环
        tokio::spawn(async move {
            while let Ok(notification) = eventloop.poll().await {
                if let Event::Incoming(Incoming::Publish(publish)) = notification {
                    // 处理消息
                    Self::handle_message(publish).await;
                }
            }
        });

        Self { client }
    }

    pub async fn subscribe(&self, topic: &str) {
        self.client.subscribe(topic, QoS::AtLeastOnce).await.unwrap();
    }

    async fn handle_message(publish: rumqttc::Publish) {
        // 解析并存储数据
    }
}
```

---

## 18. 时间序列扩展（未来）

预留支持：

* TimescaleDB hypertable
* time_bucket 聚合
* 数据保留策略
* 数据压缩

### 18.1 TimescaleDB 启用（后期）

```sql
-- 启用扩展
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- 转换为 hypertable
SELECT create_hypertable('datasheet', 'time');

-- 设置保留策略（保留 90 天）
SELECT add_retention_policy('datasheet', INTERVAL '90 days');

-- 设置压缩策略
ALTER TABLE datasheet SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'device_id'
);
SELECT add_compression_policy('datasheet', INTERVAL '7 days');
```

---

## 19. 开发流程（标准）

1. 修改数据库 → 写 migration
2. 执行 migration
3. 编写 repository
4. 编写 service
5. 编写 API
6. 测试

### 19.1 开发命令速查

```bash
# 创建 migration
sqlx migrate add create_device_table

# 执行 migration
sqlx migrate run

# 离线模式编译检查（推荐）
cargo sqlx prepare

# 运行测试
cargo test

# 运行项目
cargo run

# 构建发布版本
cargo build --release
```

---

## 20. 代码质量最低标准

每段代码必须满足：

* 能一句话解释作用
* 无跨层行为
* 无隐藏复杂度
* 无不必要抽象

---

## 21. 反模式（必须避免）

* ❌ 过度设计
* ❌ 提前优化
* ❌ ORM 魔法依赖
* ❌ AI 直接生成整模块
* ❌ 无 migration 管理
* ❌ 循环依赖
* ❌ 全局可变状态

---

## 22. 当前阶段目标

当前阶段只关注：

```
设备注册
→ 用户绑定
→ 数据上报
→ 数据查询
```

### 22.1 MVP 功能清单

| 功能 | 优先级 | 状态 |
|------|--------|------|
| 设备注册 API | P0 | 待开发 |
| 患者管理 API | P0 | 待开发 |
| 设备-患者绑定 | P0 | 待开发 |
| 数据上报 API | P0 | 待开发 |
| 数据查询 API | P0 | 待开发 |
| MQTT 数据接入 | P1 | 待开发 |
| TimescaleDB 集成 | P2 | 待规划 |

---

## 23. 附录：常用命令

### 23.1 Cargo 命令

```bash
# 检查编译
cargo check

# 运行测试
cargo test

# 运行测试（显示输出）
cargo test -- --nocapture

# 代码格式化
cargo fmt

# 静态检查
cargo clippy

# 生成文档
cargo doc --open
```

### 23.2 SQLx 命令

```bash
# 创建 migration
sqlx migrate add <name>

# 执行 migration
sqlx migrate run

# 回滚 migration
sqlx migrate revert

# 生成离线检查文件
cargo sqlx prepare
```

### 23.3 Docker 命令（开发环境）

```bash
# 启动 PostgreSQL
docker run -d --name remipedia-db \
  -e POSTGRES_DB=remipedia \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres:16

# 启动 MQTT Broker
docker run -d --name remipedia-mqtt \
  -p 1883:1883 \
  eclipse-mosquitto:2
```
