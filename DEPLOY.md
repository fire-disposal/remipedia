# 部署指南

## 系统要求

- Docker 20.10+
- Docker Compose 2.0+

## 一键部署

```bash
./deploy.sh
```

## 手动部署

```bash
docker-compose up -d
```

## 访问服务

| 服务 | 地址 |
|------|------|
| API | http://localhost:8000 |
| Swagger | http://localhost:8000/swagger |
| TCP床垫数据 | localhost:5858 |

## 数据库访问（SSH隧道）

数据库仅在Docker内部网络可访问，通过SSH隧道安全连接：

```bash
# 建立隧道
ssh -L 5432:postgres:5432 user@server

# 本地连接
psql -h localhost -U postgres -d remipedia
# 密码: postgres
```

## GitHub Actions 自动部署

需要配置4个Secrets：

| Secret | 说明 |
|--------|------|
| DEPLOY_HOST | 服务器IP |
| DEPLOY_USER | SSH用户 |
| DEPLOY_KEY | SSH私钥 |
| DEPLOY_PATH | 部署路径 |

推送tag或合并main分支自动触发部署。

## 常用命令

```bash
# 查看状态
docker-compose ps

# 查看日志
docker-compose logs -f app

# 重启服务
docker-compose restart

# 停止服务
docker-compose down

# 备份数据库
docker-compose exec postgres pg_dump -U postgres remipedia > backup.sql
```