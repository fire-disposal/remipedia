# Remipedia 开发任务清单

> 按实现复杂性与收益排序

---

## 🔴 P0 - 高收益 / 低复杂性（立即行动）

### 0.1 数据库模型修复

- [ ] **修复 datasheet 主键设计**
  - 问题：`(time, device_id)` 主键可能在同一毫秒冲突
  - 方案：改为 `(time, device_id, data_type)` 或使用 `gen_random_uuid()`
  - 复杂度：低

- [ ] **添加 CHECK 约束**
  - 为 `device_type`、`data_type` 添加合法值约束
  ```sql
  ALTER TABLE device ADD CONSTRAINT device_type_check 
    CHECK (device_type IN ('heart_rate_monitor', 'fall_detector', 'spo2_sensor', 'smart_mattress'));
  ```
  - 复杂度：低

### 0.2 核心 API 补充

- [ ] **用户-患者绑定 API**
  - `POST /api/v1/user-patient-bindings` - 创建绑定
  - `GET /api/v1/user-patient-bindings` - 绑定列表
  - `DELETE /api/v1/user-patient-bindings/{id}` - 解除绑定
  - 复杂度：低
  - 收益：核心业务功能

- [ ] **告警 API**
  - `GET /api/v1/alerts` - 告警列表
  - `GET /api/v1/alerts/{id}` - 告警详情
  - `PUT /api/v1/alerts/{id}/acknowledge` - 确认告警
  - `PUT /api/v1/alerts/{id}/resolve` - 解决告警
  - 复杂度：低
  - 收益：核心业务功能

### 0.3 Ingest 层可靠性

- [ ] **死信队列 (DLQ)**
  - 创建 `datasheet_dlq` 表
  - 消息处理失败时存入 DLQ
  - 复杂度：低
  - 收益：防止数据丢失

---

## 🟡 P1 - 高收益 / 中等复杂性（近期规划）

### 1.1 Ingest 层改进

- [ ] **MQTT 主题标准化**
  - 格式：`{prefix}/{serial}/{device_type}/{data_type}`
  - 设备类型从主题提取，不依赖 payload
  - 复杂度：中

- [ ] **消息重试机制**
  - 指数退避重试 3 次
  - 超过阈值后存入 DLQ
  - 复杂度：中

- [ ] **背压与监控**
  - 队列长度监控
  - 队列满时拒绝新消息
  - 复杂度：中

### 1.2 设备管理增强

- [ ] **设备状态 API**
  - `GET /api/v1/devices/{id}/status` - 设备在线状态
  - `GET /api/v1/devices/{id}/history` - 设备历史
  - 复杂度：低

- [ ] **设备命令下发**
  - `POST /api/v1/devices/{id}/command` - 下发命令
  - 复杂度：中

- [ ] **设备最新数据**
  - `GET /api/v1/devices/{id}/data/latest` - 最新数据
  - `GET /api/v1/devices/{id}/data/trend` - 趋势数据
  - 复杂度：低

### 1.3 患者档案

- [ ] **档案更新 API**
  - `PUT /api/v1/patients/{id}/profile` - 更新档案
  - 复杂度：低

- [ ] **患者摘要**
  - `GET /api/v1/patients/{id}/summary` - 数据摘要
  - 复杂度：低

### 1.4 认证增强

- [ ] **角色扩展**
  - 从 Admin/User 扩展为 Doctor/Nurse/Caregiver/Patient
  - 复杂度：中

---

## 🟢 P2 - 高收益 / 高复杂性（中期规划）

### 2.1 数据分析

- [ ] **数据聚合 API**
  - `GET /api/v1/data/aggregate` - 小时/天/周聚合
  - 支持 `time_bucket` 聚合
  - 复杂度：高

- [ ] **数据导出**
  - `GET /api/v1/data/export` - 导出 CSV/Excel
  - 复杂度：中

- [ ] **健康报告**
  - `GET /api/v1/patients/{id}/report` - 患者健康报告
  - 复杂度：高

### 2.2 运维能力

- [ ] **系统配置 API**
  - `GET /api/v1/admin/config` - 获取配置
  - `PUT /api/v1/admin/config` - 更新配置
  - 复杂度：中

- [ ] **审计日志**
  - 记录用户操作
  - `GET /api/v1/admin/logs`
  - 复杂度：中

### 2.3 TCP Transport 改进

- [ ] **连接重连逻辑**
  - 断线自动重连
  - 指数退避
  - 复杂度：中

### 2.4 数据分类存储

- [ ] **原始数据 vs 事件数据分离**
  - 高频数据写入 `raw_data` 表
  - 事件数据写入 `events` 表 + 触发告警
  - 复杂度：高

---

## 🔵 P3 - 长期规划

### 3.1 权限系统

- [ ] **完整 RBAC**
  - 角色管理 API
  - 权限分配
  - 资源级访问控制
  - 复杂度：高

### 3.2 架构演进

- [ ] **DDD 限界上下文重构**
  - Auth / Patient / Device / Care / Data 上下文分离
  - 引入聚合根
  - 复杂度：非常高

- [ ] **TimescaleDB 集成**
  - 将 datasheet 转为 hypertable
  - 数据压缩
  - 保留策略
  - 复杂度：高（需数据迁移）

### 3.3 基础设施

- [ ] **Redis 引入**
  - Session/JWT 缓存
  - 设备状态缓存
  - Pub/Sub（多实例部署）
  - 复杂度：中

- [ ] **Metrics 监控**
  - Prometheus 集成
  - Grafana 仪表盘
  - 复杂度：中

---

## ⚪ 已完成 ✅

- [x] 基础 CRUD API (User/Device/Patient/Binding/Data)
- [x] JWT 认证 (login/refresh/logout)
- [x] 分层架构 (API → Service → Repository)
- [x] MQTT/TCP 数据接入
- [x] 设备适配器框架
- [x] OpenAPI 文档

---

## 📋 实施建议

### 阶段 1：P0（1-2 周）
```
0.1 数据库修复 → 0.2 核心 API → 0.3 死信队列
```

### 阶段 2：P1（2-4 周）
```
1.1 Ingest 可靠性 → 1.2 设备增强 → 1.3 患者档案
```

### 阶段 3：P2（1-2 月）
```
2.1 数据分析 → 2.2 运维能力 → 2.3 数据分类
```

### 阶段 4：P3（按需）
```
根据业务需求选择性实施
```

---

## 📝 备注

- 复杂性评估基于当前团队技术栈熟悉度
- 收益评估基于 IoT 健康平台核心业务价值
- 可根据实际业务优先级调整顺序
