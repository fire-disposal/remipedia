# 跌倒检测器边缘设备集成指南（简化 MQTT 示例）

## 概述

跌倒检测器在当前版本中作为**纯 MQTT 事件输入示例**：
- 后端只负责事件接入、时间标准化、自动注册、落库。
- 不再在 ingest 侧做置信度阈值判断/分级计算。

## MQTT 协议规范

### 连接参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| Broker | `localhost` | MQTT 代理服务器地址 |
| Port | `1883` | MQTT 端口 |
| Client ID | 设备序列号 | 建议使用唯一标识 |
| Keep Alive | 30秒 | 心跳间隔 |
| QoS | 1 | 至少一次投递 |

### 主题定义

| 方向 | 主题格式 | 说明 |
|------|----------|------|
| 上行 | `remipedia/{serial_number}/event` | 事件上报 |

### 消息格式

```json
{
  "event_type": "person_fall",
  "timestamp": "2024-01-15T10:30:00Z",
  "details": {
    "zone": "A-01",
    "source": "edge_cam_2"
  }
}
```

#### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| event_type | string | 是 | 事件类型 |
| timestamp | string | 否 | RFC3339 时间，不传则使用服务器时间 |
| details | object | 否 | 任意业务附加信息，后端透传落库 |

#### 事件类型

| 事件类型 | 说明 |
|----------|------|
| `person_fall` | 人物跌倒 |
| `person_still` | 人物静止 |
| `person_enter` | 人物进入 |
| `person_leave` | 人物离开 |

## 自动注册流程

1. 设备连接 MQTT Broker
2. 设备发布到 `remipedia/{serial_number}/event`
3. 后端自动注册/识别设备
4. 事件进入 ingest 数据流并落库

## 测试样例

```bash
mosquitto_pub -h localhost -p 1883 \
  -t "remipedia/FALL-TEST-001/event" \
  -m '{"event_type":"person_fall","details":{"zone":"bed-3"}}'
```

## 设计说明

- 置信度评分、算法阈值属于边缘算法域，不属于 ingest 解析域。
- ingest 侧优先保证吞吐、可观测、可追溯。
