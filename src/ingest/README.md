# 设备接入框架

## 概述

统一的设备接入框架，支持多种设备类型的数据流处理、状态管理和事件生成。框架设计遵循以下原则：

- **易于扩展**：添加新设备只需实现核心 trait
- **类型安全**：使用 Rust 类型系统保证正确性
- **异步友好**：基于 tokio 异步运行时
- **职责分离**：清晰的模块边界

## 核心架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Transport Layer                        │
│  (TCP/MQTT/HTTP) → 原始数据帧 → InboundMessage              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    DeviceManager                            │
│  设备实例管理 → 适配器分发 → 状态维护 → 持久化               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    DeviceAdapter                            │
│  原始数据解析 → 验证 → AdapterOutput → MessagePayload        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    DeviceState                              │
│  设备特定状态 → 状态更新 → 状态快照                          │
└─────────────────────────────────────────────────────────────┘
```

## 核心 Trait

### DeviceAdapter

负责解析原始数据并验证。

```rust
pub trait DeviceAdapter: Send + Sync {
    /// 获取设备元信息
    fn metadata(&self) -> DeviceMetadata;
    
    /// 解析原始数据
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput>;
    
    /// 验证解析后的输出
    fn validate(&self, output: &AdapterOutput) -> AppResult<()>;
    
    /// 获取设备类型
    fn device_type(&self) -> DeviceType {
        self.metadata().device_type
    }
}
```

### DeviceState

管理设备特定状态（可选）。

```rust
pub trait DeviceState: Send + Sync {
    /// 更新状态
    fn update(&mut self, data: &MessagePayload) -> AppResult<()>;
    
    /// 获取状态快照
    fn snapshot(&self) -> Value;
    
    /// 重置状态
    fn reset(&mut self);
}
```

## 核心类型

### DeviceType

设备类型标识，当前支持：

- `SmartMattress` - 智能床垫
- `HeartRateMonitor` - 心率监测器（预留）
- `FallDetector` - 跌倒检测器（预留）

### DeviceMetadata

设备元信息：

```rust
pub struct DeviceMetadata {
    pub device_type: DeviceType,
    pub display_name: String,
    pub description: String,
    pub supported_data_types: Vec<String>,
    pub protocol_version: String,
}
```

### AdapterOutput

适配器输出：

```rust
pub enum AdapterOutput {
    Messages(Vec<MessagePayload>),
    Empty,
}
```

### MessagePayload

统一消息负载：

```rust
pub struct MessagePayload {
    pub time: DateTime<Utc>,
    pub data_type: String,
    pub message_type: Option<String>,
    pub severity: Option<String>,
    pub payload: Value,
}
```

## 使用示例

### 1. 实现新的适配器

```rust
pub struct MyDeviceAdapter;

impl DeviceAdapter for MyDeviceAdapter {
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: DeviceType::SmartMattress,
            display_name: "我的设备".to_string(),
            description: "自定义设备适配器".to_string(),
            supported_data_types: vec!["my_device".to_string()],
            protocol_version: "1.0".to_string(),
        }
    }
    
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        // 解析原始数据
        let data: Value = serde_json::from_slice(raw)?;
        
        let msg = MessagePayload {
            time: Utc::now(),
            data_type: "my_device".to_string(),
            message_type: None,
            severity: None,
            payload: data,
        };
        
        Ok(AdapterOutput::Messages(vec![msg]))
    }
    
    fn validate(&self, output: &AdapterOutput) -> AppResult<()> {
        // 验证逻辑
        Ok(())
    }
}
```

### 2. 注册适配器

```rust
// 在 main.rs 中
let mut registry = AdapterRegistry::new();
registry.register(Arc::new(MyDeviceAdapter::new()));
```

### 3. 处理设备数据

```rust
// DeviceManager 会自动管理设备实例
let result = device_manager.process(
    serial_number,
    device_type,
    raw_data,
    "tcp", // 或 "mqtt"
).await?;
```

## 扩展指南

### 添加新设备类型

1. 在 `core/value_object/device_type.rs` 中添加新的 `DeviceType` 变体
2. 在 `adapters/` 下创建新目录，实现 `DeviceAdapter` trait
3. 在 `main.rs` 中注册新适配器
4. 实现 `DeviceState` trait（如果需要状态管理）

### 自定义状态管理

```rust
pub struct MyDeviceState {
    last_value: i32,
    history: Vec<i32>,
}

impl DeviceState for MyDeviceState {
    fn update(&mut self, data: &MessagePayload) -> AppResult<()> {
        // 更新状态
        Ok(())
    }
    
    fn snapshot(&self) -> Value {
        serde_json::json!({
            "last_value": self.last_value,
            "history_len": self.history.len(),
        })
    }
    
    fn reset(&mut self) {
        self.last_value = 0;
        self.history.clear();
    }
}
```

## 文件结构

```
src/ingest/
├── framework.rs          # 核心框架（DeviceManager, DeviceAdapter, DeviceState）
├── mod.rs               # 模块导出
├── transport/           # 网络传输层
│   ├── mod.rs           # Transport trait 和 TransportContext
│   ├── tcp.rs           # TCP 传输实现
│   └── mqtt.rs          # MQTT 传输实现
└── adapters/            # 设备适配器
    └── mattress/        # 智能床垫适配器
        ├── mod.rs       # 模块导出
        ├── adapter.rs   # MattressAdapter 实现
        ├── types.rs     # 类型定义
        ├── event_engine.rs  # 事件引擎
        └── transport.rs # 专用传输（可选）
```

## API 探知

框架提供以下 API 端点用于设备信息查询：

- `GET /api/v1/admin/devices/types` - 获取支持的设备类型
- `GET /api/v1/admin/devices/sessions` - 获取设备会话列表
- `GET /api/v1/admin/devices/status` - 获取设备系统状态
- `POST /api/v1/admin/devices/sessions/cleanup` - 清理空闲会话

## 性能考虑

1. **异步处理**：所有 I/O 操作都是异步的
2. **状态隔离**：每个设备实例有独立的状态
3. **内存效率**：使用 `Arc` 共享只读数据
4. **错误隔离**：单个设备失败不影响其他设备

## 迁移说明

旧的 `AdapterRegistry` 和 `DeviceAdapter` trait 已被新的 `framework` 模块替代。主要变化：

1. `DeviceType` 从自定义结构体改为枚举
2. 移除了 `DeviceModule` trait，简化注册流程
3. `AdapterRegistry` 现在使用 `Arc<dyn DeviceAdapter>`
4. `DeviceManager` 直接管理设备实例，无需中间层

## 快速开始：添加新设备

### 步骤1：定义设备类型

在 `core/value_object/device_type.rs` 中添加新的设备类型：

```rust
pub enum DeviceType {
    SmartMattress,
    HeartRateMonitor,    // 现有
    FallDetector,        // 现有
    MyNewDevice,         // 新增
}
```

### 步骤2：实现设备适配器

在 `adapters/` 下创建新目录 `my_device/adapter.rs`：

```rust
use crate::errors::{AppError, AppResult};
use crate::ingest::framework::{
    AdapterOutput, DeviceAdapter, DeviceMetadata, MessagePayload,
};
use crate::core::value_object::DeviceType;
use chrono::Utc;
use serde_json::Value;

pub struct MyDeviceAdapter;

impl MyDeviceAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceAdapter for MyDeviceAdapter {
    fn metadata(&self) -> DeviceMetadata {
        DeviceMetadata {
            device_type: DeviceType::MyNewDevice,
            display_name: "我的设备".to_string(),
            description: "自定义设备适配器".to_string(),
            supported_data_types: vec!["my_device_data".to_string()],
            protocol_version: "1.0".to_string(),
        }
    }
    
    fn parse(&self, raw: &[u8]) -> AppResult<AdapterOutput> {
        // 解析原始数据（假设是JSON格式）
        let data: Value = serde_json::from_slice(raw)
            .map_err(|e| AppError::ValidationError(format!("JSON解析失败: {}", e)))?;
        
        // 提取需要的数据
        let value = data.get("value")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        
        // 创建消息
        let msg = MessagePayload {
            time: Utc::now(),
            data_type: "my_device_data".to_string(),
            message_type: None,
            severity: None,
            payload: serde_json::json!({
                "value": value,
                "timestamp": Utc::now().to_rfc3339(),
            }),
        };
        
        Ok(AdapterOutput::Messages(vec![msg]))
    }
    
    fn validate(&self, output: &AdapterOutput) -> AppResult<()> {
        match output {
            AdapterOutput::Messages(msgs) => {
                if msgs.is_empty() {
                    return Err(AppError::ValidationError("空消息".into()));
                }
                for msg in msgs {
                    if msg.data_type != "my_device_data" {
                        return Err(AppError::ValidationError("无效的数据类型".into()));
                    }
                }
                Ok(())
            }
            AdapterOutput::Empty => Ok(()),
        }
    }
}
```

### 步骤3：注册适配器

在 `main.rs` 中注册新适配器：

```rust
// 在 build_rocket 函数中
let mut registry = AdapterRegistry::new();

// 注册现有适配器
registry.register(Arc::new(remipedia::ingest::adapters::mattress::MattressAdapter::new()));

// 注册新适配器
registry.register(Arc::new(remipedia::ingest::adapters::my_device::MyDeviceAdapter::new()));
```

### 步骤4：创建模块文件

在 `adapters/my_device/mod.rs` 中：

```rust
pub mod adapter;
pub use adapter::MyDeviceAdapter;
```

并在 `adapters/mod.rs` 中添加：

```rust
pub mod my_device;
```

### 步骤5：测试新设备

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::framework::DeviceAdapter;
    
    #[test]
    fn test_my_device_adapter() {
        let adapter = MyDeviceAdapter::new();
        let raw = br#"{"value": 42}"#;
        
        let output = adapter.parse(raw).unwrap();
        assert!(adapter.validate(&output).is_ok());
    }
}
```

## 最佳实践

1. **保持适配器无状态**：所有状态应该由 `DeviceState` trait 管理
2. **使用异步**：I/O 操作应该是异步的
3. **错误处理**：提供有意义的错误信息
4. **测试覆盖**：为每个适配器编写单元测试
5. **文档**：为每个设备类型编写文档说明

## 故障排查

### 常见问题

1. **设备类型不匹配**：确保 `DeviceType` 枚举值与适配器元信息一致
2. **解析失败**：检查原始数据格式是否符合预期
3. **状态不一致**：确保 `DeviceState` 的 `update` 方法正确实现

### 调试技巧

1. 启用日志：设置 `RUST_LOG=debug` 环境变量
2. 检查设备管理器状态：使用 `/api/v1/admin/devices/status` 端点
3. 查看设备会话：使用 `/api/v1/admin/devices/sessions` 端点
