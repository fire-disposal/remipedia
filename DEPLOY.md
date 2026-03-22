# 🚀 Remipedia IoT 健康平台 - 快速部署指南

## 📋 部署前准备

### 1. 系统要求
- **操作系统**: Linux (推荐 Ubuntu 20.04+)
- **内存**: 最少 2GB RAM
- **存储**: 最少 10GB 可用空间
- **网络**: 开放端口 8000, 5858, 5432, 1883
- **Docker**: 20.10+ 版本
- **Docker Compose**: 2.0+ 版本

### 2. 安装依赖
```bash
# 安装 Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

## ⚡ 一键部署

### 1. 克隆代码
```bash
git clone https://github.com/your-org/remipedia.git
cd remipedia
```

### 2. 环境配置
```bash
# 创建环境配置文件
cp .env.example .env.prod

# 编辑配置文件（必须设置以下参数）
nano .env.prod
```

**必需配置项：**
- `DB_PASSWORD`: 数据库密码（至少8位）
- `JWT_SECRET`: JWT密钥（至少32位随机字符串）
- `APP_ENV`: 应用环境（production）

### 3. 快速部署
```bash
# 使用部署脚本（推荐）
./deploy.sh deploy

# 或者手动部署
docker-compose up -d
```

### 3. 验证部署
```bash
# 检查服务状态
docker-compose ps

# 测试API
curl http://localhost:8000/health

# 访问Swagger文档
open http://localhost:8000/swagger
```

## 🔧 配置说明

### 环境变量配置
系统使用统一的 `docker-compose.yml` 配置文件，通过环境变量区分开发/生产环境。

**创建 `.env.prod` 文件：**
```bash
# 数据库配置（必需）
DB_PASSWORD=your-secure-password-here

# JWT配置（必需，至少32位随机字符串）
JWT_SECRET=your-super-secret-jwt-key-here-32-chars-min

# 应用配置
APP_ENV=production

# 可选配置（使用默认值即可）
# APP_PORT=8000                    # HTTP API端口
# TCP_PORT=5858                    # TCP床垫数据端口
# MQTT_PORT=1883                   # MQTT消息端口
# DB_PORT=5432                     # 数据库端口
# RUST_LOG=info                    # 日志级别
```

**配置验证：**
```bash
# 检查环境变量是否生效
docker-compose config

# 验证配置文件语法
docker-compose config -q
```

### 端口映射和服务
系统使用统一的 `docker-compose.yml` 配置，所有服务自动编排：

| 端口 | 服务 | 说明 |
|------|------|------|
| **8000** | HTTP API 服务 | RESTful API和Swagger文档 |
| **5858** | TCP 智能床垫数据接入 | 高频床垫数据接收 |
| **1883** | MQTT 消息代理 | 多设备MQTT数据接入 |
| **5432** | PostgreSQL 数据库 | 数据存储（内部端口）|

**服务架构：**
- **App服务**: Rust后端API，处理HTTP请求和TCP连接
- **PostgreSQL**: 主数据库，存储用户、设备、数据
- **MQTT Broker**: 消息代理，支持多协议设备接入
- **所有服务**: 通过Docker网络互联，自动发现和通信

## 📊 智能床垫接入

### 🛏️ 智能床垫特性
- **99.9%数据存储优化**: 从每天17万条减少到几十条有价值事件
- **自动设备注册**: 首次数据上报自动创建设备
- **事件驱动存储**: 只存储护理相关事件（上床/下床/体动/呼叫）
- **高频数据接入**: 支持每秒2次数据上报
- **零配置接入**: 设备插电即用，自动绑定

### 1. TCP连接测试
```bash
# 测试TCP连接（智能床垫专用端口）
telnet localhost 5858

# 发送模拟床垫数据（MessagePack格式）
echo -e "\x82\xa4\x74\x79\x70\x65\xa6\x6d\x61\x74\x74\x72\x65\x73\x73\xa4\x64\x61\x74\x61\x92\xcd\x12\x34\xcd\x56\x78" | nc localhost 5858
```

### 2. MQTT连接测试
```bash
# 订阅所有设备数据主题
mosquitto_sub -h localhost -t "remipedia/+/data"

# 发布心率监测数据
mosquitto_pub -h localhost -t "remipedia/heart-rate-001/data" -m '{
  "device_type": "heart_rate_monitor",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": [75, 76, 77]
}'

# 发布血氧监测数据
mosquitto_pub -h localhost -t "remipedia/spo2-001/data" -m '{
  "device_type": "spo2_monitor",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": [98, 97, 99]
}'
```

## 🎯 核心功能验证

### 1. 系统健康检查
```bash
# 检查API服务状态
curl http://localhost:8000/health

# 预期响应：{"status":"healthy","timestamp":"2024-01-01T12:00:00Z"}

# 访问Swagger文档
open http://localhost:8000/swagger
```

### 2. 设备自动注册
```bash
# 检查设备列表（首次访问应为空）
curl http://localhost:8000/api/v1/devices

# 发送测试数据后再次检查，设备应自动创建
curl http://localhost:8000/api/v1/devices
```

### 3. 智能床垫数据验证
```bash
# 查询上床事件（智能床垫检测到患者上床）
curl "http://localhost:8000/api/v1/data?data_type=bed_entry_event"

# 查询下床事件（智能床垫检测到患者下床）
curl "http://localhost:8000/api/v1/data?data_type=bed_exit_event"

# 查询体动事件（智能床垫检测到显著体动）
curl "http://localhost:8000/api/v1/data?data_type=significant_movement_event"

# 查询定期测量数据（每30分钟一次的生理数据快照）
curl "http://localhost:8000/api/v1/data?data_type=periodic_measurement"
```

### 3. 用户认证
```bash
# 注册用户
curl -X POST http://localhost:8000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin123",
    "email": "admin@example.com"
  }'

# 登录获取token
curl -X POST http://localhost:8000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin123"
  }'
```

## 🔧 运维管理

### 服务管理
```bash
# 启动服务
docker-compose up -d

# 停止服务
docker-compose down

# 重启服务
docker-compose restart

# 查看日志
docker-compose logs -f app
```

### 数据备份
```bash
# 备份数据库
docker-compose exec postgres pg_dump -U remipedia remipedia > backup.sql

# 恢复数据库
docker-compose exec -T postgres psql -U remipedia -d remipedia < backup.sql
```

### 性能监控
```bash
# 查看容器资源使用
docker stats

# 查看系统日志
docker-compose logs --tail=100
```

## 🚨 故障排除

### 🔍 快速诊断

**1. 服务状态检查**
```bash
# 查看所有服务状态
docker-compose ps

# 检查服务健康状态
curl -f http://localhost:8000/health || echo "API服务异常"

# 验证端口监听
netstat -tlnp | grep -E ':(8000|5858|1883|5432)'
```

**2. 智能床垫连接问题**
```bash
# 检查TCP端口5858
nc -zv localhost 5858

# 检查TCP服务日志
docker-compose logs app | grep -i tcp

# 模拟床垫数据测试
echo "test" | nc localhost 5858
```

### 🔧 常见问题解决

**1. 端口被占用**
```bash
# 检查端口占用情况
sudo lsof -i :8000  # HTTP API端口
sudo lsof -i :5858  # TCP床垫数据端口
sudo lsof -i :1883  # MQTT端口
sudo lsof -i :5432  # 数据库端口

# 终止占用进程
sudo kill -9 <PID>
```

**2. 数据库连接失败**
```bash
# 检查数据库状态
docker-compose exec postgres pg_isready -U remipedia

# 检查数据库日志
docker-compose logs postgres | tail -20

# 重置数据库（谨慎操作）
docker-compose down
docker volume rm remipedia_postgres_data
docker-compose up -d
```

**3. TCP连接失败**
```bash
# 检查TCP服务状态
docker-compose exec app netstat -tlnp | grep 5858

# 检查应用日志中的TCP相关错误
docker-compose logs app | grep -i "tcp\|mattress"

# 重启TCP服务
docker-compose restart app
```

**4. MQTT连接失败**
```bash
# 检查MQTT服务
mosquitto_pub -h localhost -t test -m "test message"

# 检查MQTT日志
docker-compose logs | grep -i mqtt

# 测试MQTT主题订阅/发布
mosquitto_sub -h localhost -t "remipedia/test/data" &
mosquitto_pub -h localhost -t "remipedia/test/data" -m '{"test": "data"}'
```

**5. 智能床垫数据过滤异常**
```bash
# 检查数据过滤日志
docker-compose logs app | grep -i "filter\|mattress\|event"

# 验证数据类型枚举
curl http://localhost:8000/api/v1/data/types

# 检查设备类型支持
curl http://localhost:8000/api/v1/devices/types
```

### 日志查看
```bash
# 应用日志
docker-compose logs app

# 数据库日志
docker-compose logs postgres

# MQTT日志
docker-compose logs mqtt
```

## 🔄 更新部署

### 1. 拉取最新代码
```bash
git pull origin main
```

### 2. 重新部署
```bash
# 使用部署脚本
./deploy.sh deploy

# 或者手动拉取更新
git pull origin main
docker-compose up -d --build
```

### 3. 零停机更新
```bash
# 使用滚动更新
docker-compose up -d --no-deps app
```

## 📚 相关文档

- [🏗️ 系统架构](docs/system_analysis.md) - 设备接入要点和前端开发指南
- [🛏️ 智能床垫集成](docs/smart_mattress_integration.md) - 详细技术实现文档
- [📖 API文档](http://localhost:8000/swagger) - 在线Swagger API文档
- [⚙️ CI/CD配置](SETUP.md) - GitHub Actions自动化部署配置

## 🎯 部署验证清单

部署完成后，请按顺序验证以下功能：

### ✅ 基础服务验证
- [ ] **HTTP API服务**: `curl http://localhost:8000/health` 返回健康状态
- [ ] **Swagger文档**: 浏览器访问 `http://localhost:8000/swagger` 正常显示
- [ ] **数据库服务**: PostgreSQL正常运行，无连接错误
- [ ] **MQTT服务**: 端口1883可连接，无拒绝错误

### ✅ 智能床垫功能验证
- [ ] **TCP端口5858**: 可建立TCP连接，无拒绝错误
- [ ] **设备自动注册**: 首次数据上报后自动创建设备记录
- [ ] **数据智能过滤**: 高频数据被过滤，只存储事件数据
- [ ] **事件类型支持**: 上床/下床/体动/定期测量事件正常生成

### ✅ 用户认证功能
- [ ] **用户注册**: 可通过API注册用户
- [ ] **用户登录**: 可获取JWT访问令牌
- [ ] **权限验证**: 需要认证的API正确验证token

### ✅ 数据查询功能
- [ ] **设备列表查询**: 可获取所有设备信息
- [ ] **数据历史查询**: 可按设备ID和数据类型查询历史数据
- [ ] **事件数据查询**: 可查询床垫事件数据（上床/下床/体动）

### ✅ 运维监控
- [ ] **容器状态**: 所有容器正常运行，无重启异常
- [ ] **日志输出**: 应用日志无严重错误信息
- [ ] **资源使用**: CPU和内存使用率在正常范围内

## 🎯 验证清单

部署完成后，请验证以下功能：

- [ ] HTTP API 正常访问 (http://localhost:8000/health)
- [ ] Swagger 文档可访问 (http://localhost:8000/swagger)
- [ ] TCP端口 5858 可连接
- [ ] MQTT端口 1883 可连接
- [ ] 数据库连接正常
- [ ] 设备自动注册功能正常
- [ ] 用户注册/登录功能正常
- [ ] 智能床垫数据过滤功能正常

## 📞 技术支持

### 🆘 紧急故障处理

**系统完全无法访问时：**
```bash
# 1. 检查Docker服务
sudo systemctl status docker

# 2. 重启所有服务
docker-compose down
docker-compose up -d

# 3. 检查日志定位问题
docker-compose logs --tail=50

# 4. 重置数据库（最后手段，会丢失数据）
docker-compose down -v
docker-compose up -d
```

**智能床垫数据异常时：**
```bash
# 检查TCP服务日志
docker-compose logs app | grep -A5 -B5 "mattress\|tcp\|5858"

# 验证数据过滤算法
docker-compose logs app | grep -i "filter\|event\|movement"

# 检查设备注册状态
curl -s http://localhost:8000/api/v1/devices | jq '.[] | {id: .id, type: .device_type, status: .status}'
```

### 📋 问题报告模板

遇到问题时，请提供以下信息：

```markdown
**环境信息：**
- 操作系统: Ubuntu 20.04
- Docker版本: docker --version
- 部署方式: 脚本部署/手动部署

**问题描述：**
- 具体现象: 
- 复现步骤:
- 预期结果:
- 实际结果:

**日志信息：**
```bash
# 服务状态
docker-compose ps

# 最近日志
docker-compose logs --tail=20

# 错误日志
docker-compose logs app | grep -i error
```

**配置文件：**
- .env.prod内容（隐藏敏感信息）
- docker-compose.yml是否修改过
```

### 🔗 相关资源

- **GitHub Issues**: [提交问题报告](https://github.com/your-org/remipedia/issues)
- **技术文档**: [详细架构文档](docs/system_analysis.md)
- **API参考**: [Swagger文档](http://localhost:8000/swagger)
- **部署指南**: [CI/CD配置](SETUP.md)

---

## 🎉 部署完成！

### 🏆 系统能力总结

**✅ 核心功能已就绪：**
- **智能床垫接入**: TCP端口5858，支持每秒2次高频数据
- **多设备MQTT接入**: 端口1883，支持各种IoT医疗设备
- **99.9%数据优化**: 事件驱动存储，从17万条/天减少到几十条
- **自动设备管理**: 首次数据上报自动注册和绑定
- **完整认证体系**: JWT-based用户认证和权限管理
- **RESTful API**: 完整的HTTP API和Swagger文档

**✅ 运维能力已就绪：**
- **容器化部署**: Docker Compose一键部署
- **健康监控**: 自动健康检查和故障恢复
- **日志管理**: 集中式日志收集和查询
- **数据备份**: 数据库自动备份和恢复
- **CI/CD集成**: GitHub Actions自动化部署

**✅ 智能床垫特色功能：**
- **状态智能识别**: 自动识别离床/在床/体动/呼叫状态
- **事件精准捕获**: 只存储有护理价值的事件数据
- **体动评分算法**: 1-10分制体动强度评估
- **定期健康快照**: 每30分钟自动保存生理数据
- **零配置接入**: 设备插电即用，无需手动配置

### 🚀 下一步行动

1. **连接智能床垫设备**：配置床垫连接到服务器IP:5858
2. **测试数据接入**：验证设备自动注册和数据过滤功能
3. **集成前端应用**：使用提供的API开发前端界面
4. **配置监控告警**：设置系统健康监控和异常告警
5. **培训操作人员**：指导护理人员使用新系统

**系统现已准备好接收智能床垫数据！** 🛏️✨