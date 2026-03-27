**适配器与接入规范（Adapter & Ingest Integration）**

概述：本文件说明 `ingest` 层新的适配器契约、规范化数据格式（时间序列与事件）、接入点改动与升级步骤。

**1. 设计目标**
- 类型安全：使用领域类型替代裸 `serde_json::Value`，降低运行期错误。
- 最小转换：仅在数据库边界做一次 JSON 序列化。
- 事件/时序分流：事件需单独持久化/路由，时序数据进入 `datasheet`。
- 支持 stateful adapter（按设备实例维护引擎）并发安全。

**2. 新的适配器契约 (核心)**
- Trait: `DeviceAdapter`（路径：`src/ingest/adapters/adapter_trait.rs`）
  - fn `parse(&self, raw: &[u8]) -> AppResult<AdapterOutput>`
  - fn `validate(&self, output: &AdapterOutput) -> AppResult<()>`
  - fn `data_type(&self) -> &'static str`
  - fn `device_type(&self) -> &'static str`

- 输出类型：`AdapterOutput`（三种形式）
  - `Timeseries(TimeseriesPayload)`
  - `Events(Vec<EventPayload>)`
  - `Both { timeseries, events }`

- 边界工具：`AdapterOutput::to_json()` 在入库前生成可写 JSON。

**3. 规范化 JSON schema（helpers）**
- 文件：`src/ingest/adapters/schema.rs`
  - `timeseries_scalar(data_type, value, unit, ts)` -> 标量时间序列
  - `timeseries_composite(data_type, value, ts)` -> 复合时间序列
  - `event_payload(event_type, severity, ts, details)` -> 事件结构

示例：心率时序（规范化）
```
{
  "type": "heart_rate",
  "value": 72,
  "unit": "bpm",
  "timestamp": "2026-03-27T12:00:00Z"
}
```

示例：跌倒事件（规范化）
```
{
  "event_type": "person_fall",
  "severity": "high",
  "timestamp": "2026-03-27T12:00:01Z",
  "details": { "confidence": 0.92 }
}
```

**4. 接入点变更（MQTT / TCP）**
- `MqttIngest` 与 `TcpServer` 从原先调用 `parse_payload` -> `serde_json::Value`，变为：
  1. `let output = adapter.parse(raw)`
  2. `adapter.validate(&output)`
  3. `let write_value = output.to_json()`  // 交给 `DataService::ingest`

- 建议：`AdapterRegistry` 在应用启动时构建并注入 `MqttIngest`/`TcpServer`，而不是每次消息处理创建。

**5. 有状态适配器（stateful）模式**
- 两类适配器：无状态（单例）与有状态（设备级引擎）。
- 推荐实现：`StatefulFactory` 持有 `DashMap<DeviceId, Arc<Mutex<Engine>>>`。首次接入为设备创建 Engine 并缓存。
- Engine 内部使用 `tokio::sync::Mutex` 或 `parking_lot::Mutex` 以确保并发安全并降低阻塞范围。

**6. 数据库与事件处理建议**
- 继续保留通用 `datasheet`（灵活 JSONB 存储），但强烈建议新增 `health_event` 专表：

示例迁移（简要）：
```
CREATE TABLE health_event (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  event_time TIMESTAMPTZ NOT NULL,
  device_id UUID REFERENCES device(id),
  subject_id UUID REFERENCES patient(id),
  event_type TEXT NOT NULL,
  severity TEXT,
  status TEXT DEFAULT 'new',
  payload JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  acknowledged_by UUID,
  acknowledged_at TIMESTAMPTZ
);
CREATE INDEX idx_health_event_time ON health_event(event_time DESC);
CREATE INDEX idx_health_event_subject ON health_event(subject_id);
CREATE INDEX idx_health_event_type ON health_event(event_type);
CREATE INDEX idx_health_event_payload_gin ON health_event USING GIN (payload);
```

- 在 `DataService::ingest` 中：
  - 在 DB 事务内写入 `datasheet`（timeseries）。
  - 若 `AdapterOutput` 含事件，则同时写入 `health_event` 或发到事件队列（Kafka/Redis）供异步处理。

**7. 迁移与升级步骤（破坏性变更）**
1. 更新所有适配器实现为 `parse`/`validate` 签名（已在 `src/ingest/adapters/` 实现 heart_rate/fall_detector/spo2/mattress 示例）。
2. 更新 `MqttIngest` 与 `TcpServer` 按新流程调用 `parse`/`validate` 并使用 `to_json()` 写库。
3. 在 DB 添加 `health_event` 迁移（如需事件专表）。
4. 将 `AdapterRegistry` 作为单例注入服务启动路径。
5. 添加端到端集成测试：模拟 MQTT/TCP 消息 -> 适配器 -> `DataService::ingest` -> `health_event`/`datasheet`。

注意：此变更不兼容旧的 `parse_payload` 接口，必须同时升级所有自定义适配器与外部调用点。

**8. 开发/测试命令**
- 本地格式化与检查：
```
cargo fmt
cargo clippy
```
- 运行测试：
```
cargo test -p remipedia
```

**9. 示例：实现要点（以心率为例）**
- `parse`：从 raw bytes 解析 `HeartRateData`，生成 `TimeseriesPayload`（使用 `schema::timeseries_scalar`）并在必要时创建 `EventPayload`。
- `validate`：仅做结构性检查（字段存在、格式），不要因异常生理值拒绝消息，异常应作为事件产生并写入 `health_event`。

**10. 下一步建议**
- 添加 `EventRepository` 和 `EventService`，并在 `DataService::ingest` 中调用以保证事件和时序数据的一致写入。
- 为高负载场景评估 TimescaleDB 或基于时间的表分区策略。

如需我把上述迁移 SQL、`EventRepository` 范例代码和一组集成测试脚本一并提交，我可以继续实现。
**适配器与接入规范（Adapter & Ingest Integration）**

概述：本文件说明 `ingest` 层新的适配器契约、规范化数据格式（时间序列与事件）、接入点改动与升级步骤。

**1. 设计目标**
- 类型安全：使用领域类型替代裸 `serde_json::Value`，降低运行期错误。
- 最小转换：仅在数据库边界做一次 JSON 序列化。
- 事件/时序分流：事件需单独持久化/路由，时序数据进入 `datasheet`。
- 支持 stateful adapter（按设备实例维护引擎）并发安全。

**2. 新的适配器契约 (核心)**
- Trait: `DeviceAdapter`（路径：`src/ingest/adapters/adapter_trait.rs`）
  - fn `parse(&self, raw: &[u8]) -> AppResult<AdapterOutput>`
  - fn `validate(&self, output: &AdapterOutput) -> AppResult<()>`
  - fn `data_type(&self) -> &'static str`
  - fn `device_type(&self) -> &'static str`

- 输出类型：`AdapterOutput`（三种形式）
  - `Timeseries(TimeseriesPayload)`
  - `Events(Vec<EventPayload>)`
  - `Both { timeseries, events }`

- 边界工具：`AdapterOutput::to_json()` 在入库前生成可写 JSON。

**3. 规范化 JSON schema（helpers）**
- 文件：`src/ingest/adapters/schema.rs`
  - `timeseries_scalar(data_type, value, unit, ts)` -> 标量时间序列
  - `timeseries_composite(data_type, value, ts)` -> 复合时间序列
  - `event_payload(event_type, severity, ts, details)` -> 事件结构

示例：心率时序（规范化）
```
{
  "type": "heart_rate",
  "value": 72,
  "unit": "bpm",
  "timestamp": "2026-03-27T12:00:00Z"
}
```

示例：跌倒事件（规范化）
```
{
  "event_type": "person_fall",
  "severity": "high",
  "timestamp": "2026-03-27T12:00:01Z",
  "details": { "confidence": 0.92 }
}
```

**4. 接入点变更（MQTT / TCP）**
- `MqttIngest` 与 `TcpServer` 从原先调用 `parse_payload` -> `serde_json::Value`，变为：
  1. `let output = adapter.parse(raw)`
  2. `adapter.validate(&output)`
  3. `let write_value = output.to_json()`  // 交给 `DataService::ingest`

- 建议：`AdapterRegistry` 在应用启动时构建并注入 `MqttIngest`/`TcpServer`，而不是每次消息处理创建。

**5. 有状态适配器（stateful）模式**
- 两类适配器：无状态（单例）与有状态（设备级引擎）。
- 推荐实现：`StatefulFactory` 持有 `DashMap<DeviceId, Arc<Mutex<Engine>>>`。首次接入为设备创建 Engine 并缓存。
- Engine 内部使用 `tokio::sync::Mutex` 或 `parking_lot::Mutex` 以确保并发安全并降低阻塞范围。

**6. 数据库与事件处理建议**
- 继续保留通用 `datasheet`（灵活 JSONB 存储），但强烈建议新增 `health_event` 专表：

示例迁移（简要）：
```
CREATE TABLE health_event (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  event_time TIMESTAMPTZ NOT NULL,
  device_id UUID REFERENCES device(id),
  subject_id UUID REFERENCES patient(id),
  event_type TEXT NOT NULL,
  severity TEXT,
  status TEXT DEFAULT 'new',
  payload JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  acknowledged_by UUID,
  acknowledged_at TIMESTAMPTZ
);
CREATE INDEX idx_health_event_time ON health_event(event_time DESC);
CREATE INDEX idx_health_event_subject ON health_event(subject_id);
CREATE INDEX idx_health_event_type ON health_event(event_type);
CREATE INDEX idx_health_event_payload_gin ON health_event USING GIN (payload);
```

- 在 `DataService::ingest` 中：
  - 在 DB 事务内写入 `datasheet`（timeseries）。
  - 若 `AdapterOutput` 含事件，则同时写入 `health_event` 或发到事件队列（Kafka/Redis）供异步处理。

**7. 迁移与升级步骤（破坏性变更）**
1. 更新所有适配器实现为 `parse`/`validate` 签名（已在 `src/ingest/adapters/` 实现 heart_rate/fall_detector/spo2/mattress 示例）。
2. 更新 `MqttIngest` 与 `TcpServer` 按新流程调用 `parse`/`validate` 并使用 `to_json()` 写库。
3. 在 DB 添加 `health_event` 迁移（如需事件专表）。
4. 将 `AdapterRegistry` 作为单例注入服务启动路径。
5. 添加端到端集成测试：模拟 MQTT/TCP 消息 -> 适配器 -> `DataService::ingest` -> `health_event`/`datasheet`。

注意：此变更不兼容旧的 `parse_payload` 接口，必须同时升级所有自定义适配器与外部调用点。

**8. 开发/测试命令**
- 本地格式化与检查：
```
cargo fmt
cargo clippy
```
- 运行测试：
```
cargo test -p remipedia
```

**9. 示例：实现要点（以心率为例）**
- `parse`：从 raw bytes 解析 `HeartRateData`，生成 `TimeseriesPayload`（使用 `schema::timeseries_scalar`）并在必要时创建 `EventPayload`。
- `validate`：仅做结构性检查（字段存在、格式），不要因异常生理值拒绝消息，异常应作为事件产生并写入 `health_event`。

**10. 下一步建议**
- 添加 `EventRepository` 和 `EventService`，并在 `DataService::ingest` 中调用以保证事件和时序数据的一致写入。
- 为高负载场景评估 TimescaleDB 或基于时间的表分区策略。

如需我把上述迁移 SQL、`EventRepository` 范例代码和一组集成测试脚本一并提交，我可以继续实现。
