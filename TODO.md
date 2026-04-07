# 开发任务

## P0 - 立即执行

**数据库修复**
- [ ] 修复datasheet主键: (time, device_id) → (time, device_id, data_type) 或使用UUID
- [ ] 添加CHECK约束: device_type, data_type合法值限制

**核心API补充**
- [ ] 用户-患者绑定: POST/GET/DELETE /api/v1/user-patient-bindings
- [ ] 告警API: GET/PUT /api/v1/alerts, 确认/解决告警

**Ingest可靠性**
- [ ] 死信队列: 创建datasheet_dlq表, 失败消息落盘

## P1 - 近期规划

**Ingest改进**
- [ ] MQTT主题标准化: {prefix}/{serial}/{device_type}/{data_type}
- [ ] 消息重试: 指数退避3次, 超时入DLQ
- [ ] 背压监控: 队列长度监控, 满时拒绝新消息

**设备管理**
- [ ] 设备状态API: /devices/{id}/status, /devices/{id}/history
- [ ] 设备命令下发: POST /devices/{id}/command
- [ ] 设备数据查询: /devices/{id}/data/latest, /devices/{id}/data/trend

**患者档案**
- [ ] 档案更新: PUT /patients/{id}/profile
- [ ] 患者摘要: GET /patients/{id}/summary

**认证增强**
- [ ] 角色扩展: Admin/User → Doctor/Nurse/Caregiver/Patient

## P2 - 中期规划

**数据分析**
- [ ] 数据聚合: GET /data/aggregate (小时/天/周)
- [ ] 数据导出: GET /data/export (CSV/Excel)
- [ ] 健康报告: GET /patients/{id}/report

**运维能力**
- [ ] 系统配置API: GET/PUT /admin/config
- [ ] 审计日志: GET /admin/logs

**TCP改进**
- [ ] 连接重连: 断线自动重连 + 指数退避

**数据分类**
- [ ] 原始/事件分离: raw_data表 + events表

## P3 - 长期规划

**权限系统**
- [ ] 完整RBAC: 角色管理 + 权限分配 + 资源级控制

**架构演进**
- [ ] DDD重构: 限界上下文分离
- [ ] TimescaleDB: datasheet转hypertable + 压缩

**基础设施**
- [ ] Redis: Session缓存 + 设备状态 + Pub/Sub
- [ ] 监控: Prometheus + Grafana

## 已完成

- [x] 基础CRUD API
- [x] JWT认证
- [x] 分层架构
- [x] MQTT/TCP数据接入
- [x] 设备适配器框架
- [x] OpenAPI文档

## 实施计划

**阶段1 (P0)**: 数据库修复 → 核心API → 死信队列

**阶段2 (P1)**: Ingest可靠性 → 设备增强 → 患者档案

**阶段3 (P2)**: 数据分析 → 运维能力 → 数据分类

**阶段4 (P3)**: 按需实施
