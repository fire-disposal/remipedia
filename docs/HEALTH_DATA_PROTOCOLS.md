# Remipedia 健康数据封装协议分析

> 分析日期：2026-04-25
> 项目：Remipedia IoT Health Platform (Rust/Rocket)

---

## 一、概览：五层封装协议架构

整个健康数据从 **设备端采集 → 平台入库 → API查询输出** 共经过 **5层封装**，每层都有独立的协议定义：

```
┌──────────────────────────────────────────────────────────┐
│   Layer 1: 设备传输层 (Ingest Modules)                    │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│   │ 床垫TCP   │  │ 视觉MQTT  │  │ IMU MQTT  │             │
│   │ Msgpack   │  │ JSON     │  │ JSON      │             │
│   └────┬─────┘  └────┬─────┘  └────┬─────┘              │
│        │              │              │                    │
├────────┼──────────────┼──────────────┼────────────────────┤
│   Layer 2: Core Entity (DataPoint)                       │
│   ┌────────────────────────────────────────────────────┐  │
│   │    统一 DataPoint 结构（Rust struct）                │  │
│   └────────────────────┬───────────────────────────────┘  │
│                        │                                   │
├────────────────────────┼───────────────────────────────────┤
│   Layer 3: Repository (数据库持久化)                      │
│   ┌────────────────────────────────────────────────────┐  │
│   │    Datasheet 表 / ingest_raw_data 表                │  │
│   └────────────────────┬───────────────────────────────┘  │
│                        │                                   │
├────────────────────────┼───────────────────────────────────┤
│   Layer 4: Service (业务逻辑)                              │
│   ┌────────────────────────────────────────────────────┐  │
│   │    DataService / IngestRawService / ServiceConverter │  │
│   └────────────────────┬───────────────────────────────┘  │
│                        │                                   │
├────────────────────────┼───────────────────────────────────┤
│   Layer 5: API (DTO 请求/响应)                            │
│   ┌────────────────────────────────────────────────────┐  │
│   │    HTTP JSON API (Rocket + Serde)                  │  │
│   └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

---

## 二、Layer 1 — 设备传输层协议（Ingest Modules）

### 2.1 模块注册机制

所有模块均实现 [`IngestModule`](src/ingest/modules/mod.rs:16) trait：

```rust
#[async_trait]
pub trait IngestModule {
    async fn start(&self, pool: &PgPool) -> AppResult<()>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
```

通过 [`ModuleRegistry`](src/ingest/modules/mod.rs:28) 统一注册管理。

| 模块名 | name() | 传输协议 | 序列化 | 端口 |
|--------|--------|----------|--------|------|
| 床垫 | `mattress_tcp` | TCP | Msgpack | 9001 |
| 视觉 | `vision_mqtt` | MQTT | JSON | 1883 |
| IMU | `imu_mqtt` | MQTT | JSON | 1883 |

---

### 2.2 智能床垫 TCP + Msgpack 协议

**文件**: [`src/ingest/modules/mattress.rs`](src/ingest/modules/mattress.rs:1)

#### 传输层
- **协议**: 原始 TCP，自主帧协议
- **帧格式**: `[0xAB, 0xCD, len, crc, data...]`
  - Magic 头: `0xAB 0xCD`（2字节）
  - 数据长度: 1字节 (`len`)
  - CRC校验: 1字节
  - 负载: `data` (Msgpack 编码)
- **帧提取**: [`extract_msgpack_frame()`](src/ingest/modules/mattress.rs:196)
- **帧恢复**: [`find_next_magic()`](src/ingest/modules/mattress.rs:231)

#### 消息体（Msgpack → JSON 解码后字段）

| 字段 | Msgpack Key | 别名字段 | 类型 | 说明 |
|------|-------------|----------|------|------|
| serial_number | `sn` | `serial_number` | String | **必填** 设备序列号 |
| manufacturer | `ma` | `manufacturer` | String | 厂商 |
| model | `md` | `model` | String | 型号 |
| firmware_version | `fv` | `firmware_version` | String | 固件版本 |
| status | `status` | - | u8 | 0=离床, 1=上床 |
| heart_rate | `heart_rate` | - | u8 | 心率 (bpm) |
| breath_rate | `breath_rate` | - | u8 | 呼吸率 (次/分) |
| wet_status | `wet_status` | - | u8 | 湿度状态 |
| apnea_count | `apnea_count` | - | u8 | 呼吸暂停次数 |
| weight_value | `weight_value` | - | u16 | 体重值 |
| position | `position` | - | u8 | 体位 |
| ts | `ts` | - | i64 | 时间戳 |

#### 派生事件类型

通过 [`process_mattress_data()`](src/ingest/modules/mattress.rs:264) 生成：

| data_type | 分类 | Severity | 触发条件 |
|-----------|------|----------|----------|
| `mattress_metric` | Metric | - | 每次数据上报（基础指标） |
| `on_bed` | Event | Info | status 0→1 |
| `off_bed` | Event | Info | status 1→0 |
| `heart_rate_high` | Event | Warning | HR > 120 |
| `heart_rate_low` | Event | Warning | HR < 50 |
| `apnea_detected` | Event | Alert | apnea_count > 0 |

#### 业务逻辑

- **床上/离床事件**: 记录卧床时长
- **心率异常**: 高/低双阈值检测（120/50 bpm）
- **呼吸暂停**: 直接告警
- **体位变化**: 跟踪但不生成事件

---

### 2.3 视觉识别 MQTT + JSON 协议

**文件**: [`src/ingest/modules/vision.rs`](src/ingest/modules/vision.rs:1)

#### 传输层
- **协议**: MQTT (rumqttc)
- **主题模式**: `device/vision/{device_id}/detect`
- **QoS**: AtLeastOnce (QoS 1)
- **数据格式**: JSON

#### 消息体字段

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| event_type | String | **是** | 事件类型: fall/wander/visitor/abnormal_behavior |
| timestamp | i64 | 否 | Unix时间戳 |
| confidence | f64 | 否 | 置信度 0.0~1.0 |
| location | String | 否 | 位置描述，默认 "unknown" |
| person_id | String | 否 | 识别到的人员ID |
| image_url | String | 否 | 截图URL |
| metadata | Value | 否 | 扩展元数据 |

#### 事件映射

| event_type | data_type | Severity | 说明 |
|------------|-----------|----------|------|
| `fall` | `vision_fall` | Alert | 跌倒检测 |
| `wander` | `vision_wander` | Warning | 徘徊/走失 |
| `visitor` | `vision_visitor` | Info | 访客识别 |
| `abnormal_behavior` | `vision_abnormal_behavior` | Warning | 异常行为 |
| 其他 | `vision_{event_type}` | Info | 默认 |

#### 附加逻辑
- 低置信度（< 0.7）生成 [`vision_low_confidence`](src/ingest/modules/vision.rs:280) 警告事件

---

### 2.4 IMU 传感器 MQTT + JSON 协议

**文件**: [`src/ingest/modules/imu.rs`](src/ingest/modules/imu.rs:1)

#### 传输层
- **协议**: MQTT (rumqttc)
- **主题模式**: `device/imu/{device_id}/data`
- **QoS**: AtLeastOnce (QoS 1)
- **数据格式**: JSON

#### 消息体字段

| 字段 | 子字段 | 类型 | 必需 | 说明 |
|------|--------|------|------|------|
| timestamp | - | i64 | 否 | Unix时间戳 |
| accelerometer | x | f64 | **是** | X轴加速度 (m/s²) |
| accelerometer | y | f64 | **是** | Y轴加速度 (m/s²) |
| accelerometer | z | f64 | **是** | Z轴加速度 (m/s²) |
| gyroscope | x | f64 | **是** | X轴角速度 (deg/s) |
| gyroscope | y | f64 | **是** | Y轴角速度 (deg/s) |
| gyroscope | z | f64 | **是** | Z轴角速度 (deg/s) |
| battery | - | u8 | 否 | 电池电量 (%) |

#### 跌倒检测算法

通过 [`ImuState`](src/ingest/modules/imu.rs:61) 维护每个设备的状态机：

**三步检测流程**:

1. **冲击检测** (Fall Impact)
   - 计算加速度模: `sqrt(x² + y² + z²)`
   - 阈值: 25.0 m/s² (≈2.5g)
   - 触发: `imu_fall_impact` (Alert)

2. **静止确认** (Fall Confirmed)
   - 计算加速度方差（10个历史窗口）
   - 静止方差阈值: 0.5
   - 静止持续 >= 2秒 → `imu_fall_confirmed` (Alert)

3. **取消检测** (Fall Cancelled)
   - 静止中检测到活动 → 取消跌倒状态
   - 事件: `imu_fall_cancelled` (Info)

| data_type | 分类 | Severity | 说明 |
|-----------|------|----------|------|
| `imu_sensor` | Metric | - | 基础传感器数据（加速度模） |
| `imu_fall_impact` | Event | Alert | 跌倒冲击（高加速度） |
| `imu_fall_confirmed` | Event | Alert | 跌倒确认（冲击后静止≥2秒） |
| `imu_fall_cancelled` | Event | Info | 跌倒取消（恢复活动） |
| `imu_low_battery` | Event | Warning | 低电量 (< 20%) |

---

### 2.5 原始数据归档（通用）

所有三个模块均使用 [`archive_raw()`](src/repository/raw_data.rs:119) 将原始协议字节存入 [`ingest_raw_data`](init.sql:243) 表：

| 字段 | 说明 |
|------|------|
| source | 数据来源: `mattress_tcp` / `vision_mqtt` / `imu_mqtt` |
| raw_payload | 原始字节 (BYTEA) |
| raw_payload_text | UTF-8文本表示（如可解码） |
| remote_addr | 来源地址（TCP: 对端地址 / MQTT: 主题） |
| status | 处理状态: `stored` → `ingested` / `format_error` / `processing_error` |

---

## 三、Layer 2 — Core Entity 数据模型

### 3.1 [`DataPoint`](src/core/entity/datasheet.rs:143) — 统一数据点（内存模型）

这是**所有数据流的统一入口结构**，从 ingest 模块到 repository 都使用此结构：

```rust
pub struct DataPoint {
    pub time: DateTime<Utc>,
    pub device_id: Option<Uuid>,
    pub patient_id: Option<Uuid>,
    pub data_type: String,          // 见 3.4 数据字典
    pub data_category: DataCategory, // Metric / Event
    pub value_numeric: Option<f64>,
    pub value_text: Option<String>,
    pub severity: Option<Severity>,  // Info / Warning / Alert
    pub status: Option<EventStatus>, // Active / Acknowledged / Resolved
    pub payload: serde_json::Value,  // 任意 JSON 负载
    pub source: String,              // "mqtt" / "http" / "mattress_tcp" / "vision_mqtt" / "imu_mqtt"
}
```

#### Builder 方法
- [`DataPoint::metric()`](src/core/entity/datasheet.rs:159) — 快速构造指标数据
- [`DataPoint::event()`](src/core/entity/datasheet.rs:182) — 快速构造事件数据
- 链式调用: `.with_numeric()`, `.with_text()`, `.with_status()`

### 3.2 [`Datasheet`](src/core/entity/datasheet.rs:104) — 数据库持久化实体

对应 `datasheet` 表，枚举字段序列化为字符串：

| Rust 字段 | 数据库列 | SQL 类型 | 约束 |
|-----------|----------|----------|------|
| time | time | TIMESTAMPTZ | PK (复合) |
| device_id | device_id | UUID | PK (复合), FK→device |
| patient_id | patient_id | UUID | FK→patient, 触发器自动填充 |
| data_type | data_type | TEXT | - |
| data_category | data_category | TEXT | CHECK: metric/event |
| value_numeric | value_numeric | DECIMAL(10,4) | - |
| value_text | value_text | TEXT | - |
| severity | severity | TEXT | CHECK: info/warning/alert |
| status | status | TEXT | CHECK: active/acknowledged/resolved |
| payload | payload | JSONB | - |
| source | source | TEXT | 默认 'mqtt' |
| ingested_at | ingested_at | TIMESTAMPTZ | 默认 NOW() |

**数据库触发器** [`auto_fill_patient_id()`](init.sql:299):
> 当 `patient_id` 为 NULL 时，自动从 `binding` 表根据 `device_id` 查找当前活跃绑定填充。

### 3.3 枚举值对象

| 枚举 | 定义位置 | 值 |
|------|----------|----|
| [`DataCategory`](src/core/entity/datasheet.rs:10) | datasheet.rs | `Metric` (metric), `Event` (event) |
| [`Severity`](src/core/entity/datasheet.rs:41) | datasheet.rs | `Info` (info), `Warning` (warning), `Alert` (alert) |
| [`EventStatus`](src/core/entity/datasheet.rs:73) | datasheet.rs | `Active` (active), `Acknowledged` (acknowledged), `Resolved` (resolved) |
| [`RawIngestStatus`](src/core/entity/raw_data.rs:8) | raw_data.rs | `Stored`, `Ingested`, `Ignored`, `FormatError`, `ProcessingError` |
| [`DataType`](src/core/value_object/data_type.rs:7) | data_type.rs | `HeartRate`, `FallEvent`, `SpO2`, `MattressStatus`, `TurnOverEvent`, `BedEntryEvent`, `BedExitEvent`, `SignificantMovementEvent`, `MeasurementSnapshot` |
| [`DeviceType`](src/core/value_object/device_type.rs:7) | device_type.rs | `HeartRateMonitor`, `FallDetector`, `SmartMattress` |

### 3.4 完整 data_type 字典

由所有 ingest 模块和 API 共同定义的完整 `data_type` 值集合：

**床垫设备 (smart_mattress)**
| data_type | 分类 | 来源 |
|-----------|------|------|
| `mattress_metric` | Metric | 床垫模块 |
| `on_bed` | Event | 床垫模块 |
| `off_bed` | Event | 床垫模块 |
| `heart_rate_high` | Event | 床垫模块 |
| `heart_rate_low` | Event | 床垫模块 |
| `apnea_detected` | Event | 床垫模块 |

**视觉设备 (vision_camera)**
| data_type | 分类 | 来源 |
|-----------|------|------|
| `vision_fall` | Event | 视觉模块 |
| `vision_wander` | Event | 视觉模块 |
| `vision_visitor` | Event | 视觉模块 |
| `vision_abnormal_behavior` | Event | 视觉模块 |
| `vision_low_confidence` | Event | 视觉模块 |
| `vision_{event_type}` | Event | 视觉模块（动态） |

**IMU传感器 (imu_sensor)**
| data_type | 分类 | 来源 |
|-----------|------|------|
| `imu_sensor` | Metric | IMU模块 |
| `imu_fall_impact` | Event | IMU模块 |
| `imu_fall_confirmed` | Event | IMU模块 |
| `imu_fall_cancelled` | Event | IMU模块 |
| `imu_low_battery` | Event | IMU模块 |

**HTTP API 通用**
| data_type | 分类 | 来源 |
|-----------|------|------|
| 任意字符串 | Metric（默认） | HTTP API 上报 |

### 3.5 数据查询模型

**用于数据库过滤的统一查询结构** [`DataQuery`](src/core/entity/datasheet.rs:224):

```rust
pub struct DataQuery {
    pub patient_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub data_type: Option<String>,
    pub data_category: Option<DataCategory>,
    pub severity: Option<Severity>,
    pub status: Option<EventStatus>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub page: u32,
    pub page_size: u32,
}
```

---

## 四、Layer 3 — Repository 持久化层

### 4.1 [`DataRepository`](src/repository/data.rs:7)

| 方法 | 说明 | SQL |
|------|------|-----|
| `insert_datapoint()` | 单条插入 | INSERT INTO datasheet ... RETURNING * |
| `insert_datapoints()` | 批量插入（事务） | 循环 INSERT ... RETURNING * |
| `query()` | 统一查询（8个可选过滤条件） | SELECT ... WHERE $1..$8 |
| `count()` | 计数 | SELECT COUNT(*) |
| `query_active_alerts()` | 活跃告警（按severity排序） | WHERE category='event' AND status='active' |
| `find_latest_by_patient()` | 患者最新数据 | ORDER BY time DESC LIMIT |
| `acknowledge_event()` | 确认事件 | UPDATE SET status='acknowledged' |
| `resolve_event()` | 解决事件 | UPDATE SET status='resolved' |
| `get_stats()` | 统计信息 | 4个 COUNT(*)...FILTER |

### 4.2 [`RawDataRepository`](src/repository/raw_data.rs:8)

| 方法 | 说明 |
|------|------|
| `archive_raw()` | 归档原始字节（自动尝试 UTF-8 解码） |
| `mark_status()` | 更新处理状态（stored→ingested/format_error） |
| `query()` | 分页查询原始归档 |
| `count()` | 计数 |
| `get_by_id()` | 根据ID获取详情 |

---

## 五、Layer 4 — Service 业务逻辑层

### 5.1 [`DataService`](src/service/data.rs:12)

数据服务，负责 DTO → Entity 的转换和业务编排：

| 方法 | 输入 | 输出 | 说明 |
|------|------|------|------|
| `ingest()` | 各字段 | `DataReportResponse` | 直接构造 DataPoint 入库 |
| `report_http()` | `DataReportRequest` | `DataReportResponse` | 验证设备→填充patient_id→入库 |
| `query()` | `DataQuery` (DTO) | `DataQueryResponse` | 转 CoreDataQuery → 查询 → 转 Response |
| `query_alerts()` | `AlertQuery` (DTO) | `DataQueryResponse` | 固定 category=Event |
| `get_alert_stats()` | patient_id | `AlertStatsResponse` | 统计信息 |
| `acknowledge_event()` | 参数 | `DataRecordResponse` | 确认事件 |
| `resolve_event()` | 参数 | `DataRecordResponse` | 解决事件 |
| `get_latest_by_patient()` | 参数 | Vec\<DataRecordResponse\> | 最新数据 |

**Entity→Response 转换**: 通过 [`From<Datasheet> for DataRecordResponse`](src/service/data.rs:209) 实现。

### 5.2 [`IngestRawService`](src/service/ingest_raw.rs:10)

| 方法 | 说明 |
|------|------|
| `query()` | 原始数据分页查询 |
| `get_detail()` | 原始数据详情（含 Base64/Hex 编码） |
| `export_csv()` | 导出 CSV |

**转换链路**:
- `RawDataRecord` → `RawDataRecordResponse`（含payload预览）
- `RawDataRecord` → `RawDataDetailResponse`（含 Base64 + Hex + 完整文本）

### 5.3 [`ServiceConverter`](src/service/converter.rs:13)

```rust
pub struct ServiceConverter<'a> {
    role_repo: RoleRepository<'a>,
}
```

用于角色名称的批量转换查询，与健康数据协议无直接关系。

---

## 六、Layer 5 — API DTO 请求/响应协议

### 6.1 HTTP API 端点总览

| 方法 | 路径 | 请求体/参数 | 响应 | 说明 |
|------|------|-------------|------|------|
| POST | `/data` | `DataReportRequest` | `DataReportResponse` | 数据上报 |
| GET | `/data` | Query params | `DataQueryResponse` | 数据查询 |
| GET | `/data/alerts` | Query params | Vec\<DataRecordResponse\> | 活跃告警 |
| POST | `/data/events/acknowledge` | `AcknowledgeRequest` | `DataRecordResponse` | 确认事件 |
| POST | `/data/events/resolve` | `ResolveRequest` | `DataRecordResponse` | 解决事件 |
| GET | `/ingest/mqtt/protocol` | - | `MqttProtocolDoc` | MQTT协议文档 |
| GET | `/ingest/raw` | Query params | `RawDataQueryResponse` | 原始数据查询 |
| GET | `/ingest/raw/export` | Query params | bytes | 原始数据导出 |
| GET | `/ingest/raw/{id}` | Path param | `RawDataDetailResponse` | 原始数据详情 |
| GET | `/health` | - | JSON | 健康检查 |
| GET | `/ready` | - | JSON | 就绪检查 |
| GET | `/live` | - | JSON | 存活检查 |

### 6.2 请求 DTO

**数据上报** [`DataReportRequest`](src/dto/request/data.rs:8):

```json
{
    "timestamp": "2026-04-02T00:00:00Z",   // 可选，默认now
    "device_id": "uuid",                     // 必填
    "patient_id": "uuid",                    // 可选，否则自动绑定
    "data_type": "heart_rate",               // 必填
    "payload": { "value": 72 }               // 必填，任意JSON
}
```

**数据查询** [`DataQuery`](src/dto/request/data.rs:52):
- 8个可选过滤参数 + 分页（默认 page=1, page_size=20）
- 筛选维度: patient_id, device_id, data_type, data_category, severity, status, start_time, end_time

**告警查询** [`AlertQuery`](src/dto/request/data.rs:108):
- 默认 status=active（仅查活跃告警）

**原始数据查询** [`RawDataQuery`](src/dto/request/data.rs:144):
- 筛选: source, serial_number, device_type, status, time range

### 6.3 响应 DTO

**数据记录** [`DataRecordResponse`](src/dto/response/data.rs:23):

```json
{
    "time": "2026-04-02T00:00:00Z",
    "device_id": "uuid",
    "patient_id": "uuid",
    "data_type": "heart_rate",
    "data_category": "metric",
    "value_numeric": 72.0,
    "value_text": null,
    "severity": null,
    "status": null,
    "payload": { "value": 72 },
    "source": "mqtt",
    "ingested_at": "2026-04-02T00:00:01Z"
}
```

**原始数据详情** [`RawDataDetailResponse`](src/dto/response/data.rs:129):

```json
{
    "id": "uuid",
    "source": "mattress_tcp",
    "status": "ingested",
    "payload_size": 128,
    "raw_payload_base64": "qEP///...",       // 原始字节 Base64
    "raw_payload_text": "{...}",               // UTF-8解码
    "raw_payload_hex": "ab cd ef...",          // 十六进制表示
    // ...
}
```

**MQTT协议文档** [`MqttProtocolDoc`](src/api/routes/ingest.rs:14):

```json
{
    "protocol": "mqtt",
    "version": "v2",
    "topic_pattern": "devices/{serial_number}/{device_type}",
    "qos": "at_least_once (QoS1)",
    "payload_required_fields": ["timestamp (RFC3339)", "value 或 data"],
    "sample_topic": "devices/SN-001/heart_rate_monitor",
    "sample_payload": {
        "timestamp": "2026-04-02T00:00:00Z",
        "device_type": "heart_rate_monitor",
        "value": 72,
        "metadata": { "firmware": "1.0.0" }
    }
}
```

---

## 七、完整数据流示例

### 7.1 床垫数据：设备 → 入库 → 查询

```
[床垫设备] 
  TCP发送: 0xAB 0xCD 0x2A CRC [Msgpack编码数据]
     │
     ▼
[mattress.rs]
  1. extract_msgpack_frame() → 提取完整帧
  2. archive_raw() → 存入 ingest_raw_data (status=stored)
  3. parse_mattress_packet() → Msgpack→JSON解析 → MattressPacket
  4. resolve_or_create_device() → 自动注册设备
  5. process_mattress_data() → 生成 Vec<DataPoint>
     ├─ mattress_metric (Metric)
     ├─ on_bed (Event) [如果状态变化]
     └─ heart_rate_high (Event, Warning) [如果超阈值]
  6. insert_datapoints() → 批量写入 datasheet 表 (status=ingested)
  7. mark_status() → 更新 ingest_raw_data (status=ingested)
     │
     ▼
[PostgreSQL datasheet 表]
  auto_fill_patient_id 触发器自动填充 patient_id
     │
     ▼
[HTTP API GET /data]
  DataService.query() → DataRecordResponse[]
```

### 7.2 通用数据转换链

```
外部协议字节 (Msgpack/JSON/自定义帧)
  │ archive_raw()
  ▼
RawDataRecord (原始字节归档)
  │ parse_xxx()
  ▼
ImuData / MattressPacket / VisionDetection (协议专用结构)
  │ process_xxx_data()
  ▼
Vec<DataPoint> (统一入口结构)
  │ insert_datapoints()
  ▼
Datasheet (数据库持久化)
  │ DataService.query() → From trait
  ▼
DataRecordResponse (API 响应)
```

---

## 八、协议设计模式总结

| 层次 | 封装模式 | 关键结构 | 序列化 |
|------|----------|----------|--------|
| 传输层 | 模块化隔离（每个协议独立模块） | ImuModule/MattressModule/VisionModule | Msgpack/JSON |
| 实体层 | 统一DataPoint + 枚举值对象 | DataPoint, DataCategory, Severity, EventStatus | Rust struct |
| 持久化层 | 查询DSL模式（可选参数注入） | DataQuery, RawDataQuery | SQL |
| 服务层 | DTO↔Entity 转换 + 业务编排 | DataService, IngestRawService | From trait |
| API层 | RESTful + OpenAPI | DataReportRequest/Response 等 | Serde JSON |

### 关键设计决策

1. **统一 datasheet 表**: 指标(Metric)和事件(Event)共用一张时间序列表，通过 `data_category` 区分
2. **Patient ID 自动填充**: 通过数据库触发器自动从活跃绑定获取，减少业务代码负担
3. **原始数据归档**: 所有 ingest 模块统一归档原始字节，支持事后审计和调试
4. **自动设备注册**: 三个 ingest 模块均实现 `resolve_or_create_device()`，首次上报自动创建设备
5. **字符串枚举**: 所有枚举（category/severity/status）在数据库中存为字符串，方便查询和扩展
6. **JSONB payload**: 所有额外数据存入 JSONB，支持灵活的 schema-less 数据
