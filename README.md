# Remipedia

Remipedia 是一个基于 Rust 的 IoT 健康数据平台，支持多协议设备数据接入、实时处理和数据管理。

## 功能特性

- **多协议数据接入**：支持 MQTT、TCP、WebSocket 三种协议
- **设备自动注册**：首次上报自动创建设备记录
- **设备适配器框架**：可扩展的设备适配器架构
- **JWT 认证**：基于 access token + refresh token 的认证机制
- **数据管道**：基于队列的异步数据摄入管道
- **OpenAPI 文档**：自动生成 Swagger UI

## 技术栈

- **Web 框架**: Rocket 0.5
- **数据库**: PostgreSQL 16 + SQLx
- **异步运行时**: Tokio
- **MQTT 客户端**: rumqttc
- **认证**: JWT + Argon2
- **API 文档**: utoipa + Swagger UI

## 快速开始

### 本地开发

```bash
# 启动依赖服务
docker-compose up -d postgres mqtt

# 运行应用
cargo run
```

访问 http://localhost:8000/swagger 查看 API 文档。

### 部署

```bash
# 使用 Docker Compose 部署
docker-compose up -d
```

或参考 [DEPLOY.md](DEPLOY.md) 使用 GitHub Actions 自动部署。

## 项目结构

```
src/
├── api/              # HTTP 接口层 (Rocket)
├── service/          # 业务逻辑层
├── repository/       # 数据访问层 (SQLx)
├── core/             # 领域模型
├── ingest/           # 数据接入层 (MQTT/TCP/WebSocket)
├── dto/              # 数据传输对象
└── config/           # 配置管理
```

## 数据接入

支持三种协议的数据接入：

| 协议 | 端口 | 用途 |
|------|------|------|
| MQTT | 1883 | 设备数据上报 |
| TCP | 5858 | 二进制协议设备 |
| WebSocket | 5859 | WebSocket 设备 |

## 配置

通过环境变量覆盖配置（使用双下划线作为分隔符）：

```bash
APP_DATABASE__URL=postgresql://user:pass@host/db
APP_MQTT__BROKER=localhost
APP_MQTT__PORT=1883
```

## 文档

- [DEPLOY.md](DEPLOY.md) - 部署指南
- [ARCHITECTURE.md](ARCHITECTURE.md) - 架构设计文档
- [TODO.md](TODO.md) - 开发任务清单

## 许可证

MIT
