#!/bin/bash
# 一键部署脚本

set -euo pipefail

log() { echo "[$(date +'%H:%M:%S')] $1"; }
error() { echo "[ERROR] $1" >&2; exit 1; }

# 检查依赖
command -v docker >/dev/null 2>&1 || error "Docker 未安装"
command -v docker-compose >/dev/null 2>&1 || error "Docker Compose 未安装"

# 部署
log "🚀 开始部署..."
docker-compose pull
docker-compose up -d

# 等待就绪
log "⏳ 等待服务启动..."
sleep 10

# 健康检查
for i in {1..30}; do
    if curl -sf http://localhost:8000/health >/dev/null 2>&1; then
        log "✅ 部署完成！"
        log "🌐 http://localhost:8000"
        log "📚 http://localhost:8000/swagger"
        docker-compose ps
        exit 0
    fi
    sleep 2
done

error "健康检查失败，查看日志: docker-compose logs"