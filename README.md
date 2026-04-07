# Remipedia

Rust IoT健康数据平台，支持MQTT/TCP/WebSocket多协议接入。

## 功能

- 多协议数据接入（MQTT、TCP、WebSocket）
- 设备自动注册
- 设备适配器框架
- JWT认证（access/refresh token）
- 异步数据管道
- OpenAPI/Swagger文档

## 技术栈

**Web框架**: Rocket 0.5  
**数据库**: PostgreSQL 16 + SQLx  
**异步运行时**: Tokio  
**MQTT**: rumqttc  
**认证**: JWT + Argon2  
**API文档**: utoipa + Swagger UI

## 快速开始

```bash
# 启动依赖
docker-compose up -d postgres mqtt

# 运行应用
cargo run
```

访问 http://localhost:8000/swagger 查看API文档。

## 部署

```bash
# 本地Docker部署
docker-compose up -d

# 或使用GitHub Actions自动部署
# 参考 DEPLOY.md
```

## 项目结构

```
src/
api/         HTTP接口层
service/     业务逻辑层
repository/  数据访问层（SQLx）
core/        领域模型
ingest/      数据接入（MQTT/TCP/WebSocket）
dto/         数据传输对象
config/      配置管理
```

## 数据接入

**MQTT**: 1883端口 - 设备数据上报  
**TCP**: 5858端口 - 二进制协议设备  
**WebSocket**: 5859端口 - WebSocket设备

## 配置

环境变量使用双下划线分隔：

```bash
APP_DATABASE__URL=postgresql://user:pass@host/db
APP_MQTT__BROKER=localhost
APP_MQTT__PORT=1883
```

## 文档

- DEPLOY.md - 部署指南
- ARCHITECTURE.md - 架构设计
- TODO.md - 开发任务

## 许可证

MIT
