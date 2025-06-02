#!/bin/bash

# Docker PostgreSQL 数据库管理脚本
# 用于管理 Voice Recorder 项目的 PostgreSQL 容器

set -e

# 配置变量
DB_NAME="voice_recorder"
DB_USER="voice_recorder_user"
DB_PASSWORD="voice_recorder_pass"
DB_PORT="5432"
CONTAINER_NAME="voice_recorder_postgres"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 帮助信息
show_help() {
    echo "Voice Recorder PostgreSQL Docker 管理脚本"
    echo ""
    echo "用法: $0 [命令]"
    echo ""
    echo "命令:"
    echo "  start     启动数据库容器"
    echo "  stop      停止数据库容器"
    echo "  restart   重启数据库容器"
    echo "  status    查看容器状态"
    echo "  logs      查看容器日志"
    echo "  connect   连接到数据库"
    echo "  backup    备份数据库"
    echo "  restore   恢复数据库 (需要指定备份文件)"
    echo "  clean     清理容器和数据卷"
    echo "  reset     重置数据库 (删除所有数据)"
    echo "  help      显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0 start                    # 启动数据库"
    echo "  $0 backup                   # 备份数据库"
    echo "  $0 restore backup.sql       # 恢复数据库"
}

# 检查Docker是否运行
check_docker() {
    if ! docker info &> /dev/null; then
        echo -e "${RED}❌ Docker 未运行，请启动 Docker Desktop 或 Docker 服务${NC}"
        exit 1
    fi
}

# 启动数据库容器
start_db() {
    echo -e "${BLUE}🚀 启动PostgreSQL容器...${NC}"
    
    if docker ps | grep -q $CONTAINER_NAME; then
        echo -e "${YELLOW}⚠️  PostgreSQL容器已在运行${NC}"
        return 0
    fi
    
    if ! [ -f "docker-compose.yml" ]; then
        echo -e "${RED}❌ docker-compose.yml 文件不存在，请先运行 setup_refactor.sh${NC}"
        exit 1
    fi
    
    docker-compose up -d postgres
    
    # 等待数据库启动
    echo -e "${BLUE}⏳ 等待数据库启动...${NC}"
    for i in {1..30}; do
        if docker exec $CONTAINER_NAME pg_isready -U $DB_USER -d $DB_NAME &> /dev/null; then
            echo -e "${GREEN}✅ PostgreSQL容器启动成功${NC}"
            return 0
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}❌ PostgreSQL容器启动超时${NC}"
            docker-compose logs postgres
            exit 1
        fi
        sleep 2
    done
}

# 停止数据库容器
stop_db() {
    echo -e "${BLUE}🛑 停止PostgreSQL容器...${NC}"
    
    if ! docker ps | grep -q $CONTAINER_NAME; then
        echo -e "${YELLOW}⚠️  PostgreSQL容器未运行${NC}"
        return 0
    fi
    
    docker-compose stop postgres
    echo -e "${GREEN}✅ PostgreSQL容器已停止${NC}"
}

# 重启数据库容器
restart_db() {
    echo -e "${BLUE}🔄 重启PostgreSQL容器...${NC}"
    stop_db
    sleep 2
    start_db
}

# 查看容器状态
show_status() {
    echo -e "${BLUE}📊 容器状态:${NC}"
    echo ""
    
    if docker ps | grep -q $CONTAINER_NAME; then
        echo -e "${GREEN}✅ 容器状态: 运行中${NC}"
        echo -e "${BLUE}📋 容器信息:${NC}"
        docker ps --filter "name=$CONTAINER_NAME" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
        echo ""
        echo -e "${BLUE}💾 数据卷信息:${NC}"
        docker volume ls --filter "name=voice-recorder_postgres_data"
    elif docker ps -a | grep -q $CONTAINER_NAME; then
        echo -e "${YELLOW}⚠️  容器状态: 已停止${NC}"
        docker ps -a --filter "name=$CONTAINER_NAME" --format "table {{.Names}}\t{{.Status}}"
    else
        echo -e "${RED}❌ 容器不存在${NC}"
    fi
}

# 查看容器日志
show_logs() {
    echo -e "${BLUE}📝 PostgreSQL容器日志:${NC}"
    
    if ! docker ps -a | grep -q $CONTAINER_NAME; then
        echo -e "${RED}❌ 容器不存在${NC}"
        exit 1
    fi
    
    docker-compose logs postgres
}

# 连接到数据库
connect_db() {
    echo -e "${BLUE}🔗 连接到PostgreSQL数据库...${NC}"
    
    if ! docker ps | grep -q $CONTAINER_NAME; then
        echo -e "${RED}❌ PostgreSQL容器未运行，请先启动容器${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}连接信息:${NC}"
    echo "  数据库: $DB_NAME"
    echo "  用户: $DB_USER"
    echo "  主机: localhost:$DB_PORT"
    echo ""
    echo -e "${YELLOW}提示: 输入 \\q 退出数据库连接${NC}"
    echo ""
    
    docker exec -it $CONTAINER_NAME psql -U $DB_USER -d $DB_NAME
}

# 备份数据库
backup_db() {
    echo -e "${BLUE}💾 备份数据库...${NC}"
    
    if ! docker ps | grep -q $CONTAINER_NAME; then
        echo -e "${RED}❌ PostgreSQL容器未运行，请先启动容器${NC}"
        exit 1
    fi
    
    # 创建备份目录
    mkdir -p backups
    
    # 生成备份文件名
    BACKUP_FILE="backups/voice_recorder_$(date +%Y%m%d_%H%M%S).sql"
    
    # 执行备份
    docker exec $CONTAINER_NAME pg_dump -U $DB_USER -d $DB_NAME > $BACKUP_FILE
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ 数据库备份成功: $BACKUP_FILE${NC}"
        echo -e "${BLUE}📊 备份文件大小: $(du -h $BACKUP_FILE | cut -f1)${NC}"
    else
        echo -e "${RED}❌ 数据库备份失败${NC}"
        exit 1
    fi
}

# 恢复数据库
restore_db() {
    local backup_file=$1
    
    if [ -z "$backup_file" ]; then
        echo -e "${RED}❌ 请指定备份文件${NC}"
        echo "用法: $0 restore <backup_file>"
        exit 1
    fi
    
    if [ ! -f "$backup_file" ]; then
        echo -e "${RED}❌ 备份文件不存在: $backup_file${NC}"
        exit 1
    fi
    
    echo -e "${BLUE}🔄 恢复数据库...${NC}"
    
    if ! docker ps | grep -q $CONTAINER_NAME; then
        echo -e "${RED}❌ PostgreSQL容器未运行，请先启动容器${NC}"
        exit 1
    fi
    
    # 确认操作
    echo -e "${YELLOW}⚠️  警告: 此操作将覆盖现有数据库内容${NC}"
    read -p "确认继续? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}操作已取消${NC}"
        exit 0
    fi
    
    # 执行恢复
    docker exec -i $CONTAINER_NAME psql -U $DB_USER -d $DB_NAME < $backup_file
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ 数据库恢复成功${NC}"
    else
        echo -e "${RED}❌ 数据库恢复失败${NC}"
        exit 1
    fi
}

# 清理容器和数据卷
clean_db() {
    echo -e "${BLUE}🧹 清理容器和数据卷...${NC}"
    
    # 确认操作
    echo -e "${YELLOW}⚠️  警告: 此操作将删除容器和所有数据${NC}"
    read -p "确认继续? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}操作已取消${NC}"
        exit 0
    fi
    
    # 停止并删除容器
    docker-compose down -v
    
    # 删除数据卷
    docker volume rm voice-recorder_postgres_data 2>/dev/null || true
    
    echo -e "${GREEN}✅ 清理完成${NC}"
}

# 重置数据库
reset_db() {
    echo -e "${BLUE}🔄 重置数据库...${NC}"
    
    # 确认操作
    echo -e "${YELLOW}⚠️  警告: 此操作将删除所有数据并重新初始化数据库${NC}"
    read -p "确认继续? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}操作已取消${NC}"
        exit 0
    fi
    
    # 清理并重新启动
    clean_db
    sleep 2
    start_db
    
    # 运行迁移
    if [ -f "migrations/001_initial_schema.sql" ]; then
        echo -e "${BLUE}🔄 运行数据库迁移...${NC}"
        export DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@localhost:$DB_PORT/$DB_NAME"
        if command -v sqlx &> /dev/null; then
            sqlx migrate run
            echo -e "${GREEN}✅ 数据库迁移完成${NC}"
        else
            echo -e "${YELLOW}⚠️  sqlx-cli 未安装，请手动运行迁移${NC}"
        fi
    fi
}

# 主函数
main() {
    check_docker
    
    case "${1:-help}" in
        start)
            start_db
            ;;
        stop)
            stop_db
            ;;
        restart)
            restart_db
            ;;
        status)
            show_status
            ;;
        logs)
            show_logs
            ;;
        connect)
            connect_db
            ;;
        backup)
            backup_db
            ;;
        restore)
            restore_db "$2"
            ;;
        clean)
            clean_db
            ;;
        reset)
            reset_db
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            echo -e "${RED}❌ 未知命令: $1${NC}"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# 运行主函数
main "$@"