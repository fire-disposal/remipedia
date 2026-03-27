# Ingest 模块架构说明（简要）

目的
- 处理所有来自设备层（TCP、MQTT 等）的原始数据帧，并把它们转换为系统内部的时序/事件数据写入数据库。

主要组件（职责）
- transport/*
  - 负责网络接入、帧边界检测与基础格式识别（例如床垫的 MessagePack/TLV）。
  - 不负责复杂领域解析或持久化。
  - 将帧封装为 `InboundMessage`（含 device_id、subject_id、device_type、raw_payload、source）并交给 `AdapterManager`。

- adapters/*
  - 每种设备类型实现一个 `DeviceAdapter`，负责把 `raw_payload` 解析为统一的 `AdapterOutput::Messages`（一组扁平消息）。
  - `parse()` 应为同步阻塞函数，调用方应在 `spawn_blocking` 中执行以避免阻塞 async runtime。
  - `validate()` 在解析后用于业务合法性检查。

- adapter_manager.rs
  - 运行时的工作池：为每个 `device_type` 启动一个 Worker（有界 mpsc），负责调用适配器解析并把结果写入 `DataService`。
  - 负责错误隔离（单个适配器解析失败不影响其他适配器）。

- event_engine（位于 adapters 下的具体引擎）
  - 把设备原始采样映射为事件（BedEntry、Apnea、翻身等），应为纯函数式接口或在可控的 service 中提供幂等保证。

- DataService / 持久化
  - 负责接收由 `AdapterManager` 转换后的 `IngestData` 并写入数据库；应支持批量、重试与幂等性保证。

数据流（简要）
1. Transport 接收网络字节流 -> 提取帧边界 -> 生成原始帧。
2. Transport 通过 `AdapterManager::dispatch(device_type, InboundMessage)` 把原始帧发给对应 device_type 的 worker。
3. Worker 在 `spawn_blocking` 中调用 `adapter.parse(raw)` -> 得到 `AdapterOutput` -> `DataService::ingest`。
4. 若适配器生成事件（由 event_engine 产生），事件也作为独立消息入库或下发通知。

设计约束与建议
- Trait 保持：`DeviceAdapter` 作为扩展点是有价值的（便于插件式扩展），但应统一使用 `Arc<dyn DeviceAdapter>` 并避免在运行时混用具体类型与 trait，保持一致性。
- DB 池注入：统一在应用启动时创建 `PgPool` 并注入到 `AdapterManager` / `TransportContext`，避免 transport 在运行时隐式创建连接。
- 责任边界：Transport 只负责网络与帧，Adapter 只负责解析与验证，AdapterManager 负责分发与持久化。
- 性能：把 CPU/阻塞解析放到 `spawn_blocking`，并使用有界队列（backpressure）避免 OOM。
- 可观测性：记录每个适配器的队列长度、解析耗时与入库失败率；在生产环境强制打开 `tracing` 或 metrics。

扩展步骤（开发者指南）
- 新适配器：在 `adapters` 下新增模块实现 `DeviceAdapter`，并在 `AdapterRegistry::new()` 中注册。
- 新 Transport：实现 `Transport` trait并在 `main.rs` 中注册到 `TransportManager`。
- 状态存储：实现 `StateStore` trait（若需要跨重启保持状态），并注入到 event_engine 服务中。

常见错误与排查
- parse 方法不可见：确保在使用 `adapter.parse()` 的文件顶部引入 `use crate::ingest::adapters::DeviceAdapter;`。
- device_type 不匹配：Transport 层生成的 `device_type` 字符串必须与 `AdapterRegistry` 中 `DeviceType::as_str()` 保持一致。
- DB 连接分散：若在 transport 里看到 `PgPool::connect_lazy`/`connect`，请改为从 `TransportContext` 或应用启动注入。

文件参考
- Adapter 管理: `src/ingest/adapter_manager.rs`
- Transport 接入: `src/ingest/transport/mod.rs` 与 `src/ingest/transport/*`
- 床垫适配器: `src/ingest/adapters/mattress/*`


快速任务清单
- 保持 `DeviceAdapter` trait，但在全库采用一致的 `Arc<dyn DeviceAdapter>` 模式。
- 统一注入 `PgPool` 并移除 transport 内部的隐式 DB 连接。
- 增加更多集成测试以覆盖 AdapterManager 全路径。

---

## 2026-03：adapter / transport 协作优化（最新）

本轮已经把「设备发现 + 分发」主路径统一收敛到了 `AdapterManager`，让新增 adapter 与新增 transport 的对接更丝滑：

- `AdapterManager::new(pool, registry)` 改为显式接收共享 `AdapterRegistry`，避免运行时重复构建注册表。
- 新增 `AdapterManager::dispatch_by_serial(...)`：
  - 统一做 `device_type` 规范化（`DeviceType::from_str`）；
  - 统一校验是否存在对应 adapter worker；
  - 统一执行设备自动注册、绑定查询、`InboundMessage` 组装与 dispatch。
- `MqttTransport` 不再自行 `connect_lazy` 建 DB 连接；改为直接调用 `dispatch_by_serial`。
- `TcpTransport` 不再自行处理设备自动注册/绑定逻辑；只负责网络收包和必要的 serial 提取后调用 `dispatch_by_serial`。
- 启动流程（`main.rs`）中，`AdapterRegistry` 与 `AdapterManager` 使用同一个共享实例注入到 `TransportContext`，保持单一事实来源（single source of truth）。

### 结果
- Transport 层职责更纯粹：网络协议 + 原始帧提取。
- Adapter 层职责更清晰：解析与校验。
- 新增 transport 或 adapter 时，接线点统一，重复代码明显减少，协作成本更低。
