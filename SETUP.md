# 🔧 GitHub Actions CI/CD 配置指南

## 🚀 开箱即用部署配置

### 1. 添加 GitHub Secrets

在 GitHub 仓库设置中添加以下 Secrets：

#### 🔐 服务器连接配置（必需）
```
SSH_HOST: 你的服务器IP地址或域名
SSH_USER: 服务器用户名（如：root 或 ubuntu）
SSH_KEY: 服务器的SSH私钥（完整内容）
SSH_PORT: SSH端口（默认22，可选）
```

#### 🔑 应用配置（必需）
```
DB_PASSWORD: PostgreSQL数据库密码（建议随机生成）
JWT_SECRET: JWT密钥（建议64位随机字符串）
```

#### 🐳 容器仓库配置（可选）
```
GITHUB_TOKEN: 自动生成，无需手动添加
```

### 2. 服务器准备工作

#### 安装 Docker（Linux服务器）
```bash
# Ubuntu/Debian
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

#### 创建部署目录
```bash
mkdir -p ~/remipedia
cd ~/remipedia
```

#### 创建SSH密钥（如果还没有）
```bash
ssh-keygen -t rsa -b 4096 -f ~/.ssh/id_rsa -N ""
```

### 3. 配置 GitHub Actions

#### 设置环境
1. 进入 GitHub 仓库 → Settings → Environments
2. 创建 `production` 环境
3. 添加保护规则（可选）：
   - 需要审查
   - 部署等待时间
   - 分支保护规则

#### 添加 Secrets
1. 进入 Settings → Secrets and variables → Actions
2. 点击 "New repository secret"
3. 按上面列表添加所有必需的 Secrets

### 4. 触发部署

#### 自动触发
- 推送代码到 `main` 或 `master` 分支
- 创建标签 `v*`（如：v1.0.0）

#### 手动触发
1. 进入 Actions 标签页
2. 选择 "🚀 Fast Deploy" 工作流
3. 点击 "Run workflow"
4. 选择分支并确认

## 📊 部署流程说明

### 构建阶段（约3-5分钟）
1. **代码检出**: 拉取最新代码
2. **Rust工具链**: 安装稳定版Rust和必要组件
3. **智能缓存**: 缓存Cargo依赖，加速后续构建
4. **代码质量**: 格式化检查和Clippy静态分析
5. **并行构建**: Release模式编译，优化性能
6. **并行测试**: 运行所有测试用例
7. **Docker构建**: 多架构镜像构建（amd64/arm64）
8. **镜像推送**: 推送到GitHub Container Registry

### 部署阶段（约2-3分钟）
1. **SSH连接**: 使用配置的密钥连接服务器
2. **代码更新**: 拉取最新代码到部署目录
3. **环境配置**: 自动生成.env.prod配置文件
4. **镜像更新**: 拉取最新容器镜像
5. **服务重启**: 零停机重启所有服务
6. **健康检查**: 验证API服务正常运行
7. **清理优化**: 清理旧镜像和容器

## 🔍 部署验证

### 查看部署状态
```bash
# 在服务器上查看服务状态
docker-compose ps

# 查看应用日志
docker-compose logs -f app

# 测试API
curl http://your-server-ip:8000/health
```

### 验证智能床垫功能
```bash
# 查询上床事件
curl "http://your-server-ip:8000/api/v1/data?data_type=bed_entry_event"

# 查询体动评分
curl "http://your-server-ip:8000/api/v1/data?data_type=significant_movement_event"
```

## 🚨 故障排除

### 部署失败常见原因

1. **SSH连接失败**
   - 检查 SSH_HOST、SSH_USER、SSH_KEY 是否正确
   - 确保服务器SSH服务正常运行
   - 检查防火墙设置

2. **Docker命令失败**
   - 确保服务器已安装Docker和Docker Compose
   - 检查用户是否有Docker权限
   - 查看详细日志：`docker-compose logs`

3. **健康检查失败**
   - 检查应用日志：`docker-compose logs app`
   - 验证数据库连接
   - 检查端口是否被占用

4. **数据库连接失败**
   - 检查 DB_PASSWORD 是否正确
   - 验证PostgreSQL服务状态
   - 查看数据库日志：`docker-compose logs postgres`

### 查看部署日志
1. 进入 GitHub Actions 页面
2. 点击最新的部署工作流
3. 查看详细的步骤日志

## 🔧 高级配置

### 自定义部署命令
修改 `.github/workflows/deploy.yml` 中的部署脚本部分。

### 多环境部署
可以创建多个环境（如：staging, production），每个环境使用不同的Secrets。

### 回滚机制
如果部署失败，可以：
1. 查看上一个成功的部署
2. 手动重新部署上一个版本
3. 使用 `./deploy.sh rollback` 命令回滚

## 📋 部署清单

部署前请确保：
- [ ] 已添加所有必需的 GitHub Secrets
- [ ] 服务器已安装 Docker 和 Docker Compose
- [ ] 服务器防火墙已开放必要端口
- [ ] 已创建部署目录（~/remipedia）
- [ ] SSH密钥已配置并可正常连接

部署完成后验证：
- [ ] GitHub Actions 工作流执行成功
- [ ] 所有服务正常运行
- [ ] API健康检查通过
- [ ] 智能床垫数据接入正常

---

**🎉 配置完成！**

现在只需要：
1. 填写服务器连接信息（SSH_HOST, SSH_USER, SSH_KEY）
2. 设置应用密钥（DB_PASSWORD, JWT_SECRET）
3. 推送代码到main分支

系统将自动完成构建、测试、部署全流程！