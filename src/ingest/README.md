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

