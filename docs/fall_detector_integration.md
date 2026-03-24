# 跌倒检测器边缘设备集成指南

## 概述

本文档描述跌倒检测边缘设备如何通过MQTT协议与后端系统集成，实现事件上报和自动注册。

## MQTT协议规范

### 连接参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| Broker | `localhost` | MQTT代理服务器地址 |
| Port | `1883` | MQTT端口 |
| Client ID | 设备序列号 | 建议使用唯一标识 |
| Keep Alive | 30秒 | 心跳间隔 |
| QoS | 1 | 至少一次投递 |

### 主题定义

| 方向 | 主题格式 | 说明 |
|------|----------|------|
| 上行 | `remipedia/{serial_number}/event` | 事件上报 |

- `remipedia`: 主题前缀（可配置）
- `{serial_number}`: 设备唯一序列号

### 消息格式

事件消息采用JSON格式：

```json
{
    "event_type": "person_fall",
    "confidence": 0.85,
    "timestamp": "2024-01-15T10:30:00Z"
}
```

#### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| event_type | string | 是 | 事件类型 |
| confidence | float | 是 | 置信度 (0.0-1.0) |
| timestamp | string | 否 | 事件时间 (RFC3339)，不提供则使用服务器时间 |

#### 事件类型

| 事件类型 | 说明 | 告警 | 置信度要求 |
|----------|------|------|------------|
| `person_fall` | 人物跌倒 | ✅ | ≥ 0.5 |
| `person_still` | 人物静止 | ❌ | - |
| `person_enter` | 人物进入 | ❌ | - |
| `person_leave` | 人物离开 | ❌ | - |

## 自动注册流程

1. 设备连接MQTT代理
2. 设备发布事件到 `remipedia/{serial_number}/event`
3. 后端检测到新设备，自动创建设备记录
4. 设备后续消息正常处理

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  边缘设备   │────▶│  MQTT代理   │────▶│   后端服务  │
└─────────────┘     └─────────────┘     └─────────────┘
       │                                        │
       │  publish: remipedia/SN001/event        │
       │  {"event_type":"person_fall",...}      │
       │                                        │
       │                              自动注册设备
       │                              解析事件数据
       │                              存储到数据库
```

## 设备端实现示例

### Python示例

```python
import json
import time
from datetime import datetime, timezone
import paho.mqtt.client as mqtt

# 配置
BROKER = "localhost"
PORT = 1883
SERIAL_NUMBER = "FALL-001"
TOPIC = f"remipedia/{SERIAL_NUMBER}/event"

def on_connect(client, userdata, flags, rc):
    print(f"已连接到MQTT代理，返回码: {rc}")

def create_event(event_type: str, confidence: float) -> dict:
    return {
        "event_type": event_type,
        "confidence": confidence,
        "timestamp": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    }

def main():
    client = mqtt.Client(client_id=SERIAL_NUMBER)
    client.on_connect = on_connect

    client.connect(BROKER, PORT, 60)
    client.loop_start()

    # 模拟事件上报
    events = [
        ("person_enter", 0.92),
        ("person_still", 0.78),
        ("person_fall", 0.87),  # 跌倒告警
        ("person_leave", 0.95),
    ]

    for event_type, confidence in events:
        event = create_event(event_type, confidence)
        payload = json.dumps(event)
        client.publish(TOPIC, payload, qos=1)
        print(f"已发送事件: {event_type}")
        time.sleep(2)

    client.loop_stop()
    client.disconnect()

if __name__ == "__main__":
    main()
```

### C/C++示例 (ESP32)

```cpp
#include <ArduinoJson.h>
#include <PubSubClient.h>
#include <WiFiClient.h>

const char* BROKER = "localhost";
const int PORT = 1883;
const char* SERIAL_NUMBER = "FALL-002";
const char* TOPIC = "remipedia/FALL-002/event";

WiFiClient espClient;
PubSubClient mqtt(espClient);

void publishEvent(const char* eventType, float confidence) {
    StaticJsonDocument<200> doc;
    doc["event_type"] = eventType;
    doc["confidence"] = confidence;
    doc["timestamp"] = "2024-01-15T10:30:00Z";  // 使用实际时间

    char buffer[256];
    serializeJson(doc, buffer);

    mqtt.publish(TOPIC, buffer, true);
    Serial.printf("已发送事件: %s\n", eventType);
}

void setup() {
    Serial.begin(115200);
    mqtt.setServer(BROKER, PORT);
}

void loop() {
    if (!mqtt.connected()) {
        mqtt.connect(SERIAL_NUMBER);
    }
    mqtt.loop();

    // 检测到跌倒事件
    if (detectFall()) {
        publishEvent("person_fall", 0.85);
    }
}
```

## 配置说明

### 后端配置 (config/default.yaml)

```yaml
mqtt:
  broker: "localhost"
  port: 1883
  client_id: "remipedia-server"
  topic_prefix: "remipedia"
  enabled: true
```

### 环境变量

```bash
APP__MQTT__BROKER=mqtt.example.com
APP__MQTT__PORT=1883
APP__MQTT__TOPIC_PREFIX=remipedia
APP__MQTT__ENABLED=true
```

## 测试验证

### 使用mosquitto_pub测试

```bash
# 发送跌倒事件
mosquitto_pub -h localhost -p 1883 \
  -t "remipedia/FALL-TEST-001/event" \
  -m '{"event_type":"person_fall","confidence":0.85}'

# 发送进入事件
mosquitto_pub -h localhost -p 1883 \
  -t "remipedia/FALL-TEST-001/event" \
  -m '{"event_type":"person_enter","confidence":0.92}'
```

### 验证数据入库

```bash
# 查询设备数据
curl "http://localhost:8000/api/v1/data/latest?device_type=fall_detector&data_type=fall_event"
```

## 错误处理

### 常见错误

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| 消息解析失败 | JSON格式错误 | 检查JSON语法 |
| 置信度超出范围 | confidence不在0-1之间 | 调整置信度值 |
| 跌倒事件置信度不足 | person_fall置信度<0.5 | 提高检测阈值 |
| 无效事件类型 | event_type不在允许列表 | 使用正确的事件类型 |

### 日志示例

```
INFO  MQTT 事件循环启动
INFO  已订阅 MQTT 主题: remipedia/+/data
INFO  已订阅 MQTT 主题: remipedia/+/event
INFO  设备自动注册成功: device_id=xxx, device_type=fall_detector
INFO  数据入库成功: device_id=xxx, subject_id=Some(uuid), data_type=fall_event
ERROR 处理 MQTT 消息失败: 跌倒事件置信度不足(需>=0.5), topic: remipedia/FALL-001/event
```

## 最佳实践

1. **序列号规范**: 使用唯一且有意义的序列号，如 `FALL-ROOM001-001`
2. **时间戳**: 建议设备提供准确的事件时间戳
3. **置信度**: 根据检测算法实际表现设置合理的置信度
4. **重连机制**: 实现MQTT断线重连
5. **消息确认**: 使用QoS 1确保消息可靠投递

## 安全建议

1. 生产环境使用TLS加密连接
2. 启用MQTT用户名/密码认证
3. 使用ACL限制设备只能发布到自己的主题
4. 敏感数据在应用层加密