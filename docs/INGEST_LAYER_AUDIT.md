# Ingest 层审计与重构建议

日期: 2026-03-27

本文件记录当前仓库中 `ingest` 层的完整实现情况、文件树、关键实现摘要、发现的问题与优先级建议，作为后续重构的上下文留档。

## 一、文件树（`src/ingest`）

- src/ingest/
  - adapters/
    - mod.rs
    - adapter_trait.rs
    - engine.rs
    - engine_service.rs
    - fall_detector/
    - mattress/
    - spo2/
  - adapter_manager.rs
  - mod.rs
  - mqtt_client.rs
  - tcp_server.rs

（相关实现分别在：
- [src/ingest/adapters/mod.rs](src/ingest/adapters/mod.rs)
- [src/ingest/adapters/adapter_trait.rs](src/ingest/adapters/adapter_trait.rs)
- [src/ingest/mqtt_client.rs](src/ingest/mqtt_client.rs)
- [src/ingest/tcp_server.rs](src/ingest/tcp_server.rs)
）

## 二、当前实现摘要

- 模块化适配器架构：适配器按设备类型模块化，存在 `AdapterRegistry` 用于注册/获取适配器（见 [src/ingest/adapters/mod.rs](src/ingest/adapters/mod.rs)）。
- 适配器契约：通过 `DeviceAdapter` trait 统一提供 `parse`, `validate` 等方法，输出统一为 `AdapterOutput::Messages`（见 [src/ingest/adapters/adapter_trait.rs](src/ingest/adapters/adapter_trait.rs)）。
- MQTT 接入：使用 `rumqttc` 的 eventloop，在独立 tokio 任务中 poll 并对 `Publish` 事件调用处理逻辑，解析 topic（`{prefix}/{serial}/{type}`），自动注册设备并交由 `AdapterManager` 异步分发（见 [src/ingest/mqtt_client.rs](src/ingest/mqtt_client.rs)）。
- TCP 接入：启动 `TcpListener`，为每个连接 spawn 任务循环读流并按自定义二进制帧（魔数 + 长度）提取数据包，直接使用 `AdapterRegistry` 获取适配器解析/校验并直接通过 `DataService` 入库（见 [src/ingest/tcp_server.rs](src/ingest/tcp_server.rs)）。

## 三、主要发现的问题（按重要性分组）

### A. 稳定性与资源保护（高优先级）
- 存在 `unwrap()`/`expect()`：例如在 MQTT 订阅处直接 `unwrap()`，运行时出错会导致任务 panic 或静默失败（见 [src/ingest/mqtt_client.rs](src/ingest/mqtt_client.rs)）。
- 并发无上限：`TcpServer` 对每个连接 `tokio::spawn`，未限制最大并发连接数或单连接任务并发，容易在高并发下导致任务爆炸与 OOM。
- 后端同步入库：TCP 连接处理与 MQTT 分发链条中多数路径直接调用 `DataService::ingest`（单条写），在高吞吐时会给数据库造成巨大压力。

### B. 可恢复性与错误处理（高/中）
- 错误/重试策略薄弱：当解析失败或入库失败时，多数情况没有指数退避重试或死信落盘机制，容易丢失数据。
- 缺少 DLQ/回放：没有成熟的死信队列或离线回放机制以便恢复失败消息。

### C. 可观测性（中）
- 缺乏指标与追踪：未集成 `tracing`/OpenTelemetry 或 Prometheus 指标（如处理延迟、队列长度、失败计数等），不利于排查与容量规划。

### D. 架构/接口设计（中）
- 适配器接口为同步 trait：若适配器需要进行 CPU 密集或异步 I/O（例如解码大型 payload 或访问外部资源），当前接口不支持 async，或需在实现里使用 `spawn_blocking`，契约不明确。
- `DeviceAdapter::device_type()` 返回 `&'static str`，但注册/查询使用 `DeviceType` 枚举作为 key，存在字符串/枚举不一致风险。

### E. 性能优化点（中）
- 缺少批量写入：逐条写入会增加事务与连接开销，建议实现批量/缓冲写入策略。
- 无背压：消息分发直接调用适配器/入库，缺少有界队列或限流策略来施加背压。

### F. 安全与数据校验（低/中）
- 输入校验不足：未严格限制 payload 大小、未针对 topic/payload 做 schema 校验或速率限制，可能导致滥用或异常数据影响服务。

## 四、优先级建议（可执行步骤）

1. 快速修复（立即）：
   - 移除或替换 `unwrap()`/`expect()`，把错误返回并记录详细日志，避免 panic。（参考文件：[src/ingest/mqtt_client.rs](src/ingest/mqtt_client.rs)）
   - 在关键路径添加限流/信号量（`tokio::sync::Semaphore`）或在入口使用有界 `mpsc`，控制并发和背压。

2. 核心重构（短期迭代）：
   - 在 `AdapterManager`/入口处引入有界队列（或中间缓冲层，例如 Redis/Kafka），实现可控消费与 DLQ。
   - 将 `DataService` 支持批量写入并实现幂等键（例如 device_id + time + data_type）。

3. 可靠性与可观测性（中期）：
   - 引入重试（指数退避）、死信队列与回放工具。
   - 集成 `tracing` + OpenTelemetry/Prometheus，记录关键指标和分布式追踪。

4. 结构化改进（中长期）：
   - 将 `DeviceAdapter` 升级为 async trait（`async_trait`），或在契约中明确 CPU/I/O 模式並在 manager 里 dispatch 到合适线程池。
   - 提供运行时插件化适配器注册与配置加载，避免硬编码适配器。

5. 运维与部署：
   - 添加单元/集成测试覆盖、文档（`docs/`）与分阶段部署/回滚计划。

## 五、建议的重构分解（PR 级别）

- PR-1: 修复所有 `unwrap/expect` 并改为可上报的错误路径；补充单元测试覆盖最易触发路径。
- PR-2: 在 `TcpServer` 与 `MqttIngest` 引入并发控制（`Semaphore`）与有界 `mpsc`，并在超限时返回/降级到 DLQ。
- PR-3: 在 `AdapterManager` 实现消息隊列與 worker 池（每個 adapter 一個有界隊列或共享隊列）；提供 metrics 钩子。
- PR-4: `DataService` 支持 batch insert 與幂等策略替换单条写入路径。
- PR-5: 集成 `tracing` + Prometheus metrics；增加 dashboard/告警示例。

## 六、参考与后续工作

- 我已在 TODO 列表中记录了分解任务（包含 `有界队列与背压`、`适配器契约升级`、`批量写入` 等）。如果你同意，我可以立刻开始：
  - 选项 A：实现 PR-1（移除 `unwrap` 并添加错误处理）——低风险、快速合并；或
  - 选项 B：实现 PR-2（有界隊列与背压）——需要对 `AdapterManager` 与两处接入点修改。

---

文档由仓库代码审计自动汇总（审计文件位置：`src/ingest/*`）。如需我直接开始实现上述某个 PR，请回复选项或 PR 编号。
