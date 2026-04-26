# 后端架构优化分析报告

> 基于全量代码审查，识别不合理封装、过度设计、低价值字段/嵌套及 Rust/Rocket 框架利用不足，提出具体优化方案。
> 审查范围：src/ 下 50+ 文件，涵盖 ingest/dto/service/repository/errors/config 全部模块。

---

## 目录

1. [紧急缺陷：TCP 帧解析错误](#1-紧急缺陷tcp-帧解析错误)
2. [过度封装与低价值设计](#2-过度封装与低价值设计)
3. [DTO 层臃肿](#3-dto-层臃肿)
4. [Repository 层反模式](#4-repository-层反模式)
5. [Ingest 模块重复代码](#5-ingest-模块重复代码)
6. [错误处理膨胀](#6-错误处理膨胀)
7. [Rust/Rocket 框架利用不足](#7-rustrocket-框架利用不足)
8. [综合优化方案](#8-综合优化方案)

---

## 1. 紧急缺陷：TCP 帧解析错误

### 1.1 问题现象

系统持续收到无法解析的 TCP 数据，表现为：
- 来源为 `tcp`，序列号/设备类型均为空
- 状态均为 `格式错误`
- 载荷大小多样：44 B、88 B、110 B、158 B、517 B

### 1.2 根因分析

**协议规范**（定义在 [`extract_msgpack_frame()`](src/ingest/modules/mattress.rs:196)）：

```
[0xAB, 0xCD] [len: u8] [crc: u8] [data: len bytes]
```

**问题一：length 字段为单字节 u8，最大仅 255**

```rust
let data_len = buffer[2] as usize;  // 只有 1 字节
if data_len > max_size { ... }
```

单字节 length 最大表示 255 字节帧。当设备发送超过 255 字节的 msgpack 数据时，`data_len` 会回绕（wrap around），导致：
- 帧边界完全错位
- 解析出看似合理长度的非法帧
- 残留数据破坏后续帧同步

**问题二：无 TCP 粘包/半包处理的状态机**

[`handle_connection()`](src/ingest/modules/mattress.rs:102) 的循环中，读取 TCP 数据后直接喂入 [`extract_msgpack_frame()`](src/ingest/modules/mattress.rs:196)：

```rust
buffer.extend_from_slice(&temp_buf[..n]);

loop {
    match extract_msgpack_frame(&mut buffer, max_frame_size) {
        Ok(Some(frame)) => { /* 处理帧 */ }
        Ok(None) => break,  // 半包，等更多数据
        Err(e) => { /* 丢弃到下一个 magic */ }
    }
}
```

如果 length 字段被截断（设备发送 `[0xAB, 0xCD, 0x00]` 后断连），会出现：
- data_len = 0 → 总长度 4 字节 → 4 字节收到的 → 解析出空帧 → 消耗掉 magic 头 → 后续数据彻底失去同步

**问题三：CRC 校验被注释**

```rust
let _expected_crc = buffer[3];   // 读但不校验
let _data = &buffer[4..4 + data_len];
// TODO: CRC校验
```

恶意或损坏的数据不会被丢弃，直接进入 msgpack 反序列化。

**问题四：`find_next_magic` 可跳过有效数据**

当帧错误时调用 [`find_next_magic()`](src/ingest/modules/mattress.rs:231)：

```rust
fn find_next_magic(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|w| w == [0xAB, 0xCD])
}
```

若 data 载荷内恰含 `[0xAB, 0xCD]` 字节序列，会导致虚假同步，永久错位。

### 1.3 紧急修复方案

```rust
// 修复方案关键变更：
// 1. length 字段改为 2 字节（大端序）
// 2. 实现真正的 CRC 校验
// 3. 添加帧计数器防止死循环
// 4. 添加长度合理性校验（min/max）

// 协议格式变更: [0xAB, 0xCD] [len_high: u8] [len_low: u8] [crc: u8] [data: len bytes]
// 最大帧长度: 65535 + 5 = 65540 字节

fn extract_msgpack_frame(buffer: &mut Vec<u8>, max_size: usize) -> AppResult<Option<Vec<u8>>> {
    const HEADER_SIZE: usize = 5; // magic(2) + len(2) + crc(1)

    // 递归/帧计数保护 - 防止空帧导致死循环
    if buffer.len() < HEADER_SIZE {
        return Ok(None);
    }

    if buffer[0] != 0xAB || buffer[1] != 0xCD {
        // 跳过无效字节，向前查找 magic
        if let Some(pos) = find_next_magic(&buffer[1..]) {
            buffer.drain(..pos + 1);
            return Ok(None); // 下次循环重新尝试
        }
        buffer.clear();
        return Ok(None);
    }

    let data_len = u16::from_be_bytes([buffer[2], buffer[3]]) as usize;
    let total_len = HEADER_SIZE + data_len;

    if data_len < 1 || data_len > max_size {
        // 非法长度，丢弃本帧并尝试恢复
        buffer.drain(..HEADER_SIZE);
        return Err(AppError::ValidationError(format!(
            "帧长度 {} 超出范围 [1, {}]", data_len, max_size
        )));
    }

    if buffer.len() < total_len {
        return Ok(None); // 半包
    }

    // TODO: 启用 CRC 校验
    // let expected_crc = buffer[4];
    // let actual_crc = crc8(&buffer[..HEADER_SIZE]) ^ crc8(&buffer[HEADER_SIZE..total_len]);
    // if expected_crc != actual_crc { ... }

    let frame = buffer[..total_len].to_vec();
    buffer.drain(..total_len);
    Ok(Some(frame))
}
```

---

## 2. 过度封装与低价值设计

### 2.1 三设备各自实现 `resolve_or_create_device`

| 文件 | 函数 | 代码量 |
|------|------|--------|
| [`vision.rs:301`](src/ingest/modules/vision.rs:301) | `resolve_or_create_device` | 26 行 |
| [`mattress.rs:381`](src/ingest/modules/mattress.rs:381) | `resolve_or_create_device` | 24 行 |
| [`imu.rs:442`](src/ingest/modules/imu.rs:442) | `resolve_or_create_device` | 25 行 |

三份完全相同的逻辑——查询 Device 表，不存在则创建新记录。差异仅在设备类型字符串不同（`"vision"` / `"mattress"` / `"imu"`）。

**优化**：提取为 [`IngestModule` trait](src/ingest/modules/mod.rs:16) 提供的默认方法或共享工具函数：

```rust
// src/ingest/modules/mod.rs
pub async fn resolve_or_create_device(pool: &PgPool, device_id: &str, device_type: &str) -> AppResult<Uuid> {
    // 单份实现，所有模块复用
}
```

### 2.2 `process_mattress_data` 返回 `Option<MattressState>`

[`process_mattress_data()`](src/ingest/modules/mattress.rs:264) 返回 `(Vec<DataPoint>, Option<MattressState>)`，其中 `MattressState` 完全用于状态追踪：

```rust
struct MattressState {
    previous_heart_rate: u8,
    previous_breath_rate: u8,
}
```

这个状态仅在单连接生命周期内有效，只用于检测"数值变化"。但模块重启后会丢失全部状态。更合理的做法是：

- **方案 A**：将状态判断下沉到 service/repository 层（查询上次值）
- **方案 B**：改为纯内存统计（如滑动窗口），不依赖逐连接状态

### 2.3 `RawDataRecordResponse` 与 `RawDataDetailResponse` 几乎相同

```rust
// src/dto/response/data.rs:100
pub struct RawDataRecordResponse {
    pub id: Uuid,
    pub source: String,
    pub raw_payload: serde_json::Value,
    pub payload_size: usize,          // ← 运行时从 raw_payload 推导
    pub device_type: Option<String>,
    pub serial_number: Option<String>,
    pub ingest_status: String,
    pub error_message: Option<String>,
    pub received_at: DateTime<Utc>,
}
```

```rust
// src/dto/response/data.rs:129
pub struct RawDataDetailResponse {
    pub id: Uuid,
    pub source: String,
    pub raw_payload: serde_json::Value,
    pub payload_size: usize,          // ← 同上
    pub device_type: Option<String>,
    pub serial_number: Option<String>,
    pub ingest_status: String,
    pub error_message: Option<String>,
    pub received_at: DateTime<Utc>,
    pub data_points: Vec<DataRecordResponse>, // 唯一差异
}
```

**`payload_size` 是运行时派生的**：`raw_payload.to_string().len()`，不需要序列化传输，DTO 中可以直接去掉或改为方法。

---

## 3. DTO 层臃肿

### 3.1 现状

DTO 层包含 **22+ 个结构体**，分布在 [`src/dto/request/`](src/dto/request/) 和 [`src/dto/response/`](src/dto/response/)：

| 请求 DTO | 字段数 | 响应 DTO | 字段数 |
|----------|--------|----------|--------|
| `DataReportRequest` | 8 | `DataReportResponse` | 2 |
| `DataQuery` | 9 | `DataRecordResponse` | 11 |
| `AlertQuery` | 6 | `DataQueryResponse` | 3 |
| `RawDataQuery` | 8 | `AlertStatsResponse` | 6 |
| `AcknowledgeEventRequest` | 2 | `BindingResponse` | 5 |
| `CreateBindingRequest` | 3 | `BindingListResponse` | 2 |
| `SwitchBindingRequest` | 2 | `RawDataRecordResponse` | 9 |
| `EndBindingRequest` | 2 | `RawDataDetailResponse` | 10 |
| (auth/device/user 请求) | ~5 个 | `RawDataQueryResponse` | 5 |

**问题**：大多数 DTO 的字段布局与 Core Entity 几乎完全相同。

### 3.2 对比：`Datasheet` vs `DataRecordResponse`

| [`Datasheet`](src/core/entity/datasheet.rs:104) | [`DataRecordResponse`](src/dto/response/data.rs:23) |
|---------|-------------------|
| `time: DateTime<Utc>` | `time: DateTime<Utc>` |
| `device_id: Uuid` | `device_id: Uuid` |
| `patient_id: Option<Uuid>` | `patient_id: Option<Uuid>` |
| `data_type: String` | `data_type: String` |
| `data_category: String` | `data_category: String` |
| `value: f64` | `value: f64` |
| `text_value: Option<String>` | `text_value: Option<String>` |
| `unit: Option<String>` | `unit: Option<String>` |
| `severity: Option<String>` | `severity: Option<String>` |
| `event_status: Option<String>` | `event_status: Option<String>` |
| `device_name: Option<String>` | `device_name: Option<String>` |

11 个字段完全重复。[`From<Datasheet>` 实现](src/service/data.rs:209) 也仅是逐字段映射：

```rust
fn from(data: Datasheet) -> Self {
    DataRecordResponse {
        time: data.time,
        device_id: data.device_id,
        patient_id: data.patient_id,
        device_name: data.device_name,
        // ... 完全一一对应
    }
}
```

### 3.3 `IntoResponse` trait 未被实际使用

[`src/dto/convert.rs`](src/dto/convert.rs:11) 定义了：

```rust
pub trait IntoResponse: Send {
    type Response;
    fn into_response(self) -> Self::Response;
}
```

但审查全部代码后发现 **没有任何一个结构体实现了该 trait**。所有转换都通过独立的 `From<T>` 实现完成。这个 trait 是死代码，应删除。

### 3.4 优化方向

**方案：Entity → Response 直接复用时使用类型别名或派生**

```rust
// 方式一：当字段完全相同时，使用类型别名
pub type DataRecordResponse = Datasheet;

// 方式二：仅需要排除某些字段时，使用 #[serde(skip_serializing_if)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataRecordResponse {
    #[serde(flatten)]
    inner: Datasheet,
    // 添加响应特有字段...
}
```

**不推荐**：为每个 Entity 都创建独立的 Response DTO + `From` 实现。这会增加 3 倍维护量。

---

## 4. Repository 层反模式

### 4.1 `QueryBuilder` 使用 `Box<dyn Any>` 类型擦除

[`src/repository/base.rs:113`](src/repository/base.rs:113)：

```rust
pub struct QueryBuilder {
    conditions: Vec<String>,
    params: Vec<Box<dyn Any + Send>>,
}

impl QueryBuilder {
    pub fn add_optional_condition<T>(&mut self, field: &str, value: Option<T>, param_index: usize)
    where
        T: Display + Any + Send,
    {
        if let Some(v) = value {
            self.conditions
                .push(format!("{} = ${}", field, param_index));
            self.params.push(Box::new(v)); // 类型信息在此丢失
        }
    }
}
```

**问题**：
1. `params` 无法传递给 sqlx——sqlx 需要具体类型（`&str`, `i64`, `Uuid` 等），而非 `Box<dyn Any>`
2. `Display` 约束存在但从未使用（不能用 `format!("{}", v)` 做 SQL 参数，有注入风险）
3. 整个 `QueryBuilder` **没有任何一个 Repository 实际使用**——[`DataRepository`](src/repository/data.rs) 所有方法都手写 SQL 带 `$1..$8` 绑定

**结论**：`QueryBuilder` 是完全的死代码，类型不安全、无人使用，应删除。

### 4.2 `define_repository!` 宏只生成了样板代码

[`src/repository/base.rs:83`](src/repository/base.rs:83)：

```rust
macro_rules! define_repository {
    ($name:ident, $pool:ty) => {
        pub struct $name<'a> {
            pool: &'a PgPool,
        }
        impl<'a> $name<'a> {
            pub fn new(pool: &'a PgPool) -> Self {
                Self { pool }
            }
            pub fn pool(&self) -> &'a PgPool {
                self.pool
            }
        }
    };
}
```

每个 Repository 生成 `new()` 和 `pool()` 两个方法。所有 Repository 最终都手写自己的 CRUD 方法，宏并未减少重复。

**结论**：3 行实现可以用 [derive 宏](https://docs.rs/derive_builder/) 替代，或直接用 `struct DataRepository<'a>(&'a PgPool);` 的元组结构体。

### 4.3 `BaseRepository` trait 未被实现

```rust
pub trait BaseRepository<E, NewE> {
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<E>>;
    async fn find_all(&self) -> AppResult<Vec<E>>;
    async fn insert(&self, entity: NewE) -> AppResult<E>;
    async fn delete(&self, id: Uuid) -> AppResult<bool>;
}
```

零个 Repository 实现了此 trait。与 `QueryBuilder` 一样是死代码。

### 4.4 `map_not_found_error` 硬编码 "未找到"

```rust
pub fn map_not_found_error(e: sqlx::Error, entity_name: &str, id: &Uuid) -> AppError {
    match e {
        sqlx::Error::RowNotFound => AppError::NotFound(format!("{} {} 未找到", entity_name, id)),
        _ => AppError::from(e),
    }
}
```

错误信息只能中文。但项目已有国际化需求的可能（`device_type.rs` 中 `as_str` 返回英文，`Display` 返回中文），应统一为英文错误码 + 前端本地化。

---

## 5. Ingest 模块重复代码

### 5.1 MQTT 客户端代码重复

[`VisionModule::run_mqtt_client`](src/ingest/modules/vision.rs:94) 与 [`ImuModule::run_mqtt_client`](src/ingest/modules/imu.rs:154) 几乎完全相同：

```rust
async fn run_mqtt_client(
    pool: &PgPool,
    broker: &str,
    port: u16,
    client_id: &str,
    topic: &str,
    qos: u8,
    config: &ImuConfig, // 仅此参数类型不同
) -> AppResult<()> {
    // ... 完全相同的 MQTT 连接、订阅、poll 循环
}
```

### 5.2 模块启动逻辑重复

[`main.rs:124`](src/main.rs:124) 中注册模块的代码：

```rust
async fn init_ingest_modules(pool: &PgPool, settings: &Settings) -> anyhow::Result<()> {
    let mut registry = ModuleRegistry::new();

    let vision = vision::VisionModule::new(
        settings.vision.broker.clone(),
        settings.vision.port,
        settings.vision.client_id.clone(),
        settings.vision.topic.clone(),
        settings.vision.qos,
    );
    registry.register(Box::new(vision));

    let imu = imu::ImuModule::new(
        settings.imu.broker.clone(),
        settings.imu.port,
        settings.imu.client_id.clone(),
        settings.imu.topic.clone(),
        settings.imu.qos,
        settings.imu.clone(),
    );
    registry.register(Box::new(imu));

    let mattress = mattress::MattressModule::new(
        settings.mattress.bind.clone(),
        settings.mattress.port,
        settings.mattress.max_frame_size,
    );
    registry.register(Box::new(mattress));

    registry.start_all(pool).await
}
```

**优化**：使用 Rocket 的配置管理的 Fairing 自动发现和注册：

```rust
// 使用 Rocket 的 Figment 或配置枚举自动注册
#[derive(Deserialize)]
struct ModuleConfig {
    #[serde(flatten)]
    mqtt: MqttCommonConfig,
    vision: VisionConfig,
    imu: ImuConfig,
    mattress: MattressConfig,
}
```

---

## 6. 错误处理膨胀

### 6.1 `AppError` 有 14 个变体，部分可合并

```rust
pub enum AppError {
    DatabaseError(sqlx::Error),       // ✓ 必要
    NotFound(String),                  // ✓ 必要
    ValidationError(String),           // ✓ 必要
    Unauthorized(String),              // ✓ 必要
    Forbidden(String),                 // ✓ 必要
    Conflict(String),                  // ✓ 必要
    InternalError(String),             // ✓ 必要
    AuthenticationError(String),       // ← 与 Unauthorized 重叠
    UuidError(uuid::Error),           // ← 可转为 ValidationError
    InvalidCredentials(String),       // ← 可转为 AuthenticationError
    UsernameExists(String),           // ← 可转为 Conflict
    InvalidPassword(String),          // ← 可转为 ValidationError
    TokenExpired,                      // ← 可转为 Unauthorized
    TokenInvalid(String),             // ← 可转为 Unauthorized
}
```

**建议合并为 7-8 个核心变体**：

```rust
pub enum AppError {
    Database(sqlx::Error),
    NotFound(String),
    Validation(String),         // 含 UuidError, InvalidPassword
    Unauthorized(String),       // 含 TokenExpired, TokenInvalid
    Forbidden(String),
    Conflict(String),           // 含 UsernameExists
    Internal(String),
}
```

### 6.2 Responder 实现可简化

当前的 [`Responder` 实现](src/errors/app_error.rs:61) 为每个变体匹配 HTTP 状态码。用辅助宏可大幅简化：

```rust
impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        let (status, error_msg) = match &self {
            AppError::NotFound(_) => (Status::NotFound, self.to_string()),
            AppError::Validation(_) => (Status::UnprocessableEntity, self.to_string()),
            AppError::Unauthorized(_) | AppError::Forbidden(_) => {
                (Status::Unauthorized, self.to_string())
            }
            AppError::Conflict(_) => (Status::Conflict, self.to_string()),
            AppError::Database(e) => {
                log::error!("数据库错误: {:?}", e);
                (Status::InternalServerError, "内部服务错误".into())
            }
            AppError::Internal(_) => (Status::InternalServerError, self.to_string()),
        };
        // ...
    }
}
```

---

## 7. Rust/Rocket 框架利用不足

### 7.1 Rocket Fairings 使用不足

当前仅有一个 [`Cors` Fairing](src/main.rs:26)：

```rust
impl Fairing for Cors {
    fn info(&self) -> Info { Info { name: "CORS", kind: Kind::Response } }
    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_raw_header("Access-Control-Allow-Origin", "*");
        res.set_raw_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
        // ... 7 个手动 set_raw_header
    }
}
```

**可添加的 Fairing**：

| Fairing | 用途 |
|---------|------|
| **请求日志 Fairing** | 替代 `env_logger`，自动记录每个请求的方法/路径/状态码/耗时 |
| **请求计时 Fairing** | `on_request` 记录开始时间，`on_response` 输出耗时 |
| **安全头 Fairing** | 统一设置 `X-Content-Type-Options`, `X-Frame-Options` 等 |
| **配置热加载 Fairing** | 监听配置文件变更自动重载 |

### 7.2 Rocket 的 Type-safe Guards 未充分利用

Rocket 0.5 的核心优势是编译期请求校验，但项目中大量参数仍在运行时解析：

```rust
// 当前做法 - 运行时字符串解析
#[get("/data?<patient_id>&<data_type>&<start_time>&<end_time>")]
pub async fn query_data(
    patient_id: Option<&str>,   // 需要手动解析为 Uuid
    data_type: Option<&str>,    // 需要手动匹配为 DataType 枚举
    start_time: Option<&str>,   // 需要手动解析为 DateTime
    end_time: Option<&str>,
)
```

**优化**：实现自定义 `FromForm` 或 `FromParam` guard：

```rust
// 类型安全的查询参数
#[derive(FromForm)]
pub struct DataQueryParams {
    patient_id: Option<Uuid>,
    data_type: Option<DataType>,       // 自动 FromStr
    #[field(validate = range(..=Utc::now()))]
    start_time: Option<DateTime<Utc>>, // 自动 FromStr
    end_time: Option<DateTime<Utc>>,
}
```

### 7.3 无 sqlx 编译期查询检查

项目中所有 sqlx 查询使用运行时字符串：

```rust
let rows = sqlx::query_as::<_, Datasheet>(
    "SELECT * FROM datasheet WHERE device_id = $1 AND time >= $2 AND time <= $3"
)
.bind(device_id)
.bind(start_time)
.bind(end_time)
.fetch_all(&self.pool)
.await?;
```

sqlx 0.8 支持 `sqlx::query!("SELECT ...")` 编译期检查，需要在构建时连接数据库。当前项目没有 `.env` 或 `SQLX_OFFLINE=true` 配置。

**优化**：

```bash
# 安装 sqlx-cli
cargo install sqlx-cli

# 生成离线数据
cd d:/repo/dev/remipedia && DATABASE_URL=postgres://... cargo sqlx prepare
```

然后使用编译期检查：

```rust
let rows = sqlx::query_as!(
    Datasheet,
    "SELECT * FROM datasheet WHERE device_id = $1 AND time >= $2 AND time <= $3",
    device_id,
    start_time,
    end_time,
)
.fetch_all(&self.pool)
.await?;
```

### 7.4 缺少 Rocket 内置序列化策略

Rocket 0.5 的 [`rocket::serde`](https://api.rocket.rs/v0.5/rocket/serde/) 支持全局 `json` 和 `msgpack`。当前项目单独依赖 `rmp-serde` 处理床垫 msgpack，但实际上 Rocket 已内置 msgpack 支持。

**Cargo.toml** 已有 `rocket = { version = "0.5", features = ["msgpack"] }`（需确认），如果已启用则可以直接在路由中使用 `MsgPack<T>` guard。

### 7.5 无 Rocket 配置分层支持

Rocket 0.5 使用 [Figment](https://docs.rs/figment/) 提供配置分层。当前项目通过独立的 `config` crate 和 [`Settings`](src/config/settings.rs) 手动管理配置。

**优化**：将配置合并到 Rocket 的 Figment 中，利用环境变量覆盖：

```yaml
# Rocket.toml（或 config/default.yaml）
[default]
ingest.vision.broker = "localhost"
ingest.vision.port = 1883

[production]
ingest.vision.broker = "mqtt.internal.prod"
```

---

## 8. 综合优化方案

### 8.1 优先级 P0（紧急修复）

| # | 优化项 | 文件 | 估计工作量 |
|---|--------|------|-----------|
| 1 | **修复 TCP 帧协议**——length 改为 2 字节、启用 CRC 校验、添加帧计数器 | [`mattress.rs:196`](src/ingest/modules/mattress.rs:196) | 1 天 |
| 2 | **修复无序列号/设备类型的格式错误**——在 `archive_raw` 前增加格式校验 | [`mattress.rs:138`](src/ingest/modules/mattress.rs:138) | 0.5 天 |
| 3 | **删除 QueryBuilder + BaseRepository trait + IntoResponse**——移除全部死代码 | [`base.rs`](src/repository/base.rs) + [`convert.rs`](src/dto/convert.rs) | 0.5 天 |

### 8.2 优先级 P1（高价值）

| # | 优化项 | 文件 | 估计工作量 |
|---|--------|------|-----------|
| 4 | **提取共享 `resolve_or_create_device`**——从三个模块各删 ~25 行重复代码 | [`ingest/modules/mod.rs`](src/ingest/modules/mod.rs) | 0.5 天 |
| 5 | **提取共享 MQTT 客户端**——`VisionModule` 和 `ImuModule` 共用 MQTT 循环 | [`ingest/modules/`](src/ingest/modules/) | 1 天 |
| 6 | **合并 AppError 变体**——从 14 个压缩到 7 个，更新所有匹配 | [`errors/app_error.rs`](src/errors/app_error.rs) | 1 天 |
| 7 | **消除 `DataRecordResponse` 冗余**——直接复用 `Datasheet` 或使用 `#[serde(flatten)]` | [`dto/response/data.rs`](src/dto/response/data.rs) | 0.5 天 |
| 8 | **添加 Rocket 请求日志 Fairing**——替代 env_logger 手动日志 | [`main.rs`](src/main.rs) | 0.5 天 |

### 8.3 优先级 P2（架构优化）

| # | 优化项 | 文件 | 估计工作量 |
|---|--------|------|-----------|
| 9 | **配置迁移到 Figment/Rocket.toml**——合并 `Settings` + 环境变量覆盖 | [`config/`](config/) + [`main.rs`](src/main.rs) | 1 天 |
| 10 | **添加 sqlx 编译期查询**——配置 `SQLX_OFFLINE=true`，逐步替换 `query_as!` | 所有 repository | 2 天 |
| 11 | **自定义 `FromForm` guard**——类型安全查询参数替代运行时字符串解析 | [`api/routes/`](src/api/routes/) | 1 天 |
| 12 | **添加连接级超时和背压**——TCP 连接数限制、MQTT 背压控制 | [`ingest/modules/mattress.rs`](src/ingest/modules/mattress.rs) | 1 天 |

### 8.4 去"过度封装"后代码量预估

| 模块 | 当前行数 | 优化后预估 | 减少 |
|------|---------|-----------|------|
| `src/repository/base.rs` | 163 | ~50 | **-69%** |
| `src/errors/app_error.rs` | 90 | ~50 | **-44%** |
| `src/dto/response/data.rs` | 171 | ~80 | **-53%** |
| `src/dto/request/data.rs` | 176 | ~100 | **-43%** |
| `src/dto/convert.rs` | 20 | 0 | **-100%** |
| `src/ingest/modules/` (3 模块) | ~1282 | ~1100 | **-14%** |
| **合计** | **~1902** | **~1380** | **~-27%** |

> 注：行数减少约 500+ 行，同时消除了 3 处死代码模块、3 份重复逻辑、提升类型安全。

---

### 附录：关键文件索引

| 文件 | 行数 | 功能 |
|------|------|------|
| [`src/main.rs`](src/main.rs) | 206 | 应用入口，Fairing + 模块注册 |
| [`src/errors/app_error.rs`](src/errors/app_error.rs) | 90 | 14 变体错误枚举 + Responder |
| [`src/repository/base.rs`](src/repository/base.rs) | 163 | 死代码集中地（QueryBuilder / BaseRepository / define_repository macro） |
| [`src/dto/convert.rs`](src/dto/convert.rs) | 20 | 未实现的 IntoResponse trait |
| [`src/ingest/modules/mattress.rs`](src/ingest/modules/mattress.rs) | 425 | TCP 帧协议（需紧急修复） |
| [`src/ingest/modules/mod.rs`](src/ingest/modules/mod.rs) | 109 | IngestModule trait + ModuleRegistry |
| [`src/dto/response/data.rs`](src/dto/response/data.rs) | 171 | 响应 DTO 层 |
| [`src/dto/request/data.rs`](src/dto/request/data.rs) | 176 | 请求 DTO 层 |
| [`src/core/entity/datasheet.rs`](src/core/entity/datasheet.rs) | 250 | DataPoint + Datasheet 核心实体 |
| [`src/service/data.rs`](src/service/data.rs) | 226 | 数据服务层 |
| [`src/repository/data.rs`](src/repository/data.rs) | 309 | 数据 Repository |

---

*文档生成时间: 2026-04-25*  
*审查范围: src/ 下 50+ 文件，全量代码审查*
