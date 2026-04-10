# MQTT 数据传入指南

## 1. MQTT Broker 连接信息

### 生产环境 (yecaoyun)

| 参数 | 值 | 说明 |
|------|-----|------|
| **Broker 地址** | `68.64.178.247` 或 `yecaoyun` | 服务器公网IP/主机名 |
| **端口** | `1883` | 标准MQTT端口（非TLS） |
| **协议** | `mqtt://` | 明文MQTT协议 |
| **完整连接地址** | `mqtt://68.64.178.247:1883` | 设备连接使用 |
| **Client ID** | 建议设备序列号 | 每个设备唯一标识 |
| **Keep Alive** | 30秒 | 心跳保活间隔 |
| **QoS** | 1 (AtLeastOnce) | 至少一次投递保证 |
| **认证** | 匿名访问 (allow_anonymous: true) | 当前配置无需用户名密码 |

### 本地开发环境

| 参数 | 值 | 说明 |
|------|-----|------|
| **Broker 地址** | `localhost` | 本地开发 |
| **端口** | `1883` | 标准MQTT端口 |
| **完整连接地址** | `mqtt://localhost:1883` | 本地测试使用 |

---

## 2. 主题(Topic)结构

系统支持两种主题格式，推荐使用带前缀格式：

### 格式一：带前缀（推荐）

```
remipedia/devices/{serial_number}/{device_type}
```

**示例：**
- `remipedia/devices/SN12345/heart_rate_monitor` - 心率监测设备
- `remipedia/devices/BED001/smart_mattress` - 智能床垫设备
- `remipedia/devices/FALL001/fall_detector` - 跌倒检测器

### 格式二：无前缀

```
devices/{serial_number}/{device_type}
```

**示例：**
- `devices/SN12345/heart_rate_monitor`
- `devices/BED001/smart_mattress`

### 主题字段说明

| 字段 | 说明 | 示例 |
|------|------|------|
| `remipedia` | 主题前缀（可配置） | 生产环境固定为 `remipedia` |
| `devices` | 固定标识 | 表示设备数据主题 |
| `serial_number` | 设备序列号 | `SN12345`, `BED001` |
| `device_type` | 设备类型 | `heart_rate_monitor`, `smart_mattress`, `fall_detector` |

---

## 3. 数据包格式要求

### 3.1 通用 JSON 格式

**Content-Type:** `application/json`

**基本结构：**
```json
{
  "device_type": "heart_rate_monitor",
  "serial_number": "SN12345",
  "timestamp": "2024-01-15T10:30:00Z",
  "data": {
    "heart_rate": 72,
    "spo2": 98
  }
}
```

### 3.2 跌倒检测器专用格式

**主题：** `remipedia/devices/{serial_number}/fall_detector`

```json
{
  "event_type": "person_fall",
  "timestamp": "2024-01-15T10:30:00Z",
  "details": {
    "zone": "A-01",
    "source": "edge_cam_2",
    "confidence": 0.95
  }
}
```

**支持的事件类型：**
| 事件类型 | 说明 |
|----------|------|
| `person_fall` | 人物跌倒 |
| `person_still` | 人物静止 |
| `person_enter` | 人物进入 |
| `person_leave` | 人物离开 |

### 3.3 智能床垫专用格式

**主题：** `remipedia/devices/{serial_number}/smart_mattress`

```json
{
  "Ma": "HT",
  "Mo": "03",
  "V": 1,
  "Sn": "BED001",
  "fv": 100,
  "St": "on",
  "Hb": 72,
  "Br": 16,
  "Wt": false,
  "Od": 0,
  "We": 15,
  "P": [50, 60],
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**字段映射：**
| 字段 | 说明 | 范围/值 |
|------|------|---------|
| `Ma` | 制造商 | 固定为 "HT" |
| `Mo` | 型号 | "02" / "03" |
| `V` | 协议版本 | 整数 |
| `Sn` | 设备序列号 | 字符串 |
| `fv` | 固件版本 | 整数 |
| `St` | 体征状态 | on/off/mov/call |
| `Hb` | 心跳频率 | 30-200 BPM |
| `Br` | 呼吸频率 | 5-60 次/分钟 |
| `Wt` | 尿湿状态 | true/false |
| `Od` | 呼吸暂停次数 | 整数 |
| `We` | 辅助重量值 | 0-20, -1表示未安装 |
| `P` | 身体位置坐标 | [头部, 胸部] |

---

## 4. 测试示例

### 使用 mosquitto_pub 测试

```bash
# 1. 测试心率监测设备
mosquitto_pub -h 68.64.178.247 -p 1883 \
  -t "remipedia/devices/HR001/heart_rate_monitor" \
  -m '{"device_type":"heart_rate_monitor","serial_number":"HR001","heart_rate":72,"spo2":98}'

# 2. 测试跌倒检测器
mosquitto_pub -h 68.64.178.247 -p 1883 \
  -t "remipedia/devices/FALL001/fall_detector" \
  -m '{"event_type":"person_fall","timestamp":"2024-01-15T10:30:00Z","details":{"zone":"bed-3"}}'

# 3. 测试智能床垫
mosquitto_pub -h 68.64.178.247 -p 1883 \
  -t "remipedia/devices/BED001/smart_mattress" \
  -m '{"Ma":"HT","Mo":"03","Sn":"BED001","St":"on","Hb":72,"Br":16,"We":15}'
```

### 使用 Python paho-mqtt 测试

```python
import paho.mqtt.client as mqtt
import json

# 连接配置
broker = "68.64.178.247"
port = 1883
client_id = "test-client-001"

# 创建客户端
client = mqtt.Client(client_id)
client.connect(broker, port, keepalive=30)

# 构造数据
payload = {
    "device_type": "heart_rate_monitor",
    "serial_number": "HR001",
    "heart_rate": 72,
    "spo2": 98,
    "timestamp": "2024-01-15T10:30:00Z"
}

# 发布消息
topic = "remipedia/devices/HR001/heart_rate_monitor"
client.publish(topic, json.dumps(payload), qos=1)

client.disconnect()
print("消息已发送")
```

---

## 5. 数据流处理流程

```
设备发送 MQTT 消息
    ↓
MQTT Broker (Mosquitto) 接收
    ↓
Remipedia 订阅主题: remipedia/devices/+/+
    ↓
解析 Topic 提取 serial_number 和 device_type
    ↓
转换为 DataPacket → Pipeline 处理
    ↓
自动设备注册（如设备不存在）
    ↓
数据存储到 PostgreSQL
```

---

## 6. 注意事项

### 6.1 序列号规则
- 设备序列号必须在 Topic 中提供
- 系统会根据序列号自动创建设备记录
- 建议序列号格式：`[类型前缀][数字]`，如 `HR001`, `BED123`

### 6.2 时间戳处理
- 支持 RFC3339 格式时间戳（可选）
- 如不提供，系统使用服务器接收时间
- 示例：`"2024-01-15T10:30:00Z"`

### 6.3 设备类型
- 可在 Topic 或 JSON payload 中指定
- Topic 优先级高于 payload
- 常用类型：`heart_rate_monitor`, `smart_mattress`, `fall_detector`, `blood_pressure_monitor`

### 6.4 网络要求
- 确保设备能访问服务器 `68.64.178.247:1883`
- 防火墙需开放 1883 端口（TCP）
- 当前配置为明文传输，内网/专线环境建议使用

### 6.5 调试建议
- 使用 `mosquitto_sub` 订阅主题查看数据流：
  ```bash
  mosquitto_sub -h 68.64.178.247 -p 1883 -t "remipedia/devices/#" -v
  ```
- 检查服务器日志确认数据接收：
  ```bash
  ssh yecaoyun "docker logs -f remipedia-app"
  ```

---

## 7. 配置文件参考

### 生产环境配置 (config/default.yaml)
```yaml
mqtt:
  broker: "localhost"    # Docker 容器内使用服务名
  port: 1883
  client_id: "remipedia-server"
  topic_prefix: "remipedia"
  enabled: true
```

### Mosquitto 配置 (mosquitto.conf)
```
listener 1883 0.0.0.0
allow_anonymous true
persistence true
persistence_location /mosquitto/data
```

---

## 8. 故障排查

| 问题 | 可能原因 | 解决方案 |
|------|----------|----------|
| 连接失败 | 防火墙/网络不通 | 检查 1883 端口连通性 |
| 消息未存储 | Topic 格式错误 | 确认使用 `remipedia/devices/{sn}/{type}` 格式 |
| 设备未注册 | 序列号为空 | 确保 Topic 包含有效 serial_number |
| 数据解析失败 | JSON 格式错误 | 验证 JSON 格式，检查特殊字符 |

---

**文档版本：** v1.0  
**最后更新：** 2025年4月  
**适用环境：** yecaoyun (68.64.178.247) 生产环境
