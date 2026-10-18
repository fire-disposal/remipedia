# Remipedia

老年人监测的后端：床垫走 TCP、视觉与 IMU 走 MQTT，指标与事件统一落 PostgreSQL，再对外提供一套带模块权限的 HTTP API。

**状态**：可用（单机 Docker 部署）；**入口**：`/swagger-ui/` 或本地 `cargo run`；**仓库**：https://github.com/fire-disposal/remipedia
**相关仓库**：Web 管理端 [remepui](https://github.com/fire-disposal/remepui)、移动端联调工具 [remepmobile](https://github.com/fire-disposal/remepmobile)

## 本质

把床垫 TCP、视觉与 IMU MQTT 统一落库并对外提供权限 API 的健康后端。

## 选型

基于 Rust 的单体服务，后端 Rocket，数据 PostgreSQL 16 + SQLx，接入 TCP + MQTT，部署 Docker Compose。

- 语言与运行时
  - Rust 2021
  - Tokio
- 后端
  - Rocket 0.5
    - fairing
    - request guard
  - utoipa
    - utoipa-swagger-ui
  - thiserror
  - anyhow
  - tracing
  - tracing-subscriber
- 数据
  - PostgreSQL 16
    - SQLx 0.8
      - migrations
      - JSONB
    - rust_decimal
- 接入
  - TCP
    - tokio TcpListener
    - rmp-serde
    - crc8
  - MQTT
    - rumqttc 0.24
      - vision
      - imu
- 认证
  - jsonwebtoken
  - argon2
- 配置
  - config
  - YAML
- 部署
  - Docker Compose
  - GitHub Actions

## 功能

- HTTP：`/api/v1` 下覆盖 `/auth`（login/refresh/logout/me/verify/register/revoke/sessions/change-password）、`/users`、`/patients`（含 `/detail`、`/devices`、`/profile`）、`/devices`（含 `/stats`、`/bindings`）、`/bindings`（含 `/end`、`/switch`、`/history`）、`/data`（写入、分页查询、`/latest`、`/alerts`、`/alerts/stats`、事件确认/解决）、`/admin`（角色 CRUD、模块授权、模块列表、审计日志）。
- 健康检查：`/health`、`/ready`（探测数据库）、`/live`、`/ingest/modules`。
- 权限：角色 + 10 个模块 code + 数据范围 `all`/`self`（`department` 仅有枚举）；管理操作写审计日志。
- 接入：床垫 TCP `5858`（Magic + 长度 + CRC8 + msgpack，64KB 上限，连续 10 帧错误断开，自动注册设备）；视觉与 IMU 各一个 MQTT client，跌倒阈值默认 25 m/s²。
- 数据模型：`data_streams` + `observations` + `alert_events`，`ingest_raw_data` 归档原始报文（`stored`/`ingested`/`ignored`/`format_error`/`processing_error`）。
- 运维：健康/就绪/存活探针，Docker Compose 与 GitHub Actions 部署。
- 测试：约 50 个 `#[test]` / `#[tokio::test]`，集中在值对象、claims、错误映射与实体行为。
- 未实现：WebSocket 接入、死信队列、背压、设备指令下发。

## 启动流程

```bash
docker compose up -d postgres mqtt
cargo run                       # 读 config/default.yaml 与可选 config/local.yaml
```

- 整栈：`docker compose up -d`，应用在 `127.0.0.1:18000`（容器内 8000），床垫 TCP 暴露 `5858`，Mosquitto `1883`。
- 文档：`http://localhost:8000/swagger-ui/`，JSON 在 `/api-docs/openapi.json`。
- 环境变量用双下划线：`APP_DATABASE__URL`、`APP_MQTT__BROKER`、`APP_SERVER__PORT`、`APP_JWT__SECRET`。
- 首次启动自动迁移，并用 `ADMIN_USERNAME` / `ADMIN_PASSWORD` 创建超级管理员，未显式设置时用代码内置默认值，部署前必须改密。

## 边界与已知问题

- WebSocket 没有实现：配置里有 `websocket.port`，`tokio-tungstenite` 与 `futures-util` 也已在依赖里声明，但 `src/` 中没有对应代码。
- 主题口径不一致：`/ingest/mqtt/protocol` 对外给 `{prefix}/devices/{serial}/{device_type}`，IMU/视觉模块实际订阅 `device/imu/+/data`、`device/vision/+/detect`，remepmobile 又发 `remep/mobile/{imu,vision}/events`。
- 文档落后于代码：`TODO.md` 里 P0 的告警 API、审计日志已实现；`ARCHITECTURE.md`、`docs/CORE_DATA_PROTOCOL_SUMMARY.md` 仍按已删除的 `datasheet` 表描述，现行结构是 `data_streams` / `observations` / `alert_events`。
- `docker-compose.yml` 里 `APP_DATABASE_URL` 与 `APP_JWT_SECRET` 是单下划线，配置层要求 `APP__`，两条覆盖都不生效；JWT 密钥等于仍用默认占位串。
- 可靠性还是 MVP：没有死信队列、背压与消费重试，解析失败的原始包可查但不会重放。
- 数据范围只落地 `all` / `self`。
- `init.sql` 是早期手工 schema，与新环境无关，建库只走 `migrations/`。

## 文档索引

- [ARCHITECTURE.md](ARCHITECTURE.md) — 架构
- [DEPLOY.md](DEPLOY.md) — 部署
- [docs/MQTT_DATA_GUIDE.md](docs/MQTT_DATA_GUIDE.md) — MQTT 数据
- [docs/bed.md](docs/bed.md) — 床垫协议
- [TODO.md](TODO.md) — 待办
