# 智能床垫设备通讯协议文档

## 一、 网络传输层
* **协议类型**：TCP/IP
* **默认端口**：`5858` (支持自定义，须与设备上传端口保持一致)
* **参考配置**：见《WiFi 设置指导.doc》

## 二、 数据链路层 (封装格式)
数据包采用 **MessagePack** 进行二进制序列化。
* **参考资料**：[MessagePack 官网](http://msgpack.org/) | [C语言实现](https://github.com/msgpack/msgpack-c)

### 2.1 数据包结构 (Struct)
每个数据包由包头和 MessagePack 载荷组成：

| 偏移 | 字段名 | 类型 | 说明 |
| :--- | :--- | :--- | :--- |
| 0-1 | **magic** | uint8[2] | 消息起始标识：`0xab 0xcd` |
| 2 | **len** | uint8 | `data` 部分的字节总数 |
| 3 | **crc** | uint8 | `data` 部分的 CRC_8 校验值 |
| 4+ | **data** | uint8[] | 消息内容 (MessagePack 封包)，长度动态计算 |

---

## 三、 数据载荷定义 (Data Section)
`data` 部分解包后为一个 Map 结构，包含 **8 个属性** (对应 MessagePack 标识 `0x88`)。

### 3.1 公共信息
| 字段键 | 原始字节 | 类型 | 说明 |
| :--- | :--- | :--- | :--- |
| **Ma** | `0x6d 0x61` | string | 制造商 (Manufacturer)，固定为 `HT` |
| **Mo** | `0x6d 0x6f` | string | 型号 (Model)，`02`/`03` 为称重型号 |
| **V** | `0x76` | integer | 协议版本号，从 `1` 开始 |
| **Sn** | `0x73 0x6e` | string | 设备唯一序列号 (如：`Z50001`) |
| **D** | `0x64` | map | 设备详细数据节点 (详见下表) |

### 3.2 设备详细数据 (D 节点)
| 字段键 | 原始字节 | 类型 | 说明 |
| :--- | :--- | :--- | :--- |
| **fv** | `0x66 0x76` | integer | 固件版本号 |
| **St** | `0x73 0x74` | string | 体征状态：`on`(在床), `off`(离床), `mov`(体动), `call`(呼叫) |
| **Hb** | `0x68 0x62` | integer | 心跳频率 (Heartbeat) |
| **Br** | `0x62 0x72` | integer | 呼吸频率 (Breath) |
| **Wt** | `0x77 0x74` | boolean | 尿湿状态：`true`(尿湿/0xc3), `false`(正常/0xc2) |
| **Od** | `0x6f 0x64` | integer | 呼吸暂停次数 (24小时清零，3秒发送一次累加值) |
| **We** | `0x77 0x65` | integer | 辅助重量值：`0-20` (详见逻辑说明)；`-1` 表示未安装 |
| **P** | `0x50` | array | 身体位置坐标，格式为 `[头部, 胸部]`。例如 `[6, 9]` |

> **注意**：当 `Mo=03` 时，心率、呼吸等值为默认值，仅通过 `St` 判断在床状态。

---

## 四、 核心业务逻辑说明

### 4.1 翻身动作分析
通过比较体动（mov）发生前后的位置坐标 `P{头部, 胸部}` 来判断。

* **报文示例**：`0x92 0x06 0x09` 表示 P=[6, 9]。
* **简易算法**：
    $Value = |头部_{后} - 头部_{前}| + |胸部_{后} - 胸部_{前}|$
    若 $Value > 2$，判定为翻身。
* **精准算法**：
    $Value = |头部_{后} - 头部_{前4次均值}| + |胸部_{后} - 胸部_{前4次均值}|$
* **自理能力参数**：
    根据老人情况设定判断阈值：
    * `1.2` (瘫痪) / `1.4` (半瘫) / `1.6` (半自理) / `2.0` (自理)

### 4.2 重量 (Weight) 辅助判断
`Weight` 并非真实体重，而是非线性的感测值（范围 0-20）。
* **无人**：`0 - 10`
* **有人**：`17 - 20`

#### 复合判断逻辑：
1.  **快速离床判定**：若 `St=on` 但 `Weight < 10` 持续 3 秒，判定为**离床**（补偿 `St` 15秒的延迟）。
2.  **紧急报警判定**：若 `St=off` 但 `Weight > 17` 持续 30 秒，判定为**紧急情况**（模块未检测到心跳但有人压在床上），需护工介入。
3.  **抗干扰修正**：若外界干扰导致 `St` 误判为有人，但 `Weight < 10`，则以 Weight 为准，修正为**离床**并过滤干扰心率。

# 解包示例
```c
主分析函数：
content 为 socket 传递过来的 byte[]


                int lenth = int.Parse(content[2].ToString());//64
                //验证总包的长度是否正确
                if (content.Length >= lenth + 4)
                {
                    try
                    {
                       
	                  bedEntry vv = unpack(content);     //调用解码函数                    
                           sn = vv.sn;
                           wt = vv.d.wt == true ? "1" : "0";
                           br = vv.d.br.ToString();
                           hb = vv.d.hb.ToString();
                           st = vv.d.st;
                           p = "[" + vv.d.p[0].ToString() + "," + vv.d.p[1].ToString() + "]";
                           fv = vv.d.fv.ToString();
                           we = vv.d.we.ToString();
                           mo = vv.mo;
                        #endregion
                    }
                   catch
                    {
                    }
                 
                    }

解码部分：（引用MsgPack.dll）
using MsgPack.Serialization;
 private MessagePackSerializer<bedEntry> serializer = null;

声明：
#region 解码
            var context = new SerializationContext();
            context.SerializationMethod = SerializationMethod.Map;
           
            serializer = MessagePackSerializer.Get<bedEntry>(context);
        
            #endregion

   public bedEntry unpack(byte[] bytes)//解码函数
        {
  		
            byte[] data1 = new byte[bytes.Length - 4];
            Array.Copy(bytes, 4, data1, 0, data1.Length);
            var deserializedObject = serializer.UnpackSingleObject(data1);
            return deserializedObject;
        }
bedEntry ：定义
  public class bedEntry
    {
        public string ma { get; set; }
        public string mo { get; set; }
        public int v { get; set; }
        public string sn { get; set; }
        public bed_d d { get; set; }
    }
    public class bed_d
    {
        public string st { get; set; }
        public int hb { get; set; }
        public int br { get; set; }
        public bool wt { get; set; }
        public int od { get; set; }
        public int we { get; set; }
        public int[] p { get; set; }
        public int fv { get; set; }
    }
```


# 数据载荷示例：

```json
{"type":null,"time":"2019-05-31 01:32:38","br":14,"fv":13,"hb":51,"od":0,"p":"[4,4]","st":"on","we":20,"wt":"0","sn":"Z56552"}
{"type":null,"time":"2019-05-31 01:32:41","br":14,"fv":13,"hb":51,"od":0,"p":"[4,3]","st":"on","we":20,"wt":"0","sn":"Z56552"}
{"type":null,"time":"2019-05-31 01:32:44","br":14,"fv":13,"hb":50,"od":0,"p":"[4,3]","st":"on","we":20,"wt":"0","sn":"Z56552"}
{"type":null,"time":"2019-05-31 01:32:47","br":14,"fv":13,"hb":50,"od":0,"p":"[4,4]","st":"on","we":20,"wt":"0","sn":"Z56552"}
{"type":null,"time":"2019-05-31 01:32:50","br":13,"fv":13,"hb":50,"od":0,"p":"[4,4]","st":"on","we":20,"wt":"0","sn":"Z56552"} 
```

