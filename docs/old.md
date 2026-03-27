**项目技术考古报告 — 床垫系统概览**

本文档汇总仓库中有关床垫设备数据接入、状态判读引擎和床垫相关数据结构的实现要点，便于后续维护与二次开发。

**总体架构**:
- **设备连接**: 床垫设备通过 TCP 二进制协议连接至服务端监听端口（`aoxin_Mattress/server.js`，监听 5858）。
- **解析与分发**: 服务端解析二进制包，形成 JSON 格式的 `requestData`，然后进行：Redis 写入、HTTP 转发（`/signal/addSignal`）、文件持久化、事件触发（内部处理/转发/判读）。

**关键接入实现位置**
- 接收与解析（TCP -> JSON）: [aoxin_Mattress/server.js](aoxin_Mattress/server.js)
- 设备数据辅助函数、存储与转发: [aoxin_Mattress/utils/tools.js](aoxin_Mattress/utils/tools.js)
- 数据结构与判读示例（Python）: [aoxin_Mattress/mattress_analysis.py](aoxin_Mattress/mattress_analysis.py)
- 位置解析: [aoxin_Mattress/utils/parser.js](aoxin_Mattress/utils/parser.js)
- Redis / MQTT 客户端: [aoxin_Mattress/utils/mqttClient.js](aoxin_Mattress/utils/mqttClient.js)
- 业务服务端接口（保存/转发体征）: [aoxin_Service/apis/signal_api.js](aoxin_Service/apis/signal_api.js)

**二进制协议（在 `server.js` 的 manage() 实现）**
- 采用类似 TLV 的解析：设备发送一串字节，解析时将 0xA1..0xA7 作为 key 前缀，后随长度与值。
- 特殊类型：当遇到 0x92 时按 两字节 int 数组 解析为类似 "[x,y]" 的字符串（用于 `p` 位置字段）。
- 最终解析出的键名及含义（常见）:
  - `hb`: 心跳（心率）
  - `br`: 呼吸（呼吸率）
  - `od`: 未知感知值（协议中使用 255 表示 -1）
  - `p`: 位置，字符串形式 "[a,b]"
  - `st`: 状态字符串，常见值 `on` / `off` / `mov`
  - `we`: 重量或力值（255 表示 -1）
  - `wt`: 标志位（协议中 195 映射为 "1"）
  - `sn`: 设备序列号
  - `fv`: 固件版本或其他字段

示例解析后对象（JavaScript 名为 `requestData` / Python 中 `MattressData`）:

```json
{
  "sn":"Z57566",
  "hb":75,
  "br":14,
  "p":"[4,4]",
  "st":"on",
  "we":20,
  "wt":"1",
  "time":"2020-04-18 08:00:06",
  "sleepState":"0"
}
```

**sleepState（入睡状态）判定规则**
- 由 `GetSleepState`（在 `server.js` 与 Python 的 `get_sleep_state` 中实现）维护每台设备的 `sleepRecordArr`/`sleepRecordArr`：
  - 新上报首次为 `sleepState='0'`。
  - 当 `st === 'on'` 并且连续在线时间超过 240s: `0` -> `1`。
  - 超过 420s: `1` -> `2`。
  - 超过 840s: `2` -> `3`。
  - `st === 'mov'` 可触发从 `2`/`3` 回滚到 `0`（有短时阈值 40s）。
  - `st === 'off'` 立即重置为 `0`。

该规则用于区分在线、浅睡、深睡与长时间睡眠状态，并影响后续离床/褥疮判定。

**状态判读引擎（主要逻辑位置与要点）**
- 心跳/呼吸报警（连续计数）:
  - 在 `aoxin_Mattress/utils/tools.js` 的 `dataClear` 与 `prepareData` 中，用 Redis 保存连续异常计数（key 示例：`<sn>breathThreshold`, `<sn>heartThreshold`）。
  - 当计数达到数据库配置的阈值（`old` 表中配置项：`heart_threshold` / `breath_threshold`）时，触发 HTTP 推送到异常接口 `/abnormal/addAbnormal` 并设置 `requestData.type=0`（心跳）或 `1`（呼吸）。
- 褥疮保护与翻身判定:
  - 数据库字段 `bedsore_protect` 为 1 时启用，`tools.js` 的 `judgeTurn` 逻辑比较当前位置 `p`（解析为 {a,b}）与上次记录，若位置差（曼哈顿距离）>= 2 视为翻身。
  - 若超过设定时间（如 >120 分钟）且未翻身，触发褥疮报警（`type=3`）。
- 离床（off）检测与离床超时报警:
  - `offTimeCheck` 使用床位配置（`off_detect_start`, `off_detect_end`, `off_time_set`）判断是否进入报警窗口，若在报警时间段内离床时长超过设置值，触发离床报警（`type=2`）。
  - 离床期间会在 Redis 中用 `<sn>offTime` 等键保存首个离床时间作为基准。
- 断线/异常判断:
  - 在 `server.js` 中有全局 `abnormal_judgect` 计数器与逻辑（例如当 `we>17 且 st=='off'` 连续计数达到 10，则 `type=4`）。

**数据流与存储**
- 实时路径（设备 -> 后端）:
  1. 设备 TCP 到 `aoxin_Mattress/server.js` -> 解析为 `requestData`。
  2. 通过 `tools.addSnSignal` 写 Redis（键名为 SN），并 HTTP POST 到 `http://127.0.0.1:7878/signal/addSignal`（由 `aoxin_Service` 处理存储/发布）。
  3. 同时将原始 JSON 追加写入日期目录下的 `/MattressData/YYYY-MM-DD/<sn>.txt`（文件持久化示例）。
  4. 触发 `prepareData` / `doDataTrans` 两个事件：前者处理阈值/离床/褥疮逻辑并请求异常接口，后者根据床位关系发布 MQTT 主题供前端/护士端订阅。
- Redis 与数据库:
  - Redis 用于保存短期计数与标志（例如 `<sn>breathThreshold`, `<sn>heartThreshold`, `<sn>offTime`, `<sn>sleepFlag`, `<sn>Turnct` 等）。
  - 关系型 DB（通过 `baseDAL`）保存持久信息，如 `old_state_change`, `abnormal_handle`, `new_vital` 等表（见 `aoxin_Service/dal` 下 DAL 实现）。
- 事件与通知:
  - 异常推送由 `tools.httpRequest` 调用 `abnormal/addAbnormal`。
  - 实时转发通过 MQTT（`aoxin_Mattress/utils/mqttClient.js`）对订阅客户端推送主题 `vitalMsg/...` 或 `mechanism/<sn>` 等。

**关键数据结构**
- Python 侧样例类（`aoxin_Mattress/mattress_analysis.py`）:
  - `MattressData` 字段: `hb, br, od, p, st, we, wt, sn, fv, type, time, sleepState`。
  - `HealthEventManager` / `MattressAnalyzer` 里包含心跳/呼吸阈值计数、翻身计数、异常事件列表与 sleepRecord 缓存。
- JavaScript 侧: `requestData` / `ans` 对象与 `util.setstorageData` 对象布局与 Python 类字段一致（见 `aoxin_Mattress/utils/util.js`）。

**重要文件一览（定位与作用）**
- `aoxin_Mattress/server.js`: TCP 服务、二进制解析、sleepState 算法、事件发射。
- `aoxin_Mattress/utils/tools.js`: 核心业务处理（阈值/离床/褥疮/文件写入/Redis/MQTT/HTTP）。
- `aoxin_Mattress/utils/parser.js`: 将位置字符串 "[x,y]" 解析为数字对象。
- `aoxin_Mattress/mattress_analysis.py`: Python 版的判读/示例实现，便于理解整体算法。
- `aoxin_Service/apis/signal_api.js` 与 `aoxin_Service/dal/SignalDAL.js`: 接收 `signal/addSignal` 的 HTTP 接口与保存/广播逻辑。

**快速复现与调试建议**
- 本地复现可先启动 `aoxin_Mattress/server.js`（端口 5858），然后用提供的 `aoxin_Mattress/test.js` 或自定义 TCP 客户端发送协议包进行验证。
- 检查本地 Redis 连接（示例端口/密码在代码中有硬编码），以及 `aoxin_Service` 的 HTTP 服务（默认 7878）是否可达。

**结论与后续工作建议**
- 此项目以轻量 TCP 二进制协议配合 Redis + HTTP + MQTT 的混合架构实现实时上报与阈值判读；判读逻辑分布在 `aoxin_Mattress` 的工具层与 `aoxin_Service` 的后端 DAL 层。
- 建议后续整理：
  - 将协议规范化为文档（字段、类型、特殊值说明）。
  - 将阈值/报警策略集中配置，减少硬编码（更易测试与升级）。
  - 增加单元测试用例覆盖 `manage()`/`GetSleepState` 与 `judgeTurn` 的边界情形。

**附：参考文件路径**
- aoxin_Mattress/server.js
- aoxin_Mattress/utils/tools.js
- aoxin_Mattress/mattress_analysis.py
- aoxin_Mattress/utils/parser.js
- aoxin_Mattress/utils/mqttClient.js
- aoxin_Service/apis/signal_api.js
- aoxin_Service/dal/SignalDAL.js

-- 结束 --

**协议文档整合（MessagePack 协议）**

下面为你提供的“智能床垫设备通讯协议”要点的整合摘要，已与仓库实现逐项比对并标注差异：

网络层与封装：
- 协议层使用 TCP，默认端口 5858（与当前 `aoxin_Mattress/server.js` 一致）。
- 载荷采用 MessagePack，外层有包头：magic(0xAB 0xCD)、len、crc、data（MessagePack bytes）。

MessagePack 载荷结构（data 解包后为 map，包含 manufacturer/model/version/sn/d 等）：
- 公共字段：`Ma`(HT)、`Mo`(型号)、`V`(版本)、`Sn`(序列号)、`D`(设备数据节点)。
- D 节点典型键：`fv, St, Hb, Br, Wt, Od, We, P`，其中 `P` 为 int[]（例如 [6,9]）。

协议行为/语义要点：
- `Wt` 为 boolean（尿湿），代码中使用 wt="1"/"0" 表示，需要映射。
- `Od` 在协议中 255 表示 -1；`We` 255 表示未安装。仓库 `manage()` 已对 255 做特殊映射，但需确认 MessagePack 解包后数值与现有判读逻辑的一致性。
- `P` 可以用单字节数组格式（协议示例中 `0x92 0x06 0x09`）或 MessagePack array；仓库现有逻辑也能处理字符串形式 "[x,y]" 并由 `utils/parser.js` 转为 `{a,b}`。

解包示例与服务器端实现差异（关键对比）：
- 协议文档：要求先校验包头 `0xAB 0xCD`、len 与 CRC8，然后用 MessagePack 库解包为 bedEntry 对象（大写字段如 `St`,`Hb`）。
- 现有代码：`aoxin_Mattress/server.js` 的 `manage()` 并未按 MessagePack 解包，而是从第 8 字节起按自定义 TLV（0xA1..0xA7 为 key，0x92 表示两字节数组）逐字节解析并通过 `byteToString()` 构造键名与值。

结论与兼容性建议：
- 可能存在两种设备/固件版本或网关：一种直接发送自定义 TLV（现有解析器支持），另一种按 MessagePack + 包头发送（需要新增解码分支）。
- 必须在接收层先判断包头：若检测到 magic(0xAB 0xCD) 则按协议文档做 len/CRC 校验并用 MessagePack 解包，否则沿用现有 TLV 解析逻辑。

实现建议（优先级）:
1. 在 `aoxin_Mattress/server.js` 增加包头检测與 MessagePack 解包分支：
  - 使用 `@msgpack/msgpack` 或 `msgpack-lite` 进行解码；将解码后对象字段（如 `St`）映射为小写 `st` 并平铺到 `requestData` 结构后复用现有流程（`tools.addSnSignal`、`prepareData` 等）。
2. 添加 CRC8 校验库（或手工实现）以验证包体完整性并丢弃错误包。  
3. 增加字段映射表（例如 {St: 'st', Hb: 'hb', Br: 'br', Wt: 'wt', P: 'p', We: 'we', Od: 'od', fv: 'fv'}），并在解包后执行映射与类型修正（boolean->"1"/"0" 或保留 boolean，视上游需要）。
4. 编写单元测试：分别用示例 MessagePack 包与现有 TLV 字节流进行端到端解析测试，确保 `sleepState` / 报警逻辑一致。

附：协议差异速览（简表）
- 包头/CRC：协议要求，有校验；仓库解析无此校验。
- 载荷编码：协议为 MessagePack；仓库当前实现为自定义 TLV（0xA1..0xA7 / 0x92）。
- 字段大小写与类型：协议用大写/驼峰如 `St`,`Hb`；仓库使用小写 `st`,`hb`，且对 `wt` 用字符 "1"/"0" 处理而非 boolean。

我已把上述内容合并到 `TECHNICAL_ARCHAEOLOGY.md` 中，并将 `对接协议文档并实现 MessagePack 解包（建议）` 标注为已完成建议项（实作尚未在 `server.js` 添加解码分支）。

下一步我可以：
- 立即实现并运行 MessagePack 解包分支（在 `aoxin_Mattress/server.js` 添加检测/解码/字段映射与快速单元测试）。
- 或先生成 JSON Schema 与若干示例 MessagePack 报文供验证。 

