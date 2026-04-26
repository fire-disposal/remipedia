# IoT 健康数据平台后端 — 优化与重设计方向

> **宏观目标**: 一套宽松的统合数据结构内，最大化便于多种设备的接入和模块化开发。
>
> 本文档基于全量代码审查、已有优化分析（[`ARCHITECTURE_OPTIMIZATION.md`](docs/ARCHITECTURE_OPTIMIZATION.md)、[`PHASE2_OPTIMIZATION.md`](docs/PHASE2_OPTIMIZATION.md)、[`INGEST_LAYER_AUDIT.md`](docs/INGEST_LAYER_AUDIT.md)）及 Rust/Rocket 生态最佳实践综合得出。

---

## 目录

1. [核心战略：从"设备驱动"到"数据驱动"](#1-核心战略从设备驱动到数据驱动)
2. [统一数据结构演进方案](#2-统一数据结构演进方案)
3. [Ingest 层完全插件化](#3-ingest-层完全插件化)
4. [Rust 包生态与 Rocket 框架理想实现](#4-rust-包生态与-rocket-框架理想实现)
5. [DTO/Service 层激进精简](#5-dtoservice-层激进精简)
6. [Repository 层统一与死代码清除](#6-repository-层统一与死代码清除)
7. [错误处理合理化](#7-错误处理合理化)
8. [实施路线图与优先级](#8-实施路线图与优先级)
9. [附录：Cargo crate 选用建议](#9-附录cargo-crate-选用建议)

---

## 1. 核心战略：从"设备驱动"到"数据驱动"

### 1.1 当前困境

当前架构本质上是**设备驱动型**——每个新设备类型都需要：
1. 新增一个 `ingest/modules/xxx.rs` 模块（300-500 行）
2. 在 `config/settings.rs` 添加配置结构体
3. 在 `main.rs` 添加模块构造和注册代码
4. 如果出现新协议类型（非 MQTT/TCP），需要重新实现连接管理

这导致从"决定接入新设备"到"上线"的路径很长且重复。

### 1.2 目标：数据驱动型架构

```
设备驱动（当前）                             数据驱动（目标）
┌──────────────┐                            ┌──────────────┐
│ VisionModule │──解析→ DataPoint           │ 统一摄取层   │──原始载荷→ 协议路由器
├──────────────┤                            │              │──→ DataPoint（规范化）
│  ImuModule   │──解析→ DataPoint           │  MQTT/TCP/   │──→ RawData（归档）
├──────────────┤                            │  HTTP/WS     │
│ MattressModule│──解析→ DataPoint           └──────────────┘
└──────────────┘                                    │
        │                                            ▼
        ▼                                    ┌──────────────────┐
   ┌─────────────┐                           │  Unified Datasheet │
   │  Datasheet  │                           │  metric/value +     │
   │  (统一表)    │                           │  payload(jsonb)     │
   └─────────────┘                           └──────────────────┘
```

**核心转变**：不再为每种设备类型编写独立的连接→解析→存储管线，而是：
- 统一的摄取入口（MQTT/TCP/HTTP/WS）
- 原始载荷完整归档（RawData）
- 协议路由器根据 `device_type` 分发到对应的数据提取器
- 提取器只负责将原始载荷映射到统一的 `Datasheet` 结构

---

## 2. 统一数据结构演进方案

### 2.1 当前 `Datasheet` 模型分析

当前的 [`Datasheet`](src/core/entity/datasheet.rs:105) 模型已经是一个合理的统一结构：

```rust
pub struct Datasheet {
    pub time: DateTime<Utc>,
    pub device_id: Uuid,
    pub patient_id: Option<Uuid>,
    pub data_type: String,        // 如 "heart_rate", "fall_detection"
    pub data_category: String,    // "metric" | "event"
    pub value: f64,               // 数值主值
    pub text_value: Option<String>, // 文本补充
    pub unit: Option<String>,
    pub severity: Option<String>, // "info" | "warning" | "alert"
    pub event_status: Option<String>,
    pub device_name: Option<String>,
}
```

这已经是一个**"宽松统合"**结构，但存在以下不足：

| 问题 | 说明 | 影响 |
|------|------|------|
| 字段固定 | 所有设备类型共用相同字段 | 无法表达设备特有属性（如床垫的呼吸率 vs 视觉的置信度） |
| `value: f64` 强制数值 | 某些事件没有合理数值（如 "visitor_detected"） | 只能用 0.0 占位 |
| `data_type: String` 松散 | 无类型约束，拼写错误无法检测 | 查询时可能遗漏 |
| 缺少关联分组 | 同一设备同一时刻的多个指标（心跳+呼吸+体温）无关联 ID | 难以聚合分析 |
| `data_category: String` 可枚举但未约束 | 未使用 `DataCategory` 枚举 | 运行时才可能发现非法值 |

### 2.2 演进方案 V1：轻量增强（推荐优先实施）

在保持向后兼容的前提下，对 `Datasheet` 做最小增强：

```rust
// src/core/entity/datasheet.rs

/// 数据点 — IoT 平台统一数据结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Datasheet {
    // === 核心标识（不变） ===
    pub time: DateTime<Utc>,
    pub device_id: Uuid,
    pub patient_id: Option<Uuid>,

    // === 类型系统强化 ===
    pub data_type: DataType,           // 从 String 改为 DataType 值对象
    pub data_category: DataCategory,   // 已有枚举，直接使用

    // === 数值体系扩展 ===
    pub metric_value: Option<f64>,     // 从 value: f64 改为 Option
    pub text_value: Option<String>,    // 保留

    // === 设备特定载荷 ===
    pub payload: Option<serde_json::Value>, // ★ 新增：设备专有数据

    // === 元数据 ===
    pub unit: Option<String>,
    pub severity: Option<Severity>,    // 使用枚举而非 String
    pub event_status: Option<EventStatus>,
    pub device_name: Option<String>,

    // === 批次关联 ===
    pub batch_id: Option<Uuid>,        // ★ 新增：同一批次数据关联
}
```

**关键变更解释**：

| 变更 | 原因 |
|------|------|
| `value: f64` → `metric_value: Option<f64>` | 事件类数据没有合理数值，允许 `null` |
| `data_type: String` → `DataType` | 编译期类型检查，非法类型无法编译通过 |
| `severity: String` → `Severity` | 同上 |
| `+ payload: Option<Json>` | **核心**：设备专有数据放入 JSONB，查询时可按需提取，不破坏统一结构 |
| `+ batch_id: Option<Uuid>` | 关联同一设备同一时刻的多项指标 |

#### `DataType` 值对象设计

```rust
// src/core/value_object/data_type.rs

/// 数据类型 — 编译期安全的统一类型标识
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum DataType {
    // === 生理指标 ===
    HeartRate,
    BreathRate,
    BodyTemperature,
    BloodPressure,
    BloodOxygen,
    Weight,
    // === 事件 ===
    FallDetection,
    WanderDetection,
    VisitorDetection,
    PressureUlcerRisk,
    ApneaEvent,
    OnBed,
    OffBed,
    // === 扩展 ===
    Custom(String),  // 允许自定义扩展
}
```

### 2.3 演进方案 V2：完全动态（长期方向）

对于追求极致灵活性的场景，可将核心结构简化为**键值对模式**：

```rust
/// 极致灵活的 IoT 数据结构
pub struct DatasheetV2 {
    pub time: DateTime<Utc>,
    pub device_id: Uuid,
    pub patient_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,

    // 设备类型（驱动解析策略）
    pub device_type: DeviceType,

    // 核心指标（预定义索引字段，用于高效查询）
    pub metrics: Vec<Metric>,

    // 完整原始数据（JSONB，保留设备原始结构）
    pub payload: serde_json::Value,
}

pub struct Metric {
    pub key: String,     // "heart_rate" / "temperature" / "fall"
    pub value: f64,      // 数值（如需文本用 payload）
    pub unit: Option<String>,
}
```

**优势**：无限灵活，新增设备类型无需修改 DB schema。
**劣势**：查询复杂指标需 JSONB 路径表达式，PostgreSQL 索引效率低于列存储。

**建议**：V2 作为备选方案，当前采用 V1 轻量增强。

---

## 3. Ingest 层完全插件化

### 3.1 当前痛点总结

来源：[`INGEST_LAYER_AUDIT.md`](docs/INGEST_LAYER_AUDIT.md) + [`ARCHITECTURE_OPTIMIZATION.md` §5](docs/ARCHITECTURE_OPTIMIZATION.md:395)

| 问题 | 位置 | 影响 |
|------|------|------|
| MQTT 客户端代码重复（Vision vs Imu 几乎一致） | [`vision.rs:94`](src/ingest/modules/vision.rs:94)、[`imu.rs:154`](src/ingest/modules/imu.rs:154) | 2 份 ~60 行重复 |
| `resolve_or_create_device` 已共享但未用 `DeviceType` | [`mod.rs:75`](src/ingest/modules/mod.rs:75) | 类型不安全 |
| blanket impl 模板化 | [`mod.rs:107-149`](src/ingest/modules/mod.rs:107) | 3 个模块 x 3 方法 |
| 新设备接入门槛高 | 全过程需 300-500 行 | 扩展性差 |
| TCP 连接无限制 | [`mattress.rs:89`](src/ingest/modules/mattress.rs:89) | 无背压，可被连接耗尽 |
| 错误吞没 | 多处 `log::error` 后 `continue` | 关键数据丢失不可恢复 |

### 3.2 目标：插件化架构

```
                            ┌─────────────────────┐
                            │   ModuleRegistry     │
                            │  (启动/停止/监控)     │
                            └──────┬──────────────┘
                                   │ 注册
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
     ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
     │  VisionPlugin   │  │   ImuPlugin     │  │ MattressPlugin  │
     │  (MQTT handler) │  │  (MQTT handler)  │  │  (TCP server)   │
     └─────────────────┘  └─────────────────┘  └─────────────────┘
              │                    │                    │
              └────────────────────┼────────────────────┘
                                   ▼
                    ┌──────────────────────────┐
                    │  共享基础设施              │
                    │  - MqttRunner (通用 MQTT) │
                    │  - TcpRunner (通用 TCP)   │
                    │  - resolve_or_create_device│
                    │  - raw_data archiver      │
                    └──────────────────────────┘
```

### 3.3 共享 MQTT 运行器（可立即实施）

```rust
// src/ingest/modules/mqtt_runner.rs — 现有文件增强

/// MQTT 消息处理器签名
pub type MessageHandler = Arc<dyn Fn(rumqttc::Packet, PgPool) -> AppResult<()> + Send + Sync>;

/// 通用 MQTT 运行器
pub async fn run_mqtt_client(
    pool: PgPool,
    params: MqttParams,
    handler: MessageHandler,
) -> AppResult<()> {
    let (client, mut eventloop) = connect_and_subscribe(&params).await?;

    loop {
        match eventloop.poll().await {
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                if let Err(e) = handler(publish, pool.clone()).await {
                    log::error!("MQTT handler error: {:?}", e);
                    // 可选的：将失败消息写入死信队列
                }
            }
            Ok(_) => {}
            Err(rumqttc::ConnectionError::Io(e)) => {
                log::warn!("MQTT 连接断开，5秒后重连: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                // 重连逻辑
                let (new_client, new_eventloop) = connect_and_subscribe(&params).await?;
                client = new_client;
                eventloop = new_eventloop;
            }
            Err(e) => {
                log::error!("MQTT 错误: {:?}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
```

### 3.4 新设备插件接入模板（目标：< 80 行）

```rust
// src/ingest/modules/temperature.rs — 新设备接入示例

use crate::ingest::modules::mqtt_runner::{self, MqttParams, MessageHandler};

pub struct TemperaturePlugin {
    params: MqttParams,
}

impl IngestModule for TemperaturePlugin {
    fn name(&self) -> &'static str { "temperature" }

    async fn start(&self, pool: PgPool, registry: &DeviceTypeRegistry) -> AppResult<()> {
        let pool = pool.clone();
        let device_type = registry.resolve("temperature_sensor")?;

        let handler: MessageHandler = Arc::new(move |packet, pool| {
            let payload: serde_json::Value = serde_json::from_slice(&packet.payload)?;
            let device_id = resolve_or_create_device(&pool, &payload["device_id"], device_type).await?;

            let datapoint = DataPoint::metric("temperature", payload["value"].as_f64().unwrap_or(0.0))
                .with_unit("celsius")
                .with_device(device_id)
                .with_payload(payload);  // 原始数据存入 payload

            DataService::ingest(&pool, datapoint).await
        });

        mqtt_runner::run_mqtt_client(pool, self.params.clone(), handler).await
    }
}
```

### 3.5 插件注册自动化

```rust
// src/main.rs — 使用 Rocket 配置 + 工厂模式

/// 插件工厂：根据配置自动构造对应插件
pub fn create_plugins(settings: &Settings) -> Vec<Box<dyn IngestModule>> {
    let mut plugins: Vec<Box<dyn IngestModule>> = Vec::new();

    for (name, config) in &settings.ingest.modules {
        match name.as_str() {
            "vision" => plugins.push(Box::new(VisionPlugin::from_config(config))),
            "imu" => plugins.push(Box::new(ImuPlugin::from_config(config))),
            "mattress" => plugins.push(Box::new(MattressPlugin::from_config(config))),
            _ => log::warn!("未知的 Ingest 模块: {}", name),
        }
    }
    plugins
}
```

**配置文件简化为**：

```yaml
# config/default.yaml
ingest:
  modules:
    vision:
      broker: "localhost:1883"
      topic: "vision/events"
    imu:
      broker: "localhost:1883"
      topic: "imu/data"
    mattress:
      bind: "0.0.0.0"
      port: 9500
```

**效果**：新增设备只需：
1. 新建 `src/ingest/modules/temperature.rs`（~80 行）
2. 在 `create_plugins` 中添加一行 `match`
3. 在 `config.yaml` 中添加配置

---

## 4. Rust 包生态与 Rocket 框架理想实现

### 4.1 Rocket 框架利用不足总览

| Rocket 特性 | 当前使用 | 理想使用 | 收益 |
|-------------|---------|---------|------|
| [Fairing](https://api.rocket.rs/v0.5/rocket/trait.Fairing.html) | 仅 CORS | 请求日志、计时、安全头、配置热加载 | 减少模板代码，集中化横切关注点 |
| [FromForm](https://api.rocket.rs/v0.5/rocket/trait.FromForm.html) | 未使用 | 类型安全查询参数 | 消除运行时字符串解析 |
| [FromParam](https://api.rocket.rs/v0.5/rocket/trait.FromParam.html) | 未使用 | 自定义路径参数解析 | 更简洁的路由定义 |
| [Figment](https://docs.rs/figment/) 配置 | 独立 `config` crate | Rocket 内置配置分层 | 减少外部依赖，环境变量自动覆盖 |
| [MsgPack](https://api.rocket.rs/v0.5/rocket/serde/msgpack/index.html) | 外部 `rmp-serde` | Rocket 内置 `MsgPack` | 减少依赖，统一序列化策略 |
| [State](https://api.rocket.rs/v0.5/rocket/struct.State.html) | 使用 `&State<Pool>` | 更广泛的 State 使用 | 减少手动传参 |
| sqlx [query!](https://docs.rs/sqlx/0.8/sqlx/macro.query.html) 编译期检查 | 未使用 | `sqlx::query!("SELECT ...")` | 编译期 SQL 校验 |

### 4.2 Fairing 优化建议

```rust
// src/main.rs — 请求日志 Fairing（替换 env_logger）

pub struct RequestLogger;

impl Fairing for RequestLogger {
    fn info(&self) -> Info {
        Info { name: "Request Logger", kind: Kind::Request | Kind::Response }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        // 记录请求开始时间到本地上下文
        request.local_cache(|| std::time::Instant::now());
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let duration = request.local_cache(|| std::time::Instant::now()).elapsed();
        log::info!(
            "{} {} {} {:?}",
            request.method(),
            request.uri(),
            response.status(),
            duration,
        );
    }
}
```

**可以添加的 Fairing**：

| Fairing | 实现优先级 | 代码量 | 说明 |
|---------|-----------|--------|------|
| `RequestLogger` | P1 | ~20 行 | 替代 `env_logger` 手动日志，全自动 |
| `SecurityHeaders` | P2 | ~15 行 | 统一添加 `X-Content-Type-Options`、`Strict-Transport-Security` 等 |
| `PerformanceTimer` | P1 | ~20 行 | 记录每个请求的耗时到 metrics |
| `ConfigReloader` | P3 | ~50 行 | 监听配置文件变更自动重载 |

### 4.3 FromForm 类型安全查询参数

**当前代码**（[`src/api/routes/data.rs:80`](src/api/routes/data.rs:80)）：

```rust
#[get("/data?<patient_id>&<data_type>&<start_time>&<end_time>")]
pub async fn query_data(
    patient_id: Option<&str>,   // 需要手动解析为 Uuid
    data_type: Option<&str>,    // 需要手动匹配
    start_time: Option<&str>,   // 需要手动解析
    end_time: Option<&str>,
) -> Json<DataQueryResponse> {
    let patient_id = patient_id.and_then(|s| Uuid::parse_str(s).ok());
    let data_type = data_type.map(|s| s.to_string());
    // ... 每个参数手动解析
}
```

**优化后**：

```rust
use rocket::form::FromForm;

#[derive(Debug, FromForm)]
pub struct DataQueryParams {
    #[field(name = "patient_id")]
    pub patient_id: Option<Uuid>,           // Rocket 自动从 &str 解析 Uuid
    pub data_type: Option<String>,
    pub start_time: Option<DateTime<Utc>>,  // Rocket 自动从 &str 解析 DateTime
    pub end_time: Option<DateTime<Utc>>,
    #[field(default = 1u32)]
    pub page: u32,
    #[field(default = 20u32)]
    pub page_size: u32,
}

#[get("/data?<params>")]
pub async fn query_data(params: DataQueryParams) -> Json<DataQueryResponse> {
    // patient_id 已经是 Option<Uuid>，无需手动解析
    // start_time 已经是 Option<DateTime<Utc>>
    // 由 Rocket 完成所有类型转换
}
```

**效果**：
- 消灭 ~5 行手动解析代码/每路由
- 400 Bad Request 自动返回（非法 Uuid 格式等）
- 编译期类型保证

### 4.4 利用已有 Cargo crate 简化代码

| 当前做法 | 推荐 crate | 简化效果 | 代码量减少 |
|---------|-----------|---------|-----------|
| 手写 `new()` + `pool()` 样板 | `define_repository!` 宏或元组结构体 | 去除宏，改用 `struct Repo<'a>(&'a PgPool);` | ~30 行 |
| 手写 `FromRow` derive（已用） | sqlx 内置 | 已使用 ✓ | — |
| `rocket::serde::json::Json`（已用） | rocket 内置 | 已使用 ✓ | — |
| 独立 `config` crate 管理配置 | Rocket Figment（内置） | 合并配置管理 | -71 行（`settings.rs`） |
| `env_logger` 手动日志 | Rocket Fairing | 零配置请求日志 | ~20 行 |
| `rmp-serde` 独立使用 | Rocket `MsgPack` feature | 统一序列化 | 无硬依赖减少 |
| `validator` + 手写校验 | Rocket `#[field(validate)]` | 框架内置校验 | 每个字段 1-2 行 |
| 手写分页逻辑 | 通用 `Pagination` 辅助函数 | 统一分页处理 | ~15 行/查询路由 |
| `anyhow` + `thiserror` | 仅保留 `thiserror` | 减少间接依赖 | 视使用情况 |
| 手写连接池管理 | Rocket `rocket::State<Pool>` | 框架管理生命周期 | 已使用 ✓ |
| 手写配置环境变量覆盖 | Rocket Figment `{profile}.{key}` | 标准化覆盖机制 | 减少 ~30 行手动解析 |

### 4.5 配置管理合并到 Rocket Figment

**当前**：独立的 [`src/config/settings.rs`](src/config/settings.rs)（71 行）+ `config` crate + 手动环境变量映射。

**理想实现**：

```toml
# Rocket.toml（或保持 YAML 通过 Figment 加载）
[default]
ingest_modules = ["vision", "imu", "mattress"]

[default.ingest.vision]
broker = "localhost"
port = 1883
topic = "vision/events"

[production]
ingest_modules = ["vision", "imu", "mattress", "temperature"]

[production.ingest.vision]
broker = "mqtt.internal.prod"
```

环境变量覆盖（Figment 原生支持）：

```bash
APP_INGEST_VISION_BROKER=mqtt.internal.prod cargo run
```

---

## 5. DTO/Service 层激进精简

### 5.1 当前 DTO 层全景

```
src/dto/
├── request/          # 请求 DTO（保留，必不可少）
│   ├── auth.rs       # LoginRequest, RegisterRequest 等
│   ├── data.rs       # DataReportRequest, DataQuery, AlertQuery, RawDataQuery, Binding requests
│   ├── device.rs     # DeviceQuery, UpdateDeviceRequest
│   ├── patient.rs    # CreatePatientRequest, UpdatePatientRequest, PatientQuery
│   └── user.rs       # CreateUserRequest, UpdateUserRequest
└── response/         # 响应 DTO（可大幅精简）
    ├── admin.rs      # RoleResponse, ModuleResponse, AuditLogResponse（可删除，Entity 替代）
    ├── auth.rs       # LoginResponse, TokenResponse
    ├── data.rs       # DataRecordResponse（可删除）, DataReportResponse, DataQueryResponse, AlertStatsResponse
                      # BindingResponse（可删除）, BindingListResponse, RawDataRecordResponse, RawDataDetailResponse
    ├── device.rs     # DeviceResponse（保留）
    ├── patient.rs    # PatientResponse（可删除）, PatientDetailResponse（保留）, PatientProfileResponse（保留）
    └── user.rs       # UserResponse（保留）
```

### 5.2 Entity-as-Response 策略

依据 [`PHASE2_OPTIMIZATION.md §2.1`](docs/PHASE2_OPTIMIZATION.md:40) 的分析：

**可直接删除的响应 DTO（Entity 加 `ToSchema` 后直接使用）**：

| DTO 结构体 | 对应 Entity | 字段差异 | 操作 |
|-----------|------------|---------|------|
| `RoleResponse` | [`Role`](src/core/entity/role.rs:9) | 0 | 删除，Entity 加 `ToSchema` |
| `ModuleResponse` | [`Module`](src/core/entity/module.rs:5)（值对象） | 0 | 删除，值对象加 `ToSchema` |
| `AuditLogResponse` | [`AuditLog`](src/core/entity/audit_log.rs:10) | 0 | 删除，Entity 加 `ToSchema` |
| `PatientResponse` | [`Patient`](src/core/entity/patient.rs:9) | 0 | 删除，Entity 加 `ToSchema` |
| `BindingResponse` | [`Binding`](src/core/entity/binding.rs:9) | 0 | 删除，Entity 加 `ToSchema` |
| `DataRecordResponse` | [`Datasheet`](src/core/entity/datasheet.rs:105) | 0 | 删除，Entity 加 `ToSchema` |

**需保留的 DTO**（因字段结构不同）：

| DTO | 保留原因 |
|-----|---------|
| `UserResponse` | 含 `role_name: String`（关联查询） |
| `DeviceResponse` | 含 `current_binding: Option<BindingInfo>` |
| `PatientDetailResponse` | 含 `Patient` + `PatientProfile` 两层 |
| `PatientProfileResponse` | 独立于实体结构 |
| `RawDataRecordResponse` | 含运行时计算的 `payload_preview` |
| `RawDataDetailResponse` | 含 base64/hex 编码 |

**效果**：6 个 DTO 结构体 + 6 个 `From<T>` 实现完全消失，减少约 **150 行**。

### 5.3 删除 `IntoResponse` + `ServiceConverter`

依据 [`ARCHITECTURE_OPTIMIZATION.md §3.3`](docs/ARCHITECTURE_OPTIMIZATION.md:273)：

- **`dto/convert.rs`**（~20 行）：`IntoResponse` trait 无人实现 → **整个文件删除**
- **`service/converter.rs`**（~56 行）：`ServiceConverter` 只有 `get_role_name` 一个方法，仅在 `UserService` 调用一次 → **整个文件删除**
- **`User::to_response()`**（[`user.rs:39`](src/core/entity/user.rs:39)）：**严重违反分层**（Core 层依赖 Repository）→ 内联到 `UserService::get_by_id`

### 5.4 Service 层 CRUD 样板去重

**当前问题**：7 个 Service 都有重复的 `new()` + 存在检查 + 分页逻辑。

**方案**：在 [`src/service/mod.rs`](src/service/mod.rs) 添加极简辅助函数：

```rust
/// 实体存在检查
pub fn ensure_found<T>(entity: Option<T>, label: &str, id: &Uuid) -> AppResult<T> {
    entity.ok_or_else(|| AppError::NotFound(format!("{} {} not found", label, id)))
}

/// 简单分页（替代每个 Service 手写）
pub struct PageResult<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}
```

**效果**：每个 Service 减少 ~5-10 行 `match` 样板，总计约 **-40 行**。

### 5.5 总体 DTO/Service 精简预期

| 文件 | 当前行数 | 优化后 | 净变化 |
|------|---------|--------|--------|
| `dto/response/admin.rs` | 121 | ~80 | **-41** |
| `dto/response/data.rs` | 143 | ~110 | **-33** |
| `dto/response/patient.rs` | 86 | ~65 | **-21** |
| `dto/convert.rs` | 20 | 0 | **-20** |
| `service/converter.rs` | 56 | 0 | **-56** |
| `service/mod.rs` | 18 | ~30 | **+12** |
| 各 Service 去重 | 多处 | — | **-40** |
| **合计** | **~444** | **~285** | **~-199 行 (-45%)** |

---

## 6. Repository 层统一与死代码清除

### 6.1 立即删除的死代码

依据 [`ARCHITECTURE_OPTIMIZATION.md §4`](docs/ARCHITECTURE_OPTIMIZATION.md:308)：

| 死代码 | 文件 | 行数 | 问题 |
|--------|------|------|------|
| [`QueryBuilder`](src/repository/base.rs:113) | `base.rs` | ~50 行 | 类型不安全（`Box<dyn Any>`），无法传给 sqlx，无人使用 |
| [`BaseRepository` trait](src/repository/base.rs:150) | `base.rs` | ~15 行 | 零个 Repository 实现 |
| [`define_repository!` 宏](src/repository/base.rs:83) | `base.rs` | ~15 行 | 只生成 `new()` + `pool()`，3 行可替代 |
| [`IntoResponse` trait](src/dto/convert.rs:11) | `convert.rs` | ~20 行 | 无人实现 |

**共计删除约 ~100 行死代码**。

### 6.2 Repository 错误模式统一

**当前**：部分 Repository 返回 `AppResult<T>` 并内部 `map_not_found_error`，部分返回 `AppResult<Option<T>>`。

**统一方案**：

```rust
// 统一规则：
// 1. Repository 方法返回 AppResult<Option<T>>（允许不存在是合法结果）
// 2. Service 层使用 ensure_found() 决定是否转为 NotFound 错误
// 3. 仅当"不存在=异常"时（如 find_by_id 后必须有权限校验），才在 Repository 抛出 NotFound

// ✅ 推荐模式
impl DataRepository {
    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Option<Datasheet>> {
        sqlx::query_as!(Datasheet, "SELECT * FROM datasheet WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::Database)
    }
}

// Service 层决定是否报错
let datasheet = data_repo.find_by_id(&id).await?
    .ok_or_else(|| AppError::NotFound(format!("Datasheet {} not found", id)))?;
```

**效果**：统一错误模式，减少 `map_not_found_error` 调用，错误信息英文（适合国际化）。

### 6.3 Repository 结构体简化

```rust
// 当前（宏生成或手写）
pub struct DataRepository<'a> { pool: &'a PgPool }

impl<'a> DataRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self { Self { pool } }
    pub fn pool(&self) -> &'a PgPool { self.pool }
}

// 简化后
pub struct DataRepository<'a>(pub &'a PgPool);
// 直接使用 repo.0.execute(...) 或通过 Deref
```

**效果**：每个 Repository 减少 5 行，11 个 Repository 共 **-55 行**。

---

## 7. 错误处理合理化

### 7.1 AppError 变体合并

依据 [`ARCHITECTURE_OPTIMIZATION.md §6`](docs/ARCHITECTURE_OPTIMIZATION.md:469)：

**当前 7+7=14 变体 → 优化为 7 变体**：

```rust
// src/errors/app_error.rs

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[source] sqlx::Error),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Validation(String),           // 合并 UuidError, InvalidPassword, ValidationError

    #[error("{0}")]
    Unauthorized(String),         // 合并 TokenExpired, TokenInvalid, AuthenticationError

    #[error("{0}")]
    Forbidden(String),

    #[error("{0}")]
    Conflict(String),             // 合并 UsernameExists

    #[error("{0}")]
    Internal(String),
}
```

**合并后影响**：

| 旧变体 | 新变体 | 原因 |
|--------|--------|------|
| `ValidationError` | `Validation` | 重命名简化 |
| `UuidError` | `Validation("Invalid UUID: ...")` | UUID 解析失败本质是验证错误 |
| `AuthenticationError` | `Unauthorized` | 语义重叠，HTTP 统一用 401 |
| `InvalidCredentials` | `Unauthorized("Invalid credentials")` | 同上 |
| `UsernameExists` | `Conflict("Username already exists")` | 语义是冲突 |
| `InvalidPassword` | `Validation("Invalid password format")` | 验证错误 |
| `TokenExpired` | `Unauthorized("Token expired")` | 授权失败 |
| `TokenInvalid` | `Unauthorized("Token invalid")` | 授权失败 |

### 7.2 Responder 实现简化

```rust
impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        let (status, body) = self.to_response_parts();
        Response::build_from(Json(json!({ "error": body })))
            .status(status)
            .ok()
    }
}

impl AppError {
    fn to_response_parts(&self) -> (Status, String) {
        match self {
            AppError::NotFound(msg) => (Status::NotFound, msg.clone()),
            AppError::Validation(msg) => (Status::UnprocessableEntity, msg.clone()),
            AppError::Unauthorized(msg) => (Status::Unauthorized, msg.clone()),
            AppError::Forbidden(msg) => (Status::Forbidden, msg.clone()),
            AppError::Conflict(msg) => (Status::Conflict, msg.clone()),
            AppError::Database(e) => {
                log::error!("{:?}", e);
                (Status::InternalServerError, "Internal server error".into())
            }
            AppError::Internal(msg) => (Status::InternalServerError, msg.clone()),
        }
    }
}
```

**效果**：错误处理代码从 ~90 行减少到 ~50 行，且每个 HTTP 状态码映射清晰。

---

## 8. 实施路线图与优先级

### 8.1 优先级定义

| 优先级 | 含义 | 预期收益 |
|--------|------|---------|
| **P0** | 阻塞性缺陷，必须先修复 | 消除数据丢失风险 |
| **P1** | 高价值快速 wins | 大量减少样板代码 |
| **P2** | 架构优化，需较多改动 | 长期可维护性提升 |
| **P3** | 框架特性深度利用 | 性能/安全最佳实践 |

### 8.2 阶段一：紧急修复 + 死代码清除（预计 2-3 天）

| # | 任务 | 参考文档 | 代码量变化 |
|---|------|---------|-----------|
| 1 | **修复 TCP 帧协议**：length 2 字节 + CRC 校验 + 帧计数器 | [`ARCHITECTURE_OPTIMIZATION.md §1`](docs/ARCHITECTURE_OPTIMIZATION.md:21) | ~+30 行 |
| 2 | **删除死代码**：`QueryBuilder`、`BaseRepository`、`IntoResponse`、`define_repository!` | [`ARCHITECTURE_OPTIMIZATION.md §4`](docs/ARCHITECTURE_OPTIMIZATION.md:308) | **-100 行** |
| 3 | **提取共享 `resolve_or_create_device`**（已部分完成，强化类型安全） | [`PHASE2_OPTIMIZATION.md §2-5`](docs/PHASE2_OPTIMIZATION.md:250) | ~0 行（重构） |

### 8.3 阶段二：DTO/Service 精简（预计 3-4 天）

| # | 任务 | 参考文档 | 代码量变化 |
|---|------|---------|-----------|
| 4 | **Entity-as-Response**：删除 6 个冗余 DTO，Entity 加 `ToSchema` | [`PHASE2_OPTIMIZATION.md §2.1`](docs/PHASE2_OPTIMIZATION.md:40) | **-150 行** |
| 5 | **删除 `IntoResponse` + `ServiceConverter`** | [`PHASE2_OPTIMIZATION.md §2.2`](docs/PHASE2_OPTIMIZATION.md:90) | **-76 行** |
| 6 | **Service 辅助函数**：`ensure_found` + `PageResult` | [`PHASE2_OPTIMIZATION.md §2.3`](docs/PHASE2_OPTIMIZATION.md:123) | ~+12 行 |
| 7 | **Repository 错误模式统一**：`AppResult<Option<T>>` + `ensure_found` | [`PHASE2_OPTIMIZATION.md §2.6`](docs/PHASE2_OPTIMIZATION.md:269) | 重构 |

### 8.4 阶段三：Ingest 层插件化（预计 4-5 天）

| # | 任务 | 参考文档 | 代码量变化 |
|---|------|---------|-----------|
| 8 | **共享 MQTT 运行器增强**：`MessageHandler` 回调模式 | [`PHASE2_OPTIMIZATION.md §2.4`](docs/PHASE2_OPTIMIZATION.md:144) | ~+50 行 |
| 9 | **VisionModule 精简为 VisionPlugin** | — | **-178 行** |
| 10 | **ImuModule 精简为 ImuPlugin** | — | **-294 行** |
| 11 | **MattressModule TCP 背压控制** | [`INGEST_LAYER_AUDIT.md`](docs/INGEST_LAYER_AUDIT.md) | ~+30 行 |
| 12 | **配置自动化**：`create_plugins` 工厂函数 | — | ~+20 行 |

### 8.5 阶段四：Rocket 框架深度利用（预计 3-4 天）

| # | 任务 | 参考文档 | 代码量变化 |
|---|------|---------|-----------|
| 13 | **添加 `RequestLogger` Fairing** | §4.2 | ~+20 行，-20 行 `env_logger` |
| 14 | **自定义 `FromForm` 查询参数** | §4.3 | 重构，每路由 -5 行 |
| 15 | **合并配置到 Figment** | §4.5 | **-71 行**（`settings.rs`） |
| 16 | **添加 sqlx 编译期查询** | [`ARCHITECTURE_OPTIMIZATION.md §7.3`](docs/ARCHITECTURE_OPTIMIZATION.md:589) | 配置变更 |

### 8.6 阶段五：错误处理优化（预计 1-2 天）

| # | 任务 | 参考文档 | 代码量变化 |
|---|------|---------|-----------|
| 17 | **AppError 变体合并**：14→7 | §7.1 | **-40 行** |
| 18 | **Responder 实现简化** | §7.2 | ~-10 行 |

### 8.7 总体代码量预估

| 阶段 | 任务数 | 净行数变化 | 累计 |
|------|--------|-----------|------|
| 阶段一（修复+死代码） | 3 | **-70** | -70 |
| 阶段二（DTO/Service） | 4 | **-214** | -284 |
| 阶段三（Ingest 插件化） | 5 | **-372** | -656 |
| 阶段四（Rocket 深度利用） | 4 | **-71** | -727 |
| 阶段五（错误处理） | 2 | **-50** | **-777 行** |

**总计预估减少约 777 行代码，占当前代码总量约 15-20%。** 更重要的是，架构质量、可扩展性和类型安全性将显著提升。

---

## 9. 附录：Cargo crate 选用建议

### 9.1 当前依赖评估

| Crate | 当前用途 | 评估 | 建议 |
|-------|---------|------|------|
| [`rocket`](https://crates.io/crates/rocket) 0.5 | Web 框架 | ✅ 核心依赖 | 保持，启用更多 feature |
| [`sqlx`](https://crates.io/crates/sqlx) 0.8 | 数据库 | ✅ 核心依赖 | 启用 compile-time queries |
| [`tokio`](https://crates.io/crates/tokio) 1 | 异步运行时 | ✅ 核心依赖 | 保持 |
| [`rumqttc`](https://crates.io/crates/rumqttc) 0.24 | MQTT 客户端 | ✅ 核心依赖 | 保持，考虑升级到 0.24+ |
| [`serde`](https://crates.io/crates/serde) / `serde_json` | 序列化 | ✅ 核心依赖 | 保持 |
| [`chrono`](https://crates.io/crates/chrono) 0.4 | 时间处理 | ✅ 核心依赖 | 保持 |
| [`uuid`](https://crates.io/crates/uuid) 1 | UUID | ✅ 核心依赖 | 保持 |
| [`argon2`](https://crates.io/crates/argon2) 0.5 | 密码哈希 | ✅ 核心依赖 | 保持 |
| [`jsonwebtoken`](https://crates.io/crates/jsonwebtoken) 10 | JWT | ✅ 核心依赖 | 保持 |
| [`config`](https://crates.io/crates/config) 0.14 | 配置管理 | 🔶 可替代 | 可被 Rocket Figment 替代 |
| [`env_logger`](https://crates.io/crates/env_logger) 0.11 | 日志 | 🔶 可替代 | 可被 Rocket Fairing + `log` 替代 |
| [`rmp-serde`](https://crates.io/crates/rmp-serde) 1.1 | MessagePack | 🔶 可替代 | 可被 Rocket `MsgPack` 替代 |
| [`validator`](https://crates.io/crates/validator) 0.18 | 请求校验 | 🔶 可替代 | 可被 Rocket `#[field(validate)]` + `serde` 替代 |
| [`crc`](https://crates.io/crates/crc) 3.0 | CRC 校验 | ✅ 必要 | 保持（TCP 帧协议需要） |
| [`bytes`](https://crates.io/crates/bytes) 1 | 字节缓冲 | ✅ 必要 | 保持 |
| [`rust_decimal`](https://crates.io/crates/rust_decimal) 1.35 | 高精度数值 | ❓ 未使用？ | 如未使用可移除 |
| [`anyhow`](https://crates.io/crates/anyhow) 1 | 错误处理 | 🔶 可优化 | 仅 main.rs 和 init 函数中使用，可保留但限制范围 |
| [`thiserror`](https://crates.io/crates/thiserror) 2 | 错误 derive | ✅ 必要 | 保持 |
| [`async-trait`](https://crates.io/crates/async-trait) 0.1 | async trait | 🔶 可替代 | Rocket + tokio 已有支持，IngestModule trait 可改为 `Send + Sync` + 返回 boxed future |
| [`base64`](https://crates.io/crates/base64) 0.22 | Base64 编码 | ✅ 必要 | 保持（RawData 二进制预览） |
| [`csv`](https://crates.io/crates/csv) 1 | CSV 导出 | ✅ 必要 | 保持 |
| [`sha2`](https://crates.io/crates/sha2) 0.10 | SHA-256 哈希 | ✅ 必要 | 保持（refresh token 哈希） |
| [`futures-util`](https://crates.io/crates/futures-util) 0.3 | 异步工具 | ✅ 必要 | 保持（WebSocket） |
| [`tokio-tungstenite`](https://crates.io/crates/tokio-tungstenite) 0.29 | WebSocket | ✅ 必要 | 保持 |
| [`utoipa`](https://crates.io/crates/utoipa) 5 | OpenAPI | ✅ 必要 | 保持 |
| [`utoipa-swagger-ui`](https://crates.io/crates/utoipa-swagger-ui) 8 | Swagger UI | ✅ 必要 | 保持 |

### 9.2 建议新增的 crate

| Crate | 用途 | 替代方案 | 推荐优先级 |
|-------|------|---------|-----------|
| [`tracing`](https://crates.io/crates/tracing) + [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber) | 结构化日志/追踪 | 替代 `log` + `env_logger` | P2 |
| [`sentry`](https://crates.io/crates/sentry) 或类似 | 错误监控/报警 | 无 | P3 |
| [`deadpool`](https://crates.io/crates/deadpool) | 连接池（如 Redis） | 如后续引入 Redis | 未来 |
| [`redis`](https://crates.io/crates/redis) | 缓存/消息队列 | 用于设备状态缓存 | P3（如有需要） |

### 9.3 不建议引入的 crate

| Crate | 原因 |
|-------|------|
| [`diesel`](https://crates.io/crates/diesel) | ORM 过于重型，当前 sqlx 已满足需求 |
| [`actix-web`](https://crates.io/crates/actix-web) | 已选 Rocket，无需引入第二个 Web 框架 |
| [`lazy_static`](https://crates.io/crates/lazy_static) | Rust 1.75 已稳定支持 `OnceCell` / `OnceLock` |
| [`derive_builder`](https://crates.io/crates/derive_builder) | 当前 builder 模式仅 DataPoint 使用，手写即可 |

### 9.4 Rocket features 推荐启用

```toml
# Cargo.toml — Rocket features 优化
rocket = {
    version = "0.5",
    features = [
        "json",           # ✅ 已启用
        "msgpack",        # 🔶 新增：替代独立 rmp-serde
        "secrets",        # ✅ 已启用（JWT secret 管理）
        "tls",            # 🔶 新增：生产环境 HTTPS 支持
    ]
}
```

---

## 总结

本重设计方向的核心理念是：

1. **数据驱动**而非设备驱动——统一数据结构 + 灵活扩展字段
2. **插件化**——新设备接入从 300-500 行降至 ~80 行
3. **框架原生化**——充分利用 Rocket Fairing/FromForm/Figment 等特性
4. **消灭冗余**——DTO 缩减 45%、删除死代码、错误处理精简
5. **类型安全**——运行时字符串解析 → 编译期 Rust 类型

按阶段实施总计可减少约 **777 行代码**（~15-20%），同时大幅提升架构的扩展性和可维护性。

---

> **文档生成时间**: 2026-04-26
>
> **参考文档**:
> - [`ARCHITECTURE_OPTIMIZATION.md`](docs/ARCHITECTURE_OPTIMIZATION.md) — 全量代码审查优化分析
> - [`PHASE2_OPTIMIZATION.md`](docs/PHASE2_OPTIMIZATION.md) — 激进简化方案
> - [`INGEST_LAYER_AUDIT.md`](docs/INGEST_LAYER_AUDIT.md) — Ingest 层审计
> - [`ARCHITECTURE.md`](ARCHITECTURE.md) — 当前架构规范
