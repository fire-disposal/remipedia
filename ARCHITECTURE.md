# 架构设计

## 1. 技术栈

**Web框架**: Rocket 0.5 - 异步、类型安全  
**数据库**: PostgreSQL 16 + SQLx - 编译时SQL检查  
**异步运行时**: Tokio  
**MQTT客户端**: rumqttc 0.24  
**配置管理**: config crate - 支持环境变量覆盖  
**日志**: log + env_logger  
**错误处理**: thiserror  
**认证**: JWT(jsonwebtoken) + Argon2  
**API文档**: utoipa + Swagger UI  

## 2. 分层架构

```
API → Service → Repository → Database
```

**约束**:
- API层不访问数据库
- Service层不处理HTTP
- Repository层不包含业务逻辑
- 禁止循环依赖

## 3. 项目结构

```
src/
├── main.rs              # 应用入口
├── lib.rs               # 库入口
├── api/                 # HTTP接口（Rocket）
│   ├── routes/          # 路由定义
│   └── guards/          # 认证守卫
├── service/             # 业务逻辑
├── repository/          # 数据访问（SQLx）
├── core/                # 领域模型
│   ├── entity/          # 实体定义
│   └── value_object/    # 值对象
├── dto/                 # 数据传输对象
├── errors/              # 统一错误定义
├── config/              # 配置管理
├── ingest/              # 数据接入
│   ├── transport/       # MQTT/TCP/WebSocket
│   └── adapters/        # 设备适配器
└── utils/               # 工具函数

migrations/              # 数据库迁移
```

## 4. 数据库设计

**所有变更必须通过migration管理**：

```bash
sqlx migrate add <name>    # 创建
sqlx migrate run           # 执行
sqlx migrate revert        # 回滚
```

**禁止**: 手动修改数据库、不记录schema版本

### 核心表

**device**: 设备信息
- id (UUID PK)
- serial_number (TEXT UNIQUE)
- device_type (TEXT)
- firmware_version (TEXT)
- status (TEXT)
- metadata (JSONB)
- created_at/updated_at

**patient**: 患者信息
- id (UUID PK)
- name (TEXT)
- date_of_birth (DATE)
- gender (TEXT)
- medical_id (TEXT UNIQUE)
- contact_phone (TEXT)
- metadata (JSONB)

**binding**: 设备-患者绑定
- id (UUID PK)
- device_id (FK)
- patient_id (FK)
- started_at/ended_at
- notes (TEXT)

**datasheet**: 时序数据
- time (TIMESTAMPTZ)
- device_id (UUID FK)
- data_type (TEXT)
- payload (JSONB)
- received_at (TIMESTAMPTZ)

索引: time, device_id, data_type

## 5. SQL规范

**必须使用参数绑定**:
```rust
// 正确
sqlx::query_as!(Entity, "SELECT * FROM device WHERE id = $1", id)

// 错误（禁止）
sqlx::query_as!(Entity, &format!("SELECT * FROM device WHERE id = {}", id))
```

**禁止动态SQL结构**:
- 表名、字段名不能由用户输入控制
- ORDER BY必须使用白名单映射

**查询优化**:
- 分页使用LIMIT + OFFSET或游标分页
- 大结果集使用流式查询（fetch而非fetch_all）
- 时间范围查询必须使用索引字段

## 6. 错误处理

```rust
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

**规则**:
- 禁止unwrap()/expect()
- 使用?传播错误
- Service返回AppResult<T>
- Repository转换sqlx::Error为AppError

## 7. 配置管理

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
  enabled: true
```

环境变量覆盖（双下划线分隔）：
```bash
APP_DATABASE__URL="postgresql://..."
APP_SERVER__PORT=8080
```

## 8. API规范

### 端点设计

```
POST   /api/v1/devices              # 注册设备
GET    /api/v1/devices/:id          # 获取设备
PUT    /api/v1/devices/:id          # 更新设备
DELETE /api/v1/devices/:id          # 删除设备

POST   /api/v1/patients             # 创建患者
GET    /api/v1/patients/:id         # 获取患者
PUT    /api/v1/patients/:id         # 更新患者
DELETE /api/v1/patients/:id         # 删除患者

POST   /api/v1/bindings             # 创建绑定
GET    /api/v1/bindings             # 查询绑定
DELETE /api/v1/bindings/:id         # 解除绑定

POST   /api/v1/data                 # 上报数据
GET    /api/v1/data                 # 查询数据
```

### 响应格式

```json
// 成功
{
  "success": true,
  "data": { ... },
  "timestamp": "2024-01-01T00:00:00Z"
}

// 错误
{
  "success": false,
  "error": "错误描述",
  "code": 400,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### 分页

```
GET /api/v1/data?page=1&page_size=20&device_id=xxx

响应:
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

## 9. 日志规范

```rust
// 初始化
env_logger::Builder::from_env("RUST_LOG")
    .filter_module("remipedia", LevelFilter::Debug)
    .filter_module("sqlx", LevelFilter::Warn)
    .init();

// 使用
info!("注册设备: serial={}", serial);
```

**日志级别**:
- ERROR: 系统错误
- WARN: 潜在问题
- INFO: 关键业务事件（注册、绑定、上报）
- DEBUG: 调试信息
- TRACE: 详细流程

**运行时控制**:
```bash
RUST_LOG=remipedia=debug,sqlx=warn cargo run
```

## 10. Repository规范

```rust
pub struct DeviceRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DeviceRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
    
    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Device> { ... }
    pub async fn insert(&self, device: &NewDevice) -> AppResult<Device> { ... }
    pub async fn update(&self, id: &Uuid, data: &UpdateDevice) -> AppResult<Device> { ... }
    pub async fn delete(&self, id: &Uuid) -> AppResult<()> { ... }
}
```

**方法命名**:
- find_by_id - 按ID查询单条
- find_all/find_by_xxx - 查询多条
- insert - 插入
- update - 更新
- delete - 删除
- count_by_xxx - 计数

## 11. Service规范

```rust
pub struct DeviceService<'a> {
    device_repo: DeviceRepository<'a>,
}

impl<'a> DeviceService<'a> {
    pub async fn register(&self, req: RegisterDeviceRequest) -> AppResult<DeviceResponse> {
        // 1. 业务校验
        // 2. 检查是否已存在
        // 3. 创建实体
        // 4. 返回响应
    }
}
```

**职责**:
- 业务规则校验
- 跨Repository协调
- 事务管理（必要时）
- DTO转换

## 12. API层规范

```rust
#[post("/devices", data = "<req>")]
pub async fn register_device(
    pool: &State<PgPool>,
    req: Json<RegisterDeviceRequest>,
) -> AppResult<Json<DeviceResponse>> {
    let service = DeviceService::new(pool);
    let response = service.register(req.into_inner()).await?;
    Ok(Json(response))
}
```

**职责**:
- HTTP请求解析
- 调用Service
- HTTP响应构建

**禁止**:
- 写业务逻辑
- 写SQL
- 直接访问Repository

## 13. MQTT数据接入

### Topic设计

```
devices/{serial_number}/+     # 设备数据上报（通配）
devices/{serial_number}/data  # 具体数据类型
```

### 消息格式

```json
{
  "device_type": "heart_rate_monitor",
  "timestamp": "2024-01-01T00:00:00Z",
  "data": [72, 0]
}
```

### 处理流程

```
MQTT消息 → 解析Topic（获取serial_number）→ 自动注册/获取设备 → 获取适配器 → 解析数据 → 存储
```

## 14. 开发流程

1. 修改数据库 → 写migration
2. 执行migration
3. 编写repository
4. 编写service
5. 编写API
6. 测试

## 15. 常用命令

```bash
# 创建migration
sqlx migrate add create_device_table

# 执行migration
sqlx migrate run

# 离线检查
cargo sqlx prepare

# 运行测试
cargo test

# 格式化
cargo fmt

# 静态检查
cargo clippy

# 运行
cargo run

# 构建发布版
cargo build --release
```

## 16. 反模式

- 过度设计
- 提前优化
- ORM魔法依赖
- AI直接生成整模块
- 无migration管理
- 循环依赖
- 全局可变状态

## 17. 代码质量要求

- 能一句话解释作用
- 无跨层行为
- 无隐藏复杂度
- 无不必要抽象
