#!/bin/bash
# 🚀 一键部署脚本 - 现代高效部署

set -euo pipefail

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置变量
ENV_FILE=".env.prod"
BACKUP_DIR="./backups"
LOG_FILE="./deploy.log"

# 日志函数
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
    exit 1
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

# 检查依赖
check_dependencies() {
    log "检查部署依赖..."
    
    command -v docker >/dev/null 2>&1 || error "Docker 未安装"
    command -v docker-compose >/dev/null 2>&1 || error "Docker Compose 未安装"
    command -v curl >/dev/null 2>&1 || error "curl 未安装"
    
    success "依赖检查通过"
}

# 创建环境文件
create_env_file() {
    if [[ ! -f "$ENV_FILE" ]]; then
        log "创建环境配置文件..."
        
        cat > "$ENV_FILE" << EOF
# 🔑 数据库配置
DB_PASSWORD=$(openssl rand -base64 32)

# 🔐 JWT 配置
JWT_SECRET=$(openssl rand -base64 64)

# 🏷️ 应用配置
APP_ENV=production
APP_PORT=8000
APP_TCP_PORT=5858
EOF
        warning "已生成新的环境配置文件，请妥善保管"
    else
        log "使用现有的环境配置文件"
    fi
}

# 数据备份
backup_data() {
    log "备份现有数据..."
    
    mkdir -p "$BACKUP_DIR"
    local backup_name="backup_$(date +%Y%m%d_%H%M%S)"
    
    if docker-compose ps | grep -q "postgres"; then
        log "备份 PostgreSQL 数据..."
        docker-compose exec -T postgres pg_dump -U remipedia remipedia > "$BACKUP_DIR/${backup_name}.sql"
        success "数据库备份完成: ${backup_name}.sql"
    fi
}

# 拉取最新镜像
pull_images() {
    log "拉取最新镜像..."
    
    # 获取环境变量中的镜像配置
    local registry="${REGISTRY:-ghcr.io}"
    local image="${IMAGE:-remipedia/remipedia-iot}"
    local tag="${IMAGE_TAG:-latest}"
    
    log "拉取镜像: ${registry}/${image}:${tag}"
    docker pull "${registry}/${image}:${tag}"
    
    # 更新docker-compose.yml中的镜像引用
    if [[ -f "docker-compose.yml" ]]; then
        sed -i "s|image:.*remipedia-iot.*|image: ${registry}/${image}:${tag}|g" docker-compose.yml
        log "已更新 docker-compose.yml 镜像版本"
    fi
    
    success "镜像拉取完成"
}

# 部署应用
deploy() {
    log "开始部署应用..."
    
    # 停止现有服务
    log "停止现有服务..."
    docker-compose down --remove-orphans
    
    # 启动新服务
    log "启动新服务..."
    docker-compose up -d
    
    # 等待服务就绪
    log "等待服务就绪..."
    sleep 10
    
    # 健康检查
    health_check
    
    success "部署完成！"
}

# 健康检查
health_check() {
    log "执行健康检查..."
    
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if curl -f http://localhost:8000/health >/dev/null 2>&1; then
            success "应用服务健康检查通过"
            break
        fi
        
        log "等待应用服务就绪... (尝试 $attempt/$max_attempts)"
        sleep 10
        ((attempt++))
    done
    
    if [[ $attempt -gt $max_attempts ]]; then
        error "应用服务健康检查失败"
    fi
    
    # 检查数据库
    if docker-compose exec -T postgres pg_isready -U remipedia >/dev/null 2>&1; then
        success "数据库服务健康检查通过"
    else
        error "数据库服务健康检查失败"
    fi
    
    # 检查MQTT
    if docker-compose exec -T mqtt mosquitto_pub -t test -m test -h localhost >/dev/null 2>&1; then
        success "MQTT服务健康检查通过"
    else
        warning "MQTT服务健康检查失败（可选服务）"
    fi
}

# 显示状态
show_status() {
    log "服务状态："
    docker-compose ps
    
    log "端口状态："
    netstat -tlnp 2>/dev/null | grep -E ":(8000|5858|5432|1883)" || ss -tlnp | grep -E ":(8000|5858|5432|1883)"
    
    log "应用日志（最近10行）："
    docker-compose logs --tail=10 app
}

# 清理旧数据
cleanup() {
    log "清理旧镜像和容器..."
    docker system prune -f
    success "清理完成"
}

# 主函数
main() {
    log "🚀 开始部署 Remipedia IoT 健康平台..."
    
    case "${1:-deploy}" in
        deploy)
            check_dependencies
            create_env_file
            backup_data
            pull_images
            deploy
            show_status
            cleanup
            ;;
        status)
            show_status
            ;;
        rollback)
            rollback
            ;;
        cleanup)
            cleanup
            ;;
        *)
            echo "用法: $0 {deploy|status|rollback|cleanup}"
            echo "  deploy   - 完整部署流程"
            echo "  status   - 显示服务状态"
            echo "  rollback - 回滚到上一个版本"
            echo "  cleanup  - 清理旧镜像和容器"
            exit 1
            ;;
    esac
    
    success "🎉 部署操作完成！"
    log "访问地址：http://localhost:8000"
    log "API文档：http://localhost:8000/swagger"
}

# 错误处理
trap 'error "部署过程中发生错误"' ERR

# 运行主函数
main "$@"

# 回滚功能
rollback() {
    warning "执行回滚操作..."
    
    if [[ -f "$BACKUP_DIR/latest.sql" ]]; then
        log "恢复数据库..."
        docker-compose -f "$COMPOSE_FILE" exec -T postgres psql -U remipedia -d remipedia < "$BACKUP_DIR/latest.sql"
        success "数据库恢复完成"
    fi
    
    log "重新启动服务..."
    docker-compose -f "$COMPOSE_FILE" restart
    
    success "回滚完成"
}

# 主函数
main() {
    log "🚀 开始部署 Remipedia IoT 健康平台..."
    
    case "${1:-deploy}" in
        deploy)
            check_dependencies
            create_env_file
            backup_data
            pull_images
            deploy
            show_status
            cleanup
            ;;
        status)
            show_status
            ;;
        rollback)
            rollback
            ;;
        cleanup)
            cleanup
            ;;
        *)
            echo "用法: $0 {deploy|status|rollback|cleanup}"
            echo "  deploy   - 完整部署流程"
            echo "  status   - 显示服务状态"
            echo "  rollback - 回滚到上一个版本"
            echo "  cleanup  - 清理旧镜像和容器"
            exit 1
            ;;
    esac
    
    success "🎉 部署操作完成！"
    log "访问地址：http://localhost:8000"
    log "API文档：http://localhost:8000/swagger"
}

# 错误处理
trap 'error "部署过程中发生错误"' ERR

# 运行主函数
main "$@"