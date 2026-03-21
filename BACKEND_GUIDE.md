# 🔧 后端开发要点

本文档总结 IoT Health Platform 后端开发的关键设计决策和实现要点。

---

## 1. 核心设计决策

### 1.1 数据归属策略

| 决策点 | 选择 | 说明 |
|--------|------|------|
| 数据归属 | 写入时确定 | `datasheet.subject_id` 在数据入库时确定，不受后续绑定变化影响 |
| 设备绑定 | 一对一换绑 | 同一设备同一时间只能绑定一个患者 |
| 患者信息 | 极简 + 档案分离 | `patient` 表仅保留 ID 和基本标识，详细信息存 `patient_profile` |
| 数据协议 | 类型标识 + JSONB | `data_type` 区分类型，`payload` 灵活存储 |

### 1.2 设备管理策略

| 决策点 | 选择 | 说明 |
|--------|------|------|
| 设备注册 | 首次上报自动注册 | 根据序列号自动创建设备记录 |
| 设备类型 | 代码硬编码（Rust 枚举） | 编译时类型安全，新增设备需重新编译 |
| 适配器管理 | Trait + 枚举模式 | 每种设备类型实现 `DeviceAdapter` trait |

### 1.3 用户系统策略

| 决策点 | 选择 | 说明 |
|--------|------|------|
| 角色类型 | admin, user | 简单 RBAC，角色固定权限 |
| 认证方式 | JWT + Refresh Token | Access Token 2小时，Refresh Token 7天 |
| 密码哈希 | argon2 | 抗 GPU 破解 |
| 用户-患者绑定 | 预留表 | `user_patient_binding` 表，未来支持用户访问患者数据 |

---

## 2. 项目结构

```
src/
├── main.rs                     # 应用入口
├── lib.rs                      # 库入口，模块导出
│
├── api/                        # HTTP 接口层（Rocket）
│   ├── mod.rs
│   ├── openapi.rs              # OpenAPI 文档配置
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── auth.rs             # 登录/登出
│   │   ├── health.rs           # 健康检查
│   │   ├── user.rs             # 用户管理
│   │   ├── patient.rs          # 患者管理
│   │   ├── device.rs           # 设备管理
│   │   ├── binding.rs          # 绑定管理
│   │   └── data.rs             # 数据上报/查询
│   └── guards/
│       ├── mod.rs
│       └── auth.rs             # JWT 认证守卫
│
├── service/                    # 业务逻辑层
│   ├── mod.rs
│   ├── auth.rs                 # 认证服务（JWT + Refresh Token）
│   ├── user.rs                 # 用户服务
│   ├── patient.rs              # 患者服务
│   ├── device.rs               # 设备服务
│   ├── binding.rs              # 绑定服务
│   └── data.rs                 # 数据服务
│
├── repository/                 # 数据访问层（SQLx）
│   ├── mod.rs
│   ├── user.rs
│   ├── refresh_token.rs        # 刷新令牌 CRUD
│   ├── patient.rs
│   ├── device.rs
│   ├── binding.rs
│   └── data.rs
│
├── core/                       # 领域模型（纯 Rust）
│   ├── mod.rs
│   ├── auth/                   # 认证相关
│   │   ├── mod.rs
│   │   └── claims.rs           # JWT Claims
│   ├── entity/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── refresh_token.rs
│   │   ├── patient.rs
│   │   ├── device.rs
│   │   ├── binding.rs
│   │   └── datasheet.rs
│   └── value_object/
│       ├── mod.rs
│       ├── user_role.rs        # 用户角色枚举
│       ├── device_type.rs      # 设备类型枚举
│       └── data_type.rs        # 数据类型枚举
│
├── ingest/                     # 数据接入层 ⭐
│   ├── mod.rs
│   ├── mqtt_client.rs          # MQTT 客户端
│   ├── tcp_server.rs           # TCP 接入（预留）
│   └── adapters/               # 设备适配器 ⭐
│       ├── mod.rs
│       ├── trait.rs            # DeviceAdapter trait
│       ├── heart_rate.rs       # 心率监测器适配器
│       ├── fall_detector.rs    # 跌倒检测器适配器
│       └── spo2.rs             # 血氧仪适配器
│
├── dto/                        # 数据传输对象
│   ├── mod.rs
│   ├── request/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── user.rs
│   │   ├── patient.rs
│   │   ├── device.rs
│   │   └── data.rs
│   └── response/
│       ├── mod.rs
│       ├── auth.rs
│       ├── user.rs
│       ├── patient.rs
│       ├── device.rs
│       └── data.rs
│
├── errors/                     # 统一错误定义
│   ├── mod.rs
│   └── app_error.rs
│
├── config/                     # 配置管理
│   ├── mod.rs
│   └── settings.rs
│
└── utils/                      # 工具函数
    ├── mod.rs
    └── time.rs
```

---

## 3. 设备适配器设计（核心）

### 3.1 设计原则

- **编译时类型安全**：使用 Rust 枚举定义设备类型
- **Trait 抽象**：所有设备适配器实现统一的 `DeviceAdapter` trait
- **易于扩展**：新增设备类型只需添加枚举变体和实现 trait

### 3.2 DeviceAdapter Trait 定义

```rust
// src/ingest/adapters/trait.rs
use async_trait::async_trait;
use crate::errors::AppResult;

/// 设备适配器 trait - 所有设备类型必须实现
#[async_trait]
pub trait DeviceAdapter: Send + Sync {
    /// 解析原始数据为标准 JSON 格式
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value>;
    
    /// 验证数据有效性
    fn validate(&self, payload: &serde_json::Value) -> AppResult<()>;
    
    /// 获取数据类型标识
    fn data_type(&self) -> &'static str;
    
    /// 获取设备类型标识
    fn device_type(&self) -> &'static str;
}
```

### 3.3 设备类型枚举

```rust
// src/core/value_object/device_type.rs
use std::fmt;
use serde::{Deserialize, Serialize};

/// 设备类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    HeartRateMonitor,
    FallDetector,
    SpO2Sensor,
    // 新增设备类型在此添加
}

impl DeviceType {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "heart_rate_monitor" => Some(Self::HeartRateMonitor),
            "fall_detector" => Some(Self::FallDetector),
            "spo2_sensor" => Some(Self::SpO2Sensor),
            _ => None,
        }
    }
    
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HeartRateMonitor => "heart_rate_monitor",
            Self::FallDetector => "fall_detector",
            Self::SpO2Sensor => "spo2_sensor",
        }
    }
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
```

### 3.4 适配器注册表

```rust
// src/ingest/adapters/mod.rs
mod trait;
mod heart_rate;
mod fall_detector;
mod spo2;

pub use trait::DeviceAdapter;
use std::collections::HashMap;
use std::sync::Arc;
use crate::core::value_object::DeviceType;

/// 适配器注册表
pub struct AdapterRegistry {
    adapters: HashMap<DeviceType, Arc<dyn DeviceAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        let mut adapters: HashMap<DeviceType, Arc<dyn DeviceAdapter>> = HashMap::new();
        
        // 注册所有适配器
        adapters.insert(DeviceType::HeartRateMonitor, Arc::new(heart_rate::HeartRateAdapter));
        adapters.insert(DeviceType::FallDetector, Arc::new(fall_detector::FallDetectorAdapter));
        adapters.insert(DeviceType::SpO2Sensor, Arc::new(spo2::SpO2Adapter));
        
        Self { adapters }
    }
    
    /// 获取适配器
    pub fn get(&self, device_type: &DeviceType) -> Option<Arc<dyn DeviceAdapter>> {
        self.adapters.get(device_type).cloned()
    }
    
    /// 检查是否支持该设备类型
    pub fn is_supported(&self, device_type: &DeviceType) -> bool {
        self.adapters.contains_key(device_type)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

### 3.5 适配器实现示例

```rust
// src/ingest/adapters/heart_rate.rs
use super::trait::DeviceAdapter;
use crate::errors::{AppError, AppResult};
use serde_json::json;

/// 心率监测器适配器
pub struct HeartRateAdapter;

impl DeviceAdapter for HeartRateAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 假设数据格式：[心率高字节, 心率低字节]
        if raw.len() < 2 {
            return Err(AppError::ValidationError("数据长度不足".into()));
        }
        
        let heart_rate = u16::from_be_bytes([raw[0], raw[1]]) as u32;
        
        Ok(json!({
            "heart_rate": heart_rate,
            "unit": "bpm"
        }))
    }
    
    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let hr = payload["heart_rate"].as_u64().unwrap_or(0);
        
        if hr < 30 {
            return Err(AppError::ValidationError("心率过低".into()));
        }
        if hr > 220 {
            return Err(AppError::ValidationError("心率过高".into()));
        }
        
        Ok(())
    }
    
    fn data_type(&self) -> &'static str {
        "heart_rate"
    }
    
    fn device_type(&self) -> &'static str {
        "heart_rate_monitor"
    }
}
```

```rust
// src/ingest/adapters/fall_detector.rs
use super::trait::DeviceAdapter;
use crate::errors::{AppError, AppResult};
use serde_json::json;

/// 跌倒检测器适配器
pub struct FallDetectorAdapter;

impl DeviceAdapter for FallDetectorAdapter {
    fn parse_payload(&self, raw: &[u8]) -> AppResult<serde_json::Value> {
        // 假设数据格式：[事件类型, 置信度]
        if raw.is_empty() {
            return Err(AppError::ValidationError("数据为空".into()));
        }
        
        let event_type = match raw[0] {
            0 => "normal",
            1 => "fall_detected",
            2 => "impact_detected",
            _ => "unknown",
        };
        
        let confidence = if raw.len() > 1 { raw[1] as f32 / 100.0 } else { 0.0 };
        
        Ok(json!({
            "event_type": event_type,
            "confidence": confidence,
            "raw_data": raw.to_vec()
        }))
    }
    
    fn validate(&self, payload: &serde_json::Value) -> AppResult<()> {
        let event_type = payload["event_type"].as_str().unwrap_or("unknown");
        
        if event_type == "unknown" {
            return Err(AppError::ValidationError("未知事件类型".into()));
        }
        
        Ok(())
    }
    
    fn data_type(&self) -> &'static str {
        "fall_event"
    }
    
    fn device_type(&self) -> &'static str {
        "fall_detector"
    }
}
```

---

## 4. 设备自动注册机制

### 4.1 注册流程

```
设备上报数据
    │
    ▼
解析消息头（序列号、设备类型）
    │
    ▼
查询设备是否存在 ──否──► 创建设备记录
    │                      │
    │◄─────────────────────┘
    │
   是
    │
    ▼
返回设备信息
```

### 4.2 服务层实现

```rust
// src/service/device.rs
use crate::repository::DeviceRepository;
use crate::core::entity::Device;
use crate::core::value_object::DeviceType;
use crate::dto::request::RegisterDeviceRequest;
use crate::errors::{AppError, AppResult};
use sqlx::PgPool;
use log::info;

pub struct DeviceService<'a> {
    device_repo: DeviceRepository<'a>,
}

impl<'a> DeviceService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            device_repo: DeviceRepository::new(pool),
        }
    }
    
    /// 自动注册或获取设备
    /// 如果设备不存在，根据序列号自动创建
    // 日志: info!("设备自动注册: serial={}", serial_number);
    pub async fn auto_register_or_get(
        &self,
        serial_number: &str,
        device_type: &str,
    ) -> AppResult<Device> {
        // 1. 尝试查找现有设备
        if let Some(device) = self.device_repo.find_by_serial(serial_number).await? {
            info!("设备已存在: device_id={}", device.id);
            return Ok(device);
        }
        
        // 2. 验证设备类型
        let dev_type = DeviceType::from_str(device_type)
            .ok_or_else(|| {
                AppError::ValidationError(format!("未知设备类型: {}", device_type))
            })?;
        
        // 3. 自动创建设备
        let device = self.device_repo.insert(&NewDevice {
            serial_number: serial_number.to_string(),
            device_type: dev_type.as_str().to_string(),
            status: "active".to_string(),
            ..Default::default()
        }).await?;
        
        info!(
            "设备自动注册成功: device_id={}, device_type={}",
            device.id,
            device_type
        );
        
        Ok(device)
    }
    
    /// 手动注册设备（管理员接口）
    pub async fn register(&self, req: RegisterDeviceRequest) -> AppResult<Device> {
        // 验证序列号唯一性
        if self.device_repo.exists_by_serial(&req.serial_number).await? {
            return Err(AppError::ValidationError("设备序列号已存在".into()));
        }
        
        // 验证设备类型
        DeviceType::from_str(&req.device_type)
            .ok_or_else(|| AppError::ValidationError(format!("未知设备类型: {}", req.device_type)))?;
        
        self.device_repo.insert(&req.into()).await
    }
    
    /// 更新设备状态
    pub async fn update_status(&self, id: &uuid::Uuid, status: &str) -> AppResult<Device> {
        self.device_repo.update_status(id, status).await
    }
}
```

---

## 5. 数据接入流程

### 5.1 MQTT 接入流程

```
MQTT 消息到达
    │
    ▼
解析 Topic（获取设备标识）
    │
    ▼
解析消息头（序列号、设备类型、时间戳）
    │
    ▼
自动注册或获取设备
    │
    ▼
获取设备适配器
    │
    ▼
解析并验证数据
    │
    ▼
查询当前绑定的患者
    │
    ▼
存储数据（带 subject_id）
```

### 5.2 MQTT 客户端实现

```rust
// src/ingest/mqtt_client.rs
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use crate::service::{DeviceService, DataService, BindingService};
use crate::ingest::adapters::AdapterRegistry;
use crate::core::value_object::DeviceType;
use crate::errors::{AppError, AppResult};
use sqlx::PgPool;
use log::{error, info, warn};

pub struct MqttIngest<'a> {
    client: AsyncClient,
    device_service: DeviceService<'a>,
    data_service: DataService<'a>,
    binding_service: BindingService<'a>,
    adapter_registry: AdapterRegistry,
}

/// 消息头结构
struct MessageHeader {
    serial_number: String,
    device_type: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    raw_data: Vec<u8>,
}

impl<'a> MqttIngest<'a> {
    pub async fn new(
        pool: &'a PgPool,
        broker: &str,
        port: u16,
        client_id: &str,
    ) -> Self {
        let mut options = MqttOptions::new(client_id, broker, port);
        options.set_keep_alive(std::time::Duration::from_secs(30));
        
        let (client, mut eventloop) = AsyncClient::new(options, 10);
        let adapter_registry = AdapterRegistry::new();
        
        // 启动事件循环
        let pool_clone = pool.clone();
        let registry_clone = adapter_registry.clone(); // 需要 Clone trait
        
        tokio::spawn(async move {
            while let Ok(notification) = eventloop.poll().await {
                if let Event::Incoming(Incoming::Publish(publish)) = notification {
                    Self::handle_message_static(
                        &pool_clone,
                        &registry_clone,
                        &publish.topic,
                        &publish.payload,
                    ).await;
                }
            }
        });
        
        Self {
            client,
            device_service: DeviceService::new(pool),
            data_service: DataService::new(pool),
            binding_service: BindingService::new(pool),
            adapter_registry,
        }
    }
    
    pub async fn subscribe(&self, topic: &str) {
        self.client.subscribe(topic, QoS::AtLeastOnce).await.unwrap();
        info!(topic = %topic, "已订阅 MQTT 主题");
    }
    
    async fn handle_message_static(
        pool: &PgPool,
        registry: &AdapterRegistry,
        topic: &str,
        payload: &[u8],
    ) {
        if let Err(e) = Self::process_message(pool, registry, topic, payload).await {
            error!(error = %e, topic = %topic, "处理 MQTT 消息失败");
        }
    }
    
    async fn process_message(
        pool: &PgPool,
        registry: &AdapterRegistry,
        topic: &str,
        payload: &[u8],
    ) -> AppResult<()> {
        // 1. 解析 Topic: remipedia/{serial_number}/data
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() < 3 {
            return Err(AppError::ValidationError("无效的 Topic 格式".into()));
        }
        let serial_number = parts[1];
        
        // 2. 解析消息（假设 JSON 格式）
        let msg: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| AppError::ValidationError(format!("消息解析失败: {}", e)))?;
        
        let device_type = msg["device_type"].as_str()
            .ok_or_else(|| AppError::ValidationError("缺少 device_type".into()))?;
        let timestamp = msg["timestamp"].as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);
        let raw_data = msg["data"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
            .unwrap_or_default();
        
        // 3. 自动注册或获取设备
        let device_service = DeviceService::new(pool);
        let device = device_service.auto_register_or_get(serial_number, device_type).await?;
        
        // 4. 获取适配器
        let dev_type = DeviceType::from_str(&device.device_type)
            .ok_or_else(|| AppError::ValidationError("无效设备类型".into()))?;
        let adapter = registry.get(&dev_type)
            .ok_or_else(|| AppError::ValidationError("无对应适配器".into()))?;
        
        // 5. 解析并验证数据
        let data_payload = adapter.parse_payload(&raw_data)?;
        adapter.validate(&data_payload)?;
        
        // 6. 获取当前绑定的患者
        let binding_service = BindingService::new(pool);
        let subject_id = binding_service.get_current_binding_subject(&device.id).await?;
        
        // 7. 存储数据
        let data_service = DataService::new(pool);
        data_service.ingest(IngestData {
            time: timestamp,
            device_id: device.id,
            subject_id,
            data_type: adapter.data_type().to_string(),
            payload: data_payload,
            source: "mqtt".to_string(),
        }).await?;
        
        info!(
            device_id = %device.id,
            subject_id = ?subject_id,
            data_type = %adapter.data_type(),
            "数据入库成功"
        );
        
        Ok(())
    }
}
```

### 5.3 HTTP 数据上报接口

```rust
// src/api/routes/data.rs
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
use crate::dto::request::DataReportRequest;
use crate::dto::response::DataReportResponse;
use crate::service::DataService;
use crate::errors::AppResult;

/// 数据上报接口
#[post("/data", data = "<req>")]
pub async fn report_data(
    pool: &State<PgPool>,
    req: Json<DataReportRequest>,
) -> AppResult<Json<DataReportResponse>> {
    let service = DataService::new(pool);
    let result = service.report_http(req.into_inner()).await?;
    Ok(Json(result))
}

/// 数据查询接口
#[get("/data?<device_id>&<subject_id>&<data_type>&<start_time>&<end_time>&<page>&<page_size>")]
pub async fn query_data(
    pool: &State<PgPool>,
    device_id: Option<uuid::Uuid>,
    subject_id: Option<uuid::Uuid>,
    data_type: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Json<DataQueryResponse>> {
    let service = DataService::new(pool);
    let query = DataQuery {
        device_id,
        subject_id,
        data_type,
        start_time: start_time.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()),
        end_time: end_time.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()),
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    };
    let result = service.query(query).await?;
    Ok(Json(result))
}
```

---

## 6. 绑定服务实现

### 6.1 绑定服务

```rust
// src/service/binding.rs
use crate::repository::BindingRepository;
use crate::core::entity::Binding;
use crate::dto::request::CreateBindingRequest;
use crate::errors::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct BindingService<'a> {
    binding_repo: BindingRepository<'a>,
}

impl<'a> BindingService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            binding_repo: BindingRepository::new(pool),
        }
    }
    
    /// 创建绑定
    pub async fn bind(&self, device_id: &Uuid, patient_id: &Uuid) -> AppResult<Binding> {
        // 检查设备是否已有有效绑定
        if let Some(existing) = self.binding_repo.find_active_by_device(device_id).await? {
            return Err(AppError::BindingAlreadyExists);
        }
        
        self.binding_repo.create(device_id, patient_id).await
    }
    
    /// 解除绑定
    pub async fn unbind(&self, binding_id: &Uuid) -> AppResult<()> {
        self.binding_repo.end_binding(binding_id, Utc::now()).await
    }
    
    /// 获取设备当前绑定的患者 ID
    pub async fn get_current_binding_subject(&self, device_id: &Uuid) -> AppResult<Option<Uuid>> {
        let binding = self.binding_repo.find_active_by_device(device_id).await?;
        Ok(binding.map(|b| b.patient_id))
    }
    
    /// 获取设备的绑定历史
    pub async fn get_binding_history(&self, device_id: &Uuid) -> AppResult<Vec<Binding>> {
        self.binding_repo.find_all_by_device(device_id).await
    }
    
    /// 获取患者的当前绑定设备
    pub async fn get_current_binding_device(&self, patient_id: &Uuid) -> AppResult<Option<Binding>> {
        self.binding_repo.find_active_by_patient(patient_id).await
    }
}
```

---

## 7. 数据服务实现

### 7.1 数据服务

```rust
// src/service/data.rs
use crate::repository::DataRepository;
use crate::core::entity::Datasheet;
use crate::dto::{DataQuery, IngestData};
use crate::errors::AppResult;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct DataService<'a> {
    data_repo: DataRepository<'a>,
}

impl<'a> DataService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            data_repo: DataRepository::new(pool),
        }
    }
    
    /// 数据入库
    pub async fn ingest(&self, data: IngestData) -> AppResult<Datasheet> {
        self.data_repo.insert(&data).await
    }
    
    /// HTTP 数据上报
    pub async fn report_http(&self, req: DataReportRequest) -> AppResult<DataReportResponse> {
        let data = IngestData {
            time: req.timestamp.unwrap_or_else(Utc::now),
            device_id: req.device_id,
            subject_id: req.subject_id,
            data_type: req.data_type,
            payload: req.payload,
            source: "http".to_string(),
        };
        
        let result = self.ingest(data).await?;
        
        Ok(DataReportResponse {
            success: true,
            time: result.time,
            device_id: result.device_id,
        })
    }
    
    /// 查询数据
    pub async fn query(&self, query: DataQuery) -> AppResult<DataQueryResponse> {
        let total = self.data_repo.count(&query).await?;
        let data = self.data_repo.query(&query).await?;
        
        Ok(DataQueryResponse {
            data,
            pagination: Pagination {
                page: query.page,
                page_size: query.page_size,
                total,
                total_pages: (total + query.page_size as i64 - 1) / query.page_size as i64,
            },
        })
    }
    
    /// 按设备查询最新数据
    pub async fn get_latest(&self, device_id: &Uuid, data_type: Option<&str>) -> AppResult<Option<Datasheet>> {
        self.data_repo.find_latest(device_id, data_type).await
    }
}
```

---

## 8. Repository 层实现

### 8.1 设备 Repository

```rust
// src/repository/device.rs
use sqlx::PgPool;
use crate::core::entity::Device;
use crate::errors::{AppError, AppResult};
use uuid::Uuid;

pub struct DeviceRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DeviceRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
    
    pub async fn find_by_id(&self, id: &Uuid) -> AppResult<Device> {
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
    
    pub async fn find_by_serial(&self, serial_number: &str) -> AppResult<Option<Device>> {
        sqlx::query_as!(
            Device,
            r#"SELECT id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at
               FROM device WHERE serial_number = $1"#,
            serial_number
        )
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }
    
    pub async fn exists_by_serial(&self, serial_number: &str) -> AppResult<bool> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM device WHERE serial_number = $1 LIMIT 1"
        )
        .bind(serial_number)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(result.is_some())
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
    
    pub async fn update_status(&self, id: &Uuid, status: &str) -> AppResult<Device> {
        sqlx::query_as!(
            Device,
            r#"UPDATE device SET status = $2 WHERE id = $1
               RETURNING id, serial_number, device_type, firmware_version, status, metadata, created_at, updated_at"#,
            id,
            status,
        )
        .fetch_one(self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("设备: {}", id)),
            other => AppError::DatabaseError(other),
        })
    }
}
```

### 8.2 数据 Repository

```rust
// src/repository/data.rs
use sqlx::PgPool;
use crate::core::entity::Datasheet;
use crate::dto::DataQuery;
use crate::errors::AppResult;
use uuid::Uuid;

pub struct DataRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DataRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
    
    pub async fn insert(&self, data: &IngestData) -> AppResult<Datasheet> {
        sqlx::query_as!(
            Datasheet,
            r#"INSERT INTO datasheet (time, device_id, subject_id, data_type, payload, source)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING time, device_id, subject_id, data_type, payload, source, ingested_at"#,
            data.time,
            data.device_id,
            data.subject_id,
            data.data_type,
            data.payload,
            data.source,
        )
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }
    
    pub async fn query(&self, query: &DataQuery) -> AppResult<Vec<Datasheet>> {
        let offset = (query.page - 1) * query.page_size;
        
        sqlx::query_as!(
            Datasheet,
            r#"SELECT time, device_id, subject_id, data_type, payload, source, ingested_at
               FROM datasheet
               WHERE ($1::uuid IS NULL OR device_id = $1)
                 AND ($2::uuid IS NULL OR subject_id = $2)
                 AND ($3::text IS NULL OR data_type = $3)
                 AND ($4::timestamptz IS NULL OR time >= $4)
                 AND ($5::timestamptz IS NULL OR time <= $5)
               ORDER BY time DESC
               LIMIT $6 OFFSET $7"#,
            query.device_id,
            query.subject_id,
            query.data_type,
            query.start_time,
            query.end_time,
            query.page_size as i64,
            offset as i64,
        )
        .fetch_all(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }
    
    pub async fn count(&self, query: &DataQuery) -> AppResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM datasheet
               WHERE ($1::uuid IS NULL OR device_id = $1)
                 AND ($2::uuid IS NULL OR subject_id = $2)
                 AND ($3::text IS NULL OR data_type = $3)
                 AND ($4::timestamptz IS NULL OR time >= $4)
                 AND ($5::timestamptz IS NULL OR time <= $5)"#
        )
        .bind(query.device_id)
        .bind(query.subject_id)
        .bind(&query.data_type)
        .bind(query.start_time)
        .bind(query.end_time)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(result.0)
    }
    
    pub async fn find_latest(&self, device_id: &Uuid, data_type: Option<&str>) -> AppResult<Option<Datasheet>> {
        sqlx::query_as!(
            Datasheet,
            r#"SELECT time, device_id, subject_id, data_type, payload, source, ingested_at
               FROM datasheet
               WHERE device_id = $1 AND ($2::text IS NULL OR data_type = $2)
               ORDER BY time DESC
               LIMIT 1"#,
            device_id,
            data_type,
        )
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }
}
```

---

## 9. 错误处理

### 9.1 统一错误类型

```rust
// src/errors/app_error.rs
use thiserror::Error;
use rocket::http::Status;
use rocket::response::{Responder, Response};
use rocket::Request;
use serde_json::json;

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
    
    #[error("认证失败: {0}")]
    AuthenticationError(String),
    
    #[error("权限不足")]
    PermissionDenied,
    
    #[error("内部错误")]
    InternalError,
}

pub type AppResult<T> = Result<T, AppError>;

impl<'r> Responder<'r, 'r> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'r> {
        let status = match &self {
            AppError::NotFound(_) => Status::NotFound,
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::DeviceNotBound => Status::BadRequest,
            AppError::BindingAlreadyExists => Status::Conflict,
            AppError::AuthenticationError(_) => Status::Unauthorized,
            AppError::PermissionDenied => Status::Forbidden,
            _ => Status::InternalServerError,
        };
        
        let json = json!({
            "success": false,
            "error": self.to_string(),
            "code": status.code,
        });
        
        Response::build_from(json.respond_to(req)?)
            .status(status)
            .ok()
    }
}
```

---

## 10. 关键编写要点总结

| 模块 | 要点 | 注意事项 |
|------|------|---------|
| **设备适配器** | 使用 trait + 枚举模式 | 新增设备只需添加枚举变体和实现 trait |
| **自动注册** | 在数据接入时触发 | 根据序列号判断是否需要创建 |
| **数据归属** | 写入时确定 subject_id | 通过 binding 服务查询当前绑定 |
| **错误处理** | 使用 AppError 统一类型 | Repository 层转换 sqlx::Error |
| **日志记录** | 关键操作使用 log | 包含设备 ID、序列号等上下文 |
| **事务处理** | 绑定/解绑操作可能需要事务 | 确保数据一致性 |
| **SQL 归属** | 所有 SQL 只在 Repository 层 | 禁止跨层调用 |
| **参数绑定** | 必须使用参数化查询 | 禁止字符串拼接 SQL |

---

## 11. 开发顺序建议

1. **基础层**
   - [ ] 配置管理（config）
   - [ ] 错误定义（errors）
   - [ ] 领域模型（core/entity, core/value_object）

2. **数据层**
   - [ ] Repository 实现
   - [ ] 数据库连接池

3. **业务层**
   - [ ] Service 实现
   - [ ] 业务逻辑

4. **接口层**
   - [ ] API 路由
   - [ ] 认证守卫

5. **接入层**
   - [ ] MQTT 客户端
   - [ ] 设备适配器

6. **测试**
   - [ ] Repository 测试
   - [ ] Service 测试
   - [ ] API 测试

---

## 12. 新增设备类型步骤

1. 在 `src/core/value_object/device_type.rs` 添加枚举变体
2. 在 `src/ingest/adapters/` 创建新适配器文件
3. 实现 `DeviceAdapter` trait
4. 在 `src/ingest/adapters/mod.rs` 注册适配器
5. 编译测试

```rust
// 1. 添加枚举变体
pub enum DeviceType {
    // ...
    NewDeviceType,  // 新增
}

// 2. 创建适配器文件
// src/ingest/adapters/new_device.rs

// 3. 实现 trait
impl DeviceAdapter for NewDeviceAdapter {
    // ...
}

// 4. 注册适配器
adapters.insert(DeviceType::NewDeviceType, Arc::new(NewDeviceAdapter));
```