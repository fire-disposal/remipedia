# Remipedia Backend MVP 极简收敛架构设计方案

## 设计原则 (MVP)
*   去中间件化：不引入新的消息队列或规则引擎中间件
*   去状态机：放弃复杂的事件流转和 ACK 确认状态机
*   以数据库为核心：依赖 PostgreSQL 强大的 SQL 能力和单点事务一致性
*   极简闭环：用最少、侵入性最小的代码完成业务在后端的流转与前端触达

---

## 1. 告警系统：激进统一化收敛 (Radical Unified Alerts)

**核心设计点：一切异常皆为一行数据，放弃复杂流处理。**

### 数据模型
不管触发源是设备离在线还是具体业务指标异常，统统作为通用告警事件存入 `alerts` 单表。

```sql
CREATE TABLE alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL,               -- 关联患者，方便家属/医生直接按人查询
    device_id UUID,                         -- 触发告警的设备 (可为空，防系统级告警)
    alert_type VARCHAR(50) NOT NULL,        -- 例如: 'fall'(跌倒), 'leave_bed'(离床), 'offline'(掉线)
    level VARCHAR(20) NOT NULL,             -- 严重等级: 'critical'(红), 'warning'(黄), 'info'(蓝)
    payload JSONB,                          -- 冗余原始特征数据/元数据
    status VARCHAR(20) DEFAULT 'unhandled', -- 'unhandled'(未处理), 'resolved'(已解决)
    resolved_by UUID,                       -- 处理人 (关联 users.id)
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 核心查询索引：加速前端轮询
CREATE INDEX idx_alerts_unhandled ON alerts(patient_id) WHERE status = 'unhandled';
```

### 生产与触达 (短轮询规避长连接)
1.  **业务事件（如跌倒）生产**：在 Ingest 层 (`pipeline.rs` 或 `adapter.rs`) 解析到 payload 时直接 `INSERT INTO alerts`。
2.  **系统事件（如掉线）生产**：`main.rs` 中通过 `tokio::time::interval` 启动异步后台任务，定期扫表 (`updated_at` < NOW - 5 mins)，转 Offline 并 `INSERT` 掉线告警。
3.  **前端触达**：放弃 WebSocket/SSE 开发维护成本，直接由前端执行 `GET /api/v1/alerts?status=unhandled` 进行 5-10s 的短轮询。

---

## 2. 数据隔离：User-Patient 极简绑定 (Basic Isolation)

**核心设计点：抛弃 OPA 或 ABAC 等重型权限框架，引入基础多对多关联表实现行级视界隔离。**

### 数据模型
```sql
CREATE TABLE user_patient_bindings (
    user_id UUID NOT NULL REFERENCES users(id),
    patient_id UUID NOT NULL REFERENCES patients(id),
    relation_type VARCHAR(20),              -- 关系类型: 'family'(家属), 'doctor'(医生)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, patient_id)
);
```

### 拦截策略
*   普通家属/医生角色在调用所有涉及设备/健康数据的 API 时（列表查/单查），Repository 层强制利用子查询切分可见边界：
    `WHERE patient_id IN (SELECT patient_id FROM user_patient_bindings WHERE user_id = {current_user})`
*   超级管理员无视边界。

---

## 3. 指令下发：移除 ACK 追踪 (Fire-and-Forget)

**核心设计点：弱化反向链路事务性。**

### 交互时序
1.  **用户操作**：前端调用 `POST /api/v1/devices/{id}/command` 发送诸如设备重启、归零请求。
2.  **异步下发**：后端直接包装 payload 并利用现有 MQTT Client (`rumqttc`) 进行 `Publish(topic: remipedia/cmd/downlink/{serial_number})`。
3.  **即时响应**：后端立刻向前端返回 `202 Accepted`，结束请求（不等待设备确认）。
4.  **状态更迭**：系统不记录指令状态流水账。由前端常规 Dashboard 数据轮询的更新自动验证指令是否生效（最终一致性体验）。
