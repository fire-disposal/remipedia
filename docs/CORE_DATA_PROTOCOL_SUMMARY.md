# Remipedia 健康数据核心封装格式（技术报告用）

---

## 一、核心思想：Metric-Event 统一模型

平台将所有健康数据抽象为 **两类基本数据单元**，共用同一结构：

| 类别 | 标识 | 语义 | 示例 |
|------|------|------|------|
| **Metric（指标）** | `data_category = "metric"` | 连续测量的数值型健康指标 | 心率、呼吸率、加速度模 |
| **Event（事件）** | `data_category = "event"` | 离散触发的告警或状态变更 | 跌倒检测、离床上床、心率异常 |

> 这种设计借鉴了 **时序数据库（TSDB）** 的 metric/event 双模型思想，但整合为一张统一表，简化了查询和存储。

---

## 二、统一数据封装结构（核心）

### 内存模型 — [`DataPoint`](src/core/entity/datasheet.rs:143)

```
DataPoint
├── time:          DateTime<Utc>        ← 数据时间戳（必填）
├── device_id:     Option<Uuid>         ← 设备ID（必填，自动注册）
├── patient_id:    Option<Uuid>         ← 患者ID（自动填充，解耦业务）
├── data_type:     String               ← 数据类型标签（见下方字典）
├── data_category: DataCategory         ← Metric | Event（二维分类）
├── value_numeric: Option<f64>          ← 数值（Metric 主值 / Event 辅助值）
├── value_text:    Option<String>       ← 文本描述
├── severity:      Option<Severity>     ← Event: Info | Warning | Alert
├── status:        Option<EventStatus>  ← Event: Active | Acknowledged | Resolved
├── payload:       serde_json::Value    ← JSONB 扩展负载（schema-less）
└── source:        String               ← 数据来源: mqtt | http | mattress_tcp | ...
```

### 数据库模型 — [`datasheet`](init.sql:269)

PostgreSQL 时间序列表，**主键为 (time, device_id)**，含两处关键设计：

```
CREATE TABLE datasheet (
    time            TIMESTAMPTZ,          -- PK (时间-设备复合主键)
    device_id       UUID,                 -- PK
    patient_id      UUID,                 -- FK→patient, 触发器自动填充
    data_type       TEXT,
    data_category   TEXT,                 -- CHECK: 'metric' | 'event'
    value_numeric   DECIMAL(10,4),
    value_text      TEXT,
    severity        TEXT,                 -- CHECK: 'info' | 'warning' | 'alert'
    status          TEXT,                 -- CHECK: 'active' | 'acknowledged' | 'resolved'
    payload         JSONB,               -- 任意扩展数据
    source          TEXT,                 -- 默认 'mqtt'
    ingested_at     TIMESTAMPTZ,          -- 入库时间
    PRIMARY KEY (time, device_id)
);
```

**两条设计规则**：
1. **Patient ID 自动填充触发器**：插入时若 `patient_id` 为 NULL，自动从 `binding` 表查找设备当前绑定患者
2. **枚举存字符串**：`category`/`severity`/`status` 全部存为 TEXT + CHECK 约束，而非整数枚举，方便 SQL 查询和外部系统对接

---

## 三、数据流转全过程

```
┌──────────────┐    ┌────────────────┐    ┌────────────────┐    ┌──────────────┐
│  设备协议     │    │  模块解析       │    │  统一入库       │    │  API 输出     │
│  Msgpack/JSON │───→│  parse + process│───→│  DataPoint     │───→│  JSON 响应    │
│  自定义帧     │    │  + 业务逻辑     │    │  → datasheet表  │    │               │
└──────────────┘    └────────────────┘    └────────────────┘    └──────────────┘
                           │                       │
                           ▼                       ▼
                    原始字节归档              存档+审计
                    ingest_raw_data          可直接查询
```

### 三步转换链

| 步骤 | 操作 | 位置 |
|------|------|------|
| **① 协议解析** | 设备特有格式 → `ImuData`/`MattressPacket`/`VisionDetection` | 各 ingest 模块 |
| **② 业务处理** | 状态机分析 → `Vec<DataPoint>`（打上 data_type/severity 标签） | `process_xxx_data()` |
| **③ 持久化** | `DataPoint` → SQL INSERT → `Datasheet`（数据库行） | `DataRepository` |

---

## 四、Enum 值域定义

### 分类维度

```
DataCategory ─┬─ Metric  (metric)  ← 连续数值流
              └─ Event   (event)   ← 离散事件流
```

### 事件属性

```
Severity ─┬─ Info    ← 普通信息（如上门/离床）
          ├─ Warning ← 警告（如心率高/低电量）
          └─ Alert   ← 紧急告警（如跌倒/呼吸暂停）

EventStatus ─┬─ Active        ← 未处理
             ├─ Acknowledged  ← 已确认
             └─ Resolved      ← 已解决
```

---

## 五、data_type 全量字典（技术报告版）

| data_type | 分类 | Severity | 来源设备 | 说明 |
|-----------|------|----------|----------|------|
| `mattress_metric` | Metric | - | 床垫 | 心率/呼吸/体重/体位 |
| `on_bed` | Event | Info | 床垫 | 上床事件 |
| `off_bed` | Event | Info | 床垫 | 离床事件（含卧床时长） |
| `heart_rate_high` | Event | Warning | 床垫 | HR > 120 bpm |
| `heart_rate_low` | Event | Warning | 床垫 | HR < 50 bpm |
| `apnea_detected` | Event | Alert | 床垫 | 呼吸暂停 |
| `imu_sensor` | Metric | - | IMU | 三维加速度+陀螺仪 |
| `imu_fall_impact` | Event | Alert | IMU | 跌倒冲击（>25 m/s²） |
| `imu_fall_confirmed` | Event | Alert | IMU | 跌倒确认（冲击+静止≥2s） |
| `imu_fall_cancelled` | Event | Info | IMU | 跌倒取消 |
| `imu_low_battery` | Event | Warning | IMU | 电池 < 20% |
| `vision_fall` | Event | Alert | 视觉 | 视觉跌倒检测 |
| `vision_wander` | Event | Warning | 视觉 | 徘徊/走失 |
| `vision_visitor` | Event | Info | 视觉 | 访客识别 |
| `vision_abnormal_behavior` | Event | Warning | 视觉 | 异常行为 |
| `vision_low_confidence` | Event | Warning | 视觉 | 置信度 < 0.7 |
| (用户自定义) | Metric | - | HTTP API | 任意 data_type 字符串 |

---

## 六、设计要点

1. **统一入口**：所有设备协议（床垫/视觉/IMU）最终映射为同一 `DataPoint` 结构，上层无需关心设备差异
2. **原始数据保留**：每个 ingest 模块在解析前先将原始字节存入 `ingest_raw_data`，支持事后审计和调试
3. **自动设备注册**：首次收到设备数据时自动创建设备记录，无需预先配置
4. **扩展性**：`payload` 字段为 JSONB，支持任意结构化扩展而不改表结构
