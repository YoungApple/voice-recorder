#!/bin/bash

# Voice Recorder 重构设置脚本
# 此脚本用于设置重构所需的基础环境

set -e

echo "🚀 开始 Voice Recorder 项目重构设置..."

# 检查必要的工具
echo "📋 检查必要工具..."

if ! command -v docker &> /dev/null; then
    echo "❌ Docker 未安装，请先安装 Docker"
    echo "   macOS: brew install --cask docker"
    echo "   Ubuntu: sudo apt-get install docker.io docker-compose"
    echo "   或访问: https://docs.docker.com/get-docker/"
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose 未安装，请先安装 Docker Compose"
    echo "   macOS: brew install docker-compose"
    echo "   Ubuntu: sudo apt-get install docker-compose"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo 未安装，请先安装 Rust"
    echo "   访问: https://rustup.rs/"
    exit 1
fi

# 检查Docker是否运行
if ! docker info &> /dev/null; then
    echo "❌ Docker 未运行，请启动 Docker Desktop 或 Docker 服务"
    exit 1
fi

echo "✅ 工具检查完成"

# 设置Docker数据库
echo "🗄️  设置Docker数据库..."

DB_NAME="voice_recorder"
DB_USER="voice_recorder_user"
DB_PASSWORD="voice_recorder_pass"
DB_PORT="5432"
CONTAINER_NAME="voice_recorder_postgres"

# 创建docker-compose.yml文件
echo "📝 创建docker-compose.yml..."

cat > docker-compose.yml << EOF
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    container_name: $CONTAINER_NAME
    environment:
      POSTGRES_DB: $DB_NAME
      POSTGRES_USER: $DB_USER
      POSTGRES_PASSWORD: $DB_PASSWORD
      POSTGRES_INITDB_ARGS: "--encoding=UTF-8 --lc-collate=C --lc-ctype=C"
    ports:
      - "$DB_PORT:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $DB_USER -d $DB_NAME"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
    driver: local
EOF

echo "✅ docker-compose.yml 创建完成"

# 启动PostgreSQL容器
echo "🚀 启动PostgreSQL容器..."

# 检查容器是否已经运行
if docker ps | grep -q $CONTAINER_NAME; then
    echo "⚠️  PostgreSQL容器已在运行"
else
    # 停止并删除可能存在的同名容器
    if docker ps -a | grep -q $CONTAINER_NAME; then
        echo "🔄 停止并删除现有容器..."
        docker stop $CONTAINER_NAME &> /dev/null || true
        docker rm $CONTAINER_NAME &> /dev/null || true
    fi
    
    # 启动新容器
    docker-compose up -d postgres
    
    # 等待数据库启动
    echo "⏳ 等待数据库启动..."
    for i in {1..30}; do
        if docker exec $CONTAINER_NAME pg_isready -U $DB_USER -d $DB_NAME &> /dev/null; then
            echo "✅ PostgreSQL容器启动成功"
            break
        fi
        if [ $i -eq 30 ]; then
            echo "❌ PostgreSQL容器启动超时"
            docker-compose logs postgres
            exit 1
        fi
        sleep 2
    done
fi

# 创建目录结构
echo "📁 创建项目目录结构..."

mkdir -p src/api/{handlers,middleware,models}
mkdir -p src/services/{ollama,audio,analysis}
mkdir -p src/repository
mkdir -p src/database
mkdir -p migrations
mkdir -p scripts
mkdir -p docs

echo "✅ 目录结构创建完成"

# 创建配置文件
echo "⚙️  创建配置文件..."

cat > config.toml << EOF
[database]
url = "postgresql://$DB_USER:$DB_PASSWORD@localhost:$DB_PORT/$DB_NAME"
max_connections = 10
min_connections = 1
connect_timeout = 30
idle_timeout = 600

[ollama]
endpoint = "http://localhost:11434"
max_connections = 5
request_timeout = 120
retry_attempts = 3
retry_delay = 2

[api]
port = 3000
max_request_size = "10MB"
rate_limit_requests = 100
rate_limit_window = 60

[storage]
audio_path = "./storage/audio"
max_file_size = "100MB"
allowed_formats = ["wav", "mp3", "m4a"]

[logging]
level = "info"
format = "json"
file = "./logs/app.log"

[monitoring]
metrics_enabled = true
metrics_port = 9090
health_check_interval = 30
EOF

echo "✅ 配置文件 config.toml 创建完成"

# 创建初始数据库迁移文件
echo "🗃️  创建数据库迁移文件..."

cat > migrations/001_initial_schema.sql << 'EOF'
-- 初始数据库Schema
-- 会话表
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    duration_ms BIGINT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    metadata JSONB
);

-- 音频文件表
CREATE TABLE audio_files (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    file_path VARCHAR(500) NOT NULL,
    file_size BIGINT NOT NULL,
    format VARCHAR(20) NOT NULL,
    sample_rate INTEGER,
    channels INTEGER,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    checksum VARCHAR(64)
);

-- 转录文本表
CREATE TABLE transcripts (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    language VARCHAR(10),
    confidence_score DECIMAL(3,2),
    provider VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    processing_time_ms INTEGER
);

-- 分析结果表
CREATE TABLE analysis_results (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    title VARCHAR(255),
    summary TEXT,
    provider VARCHAR(50) NOT NULL,
    model_version VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    processing_time_ms INTEGER
);

-- 想法/观点表
CREATE TABLE ideas (
    id UUID PRIMARY KEY,
    analysis_id UUID NOT NULL REFERENCES analysis_results(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    category VARCHAR(100),
    priority INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- 任务表
CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    analysis_id UUID NOT NULL REFERENCES analysis_results(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    priority VARCHAR(20) NOT NULL CHECK (priority IN ('Low', 'Medium', 'High', 'Urgent')),
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'cancelled')),
    due_date TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- 结构化笔记表
CREATE TABLE structured_notes (
    id UUID PRIMARY KEY,
    analysis_id UUID NOT NULL REFERENCES analysis_results(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    note_type VARCHAR(20) NOT NULL CHECK (note_type IN ('Meeting', 'Brainstorm', 'Decision', 'Action', 'Reference')),
    tags TEXT[],
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
EOF

cat > migrations/002_add_indexes.sql << 'EOF'
-- 添加索引以提升查询性能
CREATE INDEX idx_sessions_created_at ON sessions(created_at);
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_audio_files_session_id ON audio_files(session_id);
CREATE INDEX idx_transcripts_session_id ON transcripts(session_id);
CREATE INDEX idx_analysis_results_session_id ON analysis_results(session_id);
CREATE INDEX idx_ideas_analysis_id ON ideas(analysis_id);
CREATE INDEX idx_tasks_analysis_id ON tasks(analysis_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_structured_notes_analysis_id ON structured_notes(analysis_id);
CREATE INDEX idx_structured_notes_note_type ON structured_notes(note_type);
CREATE INDEX idx_structured_notes_tags ON structured_notes USING GIN(tags);
EOF

echo "✅ 数据库迁移文件创建完成"

# 备份当前的 Cargo.toml
echo "💾 备份当前配置..."
cp Cargo.toml Cargo.toml.backup
echo "✅ Cargo.toml 已备份为 Cargo.toml.backup"

# 创建新的 Cargo.toml
echo "📦 更新项目依赖..."

cat > Cargo.toml << 'EOF'
[package]
name = "voice-recorder"
version = "0.2.0"
edition = "2021"

[dependencies]
# 异步运行时
tokio = { version = "1.0", features = ["full"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 命令行
clap = { version = "4.0", features = ["derive"] }

# 时间处理
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1.0", features = ["v4", "serde"] }

# HTTP客户端
reqwest = { version = "0.11", features = ["json", "multipart"] }

# 音频处理
rodio = "0.17"
cpal = "0.15"
hound = "3.5"

# 键盘监听
rdev = "0.4"

# 日志
log = "0.4"
env_logger = "0.11"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }

# AI集成
async-openai = "0.18.2"

# 并发
crossbeam-channel = "0.5"

# 错误处理
anyhow = "1.0"
thiserror = "1.0"

# 系统目录
dirs = "5.0"

# Web框架
axum = { version = "0.7", features = ["multipart"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors"] }

# 文件嵌入
include_dir = "0.7"

# 静态变量
lazy_static = "1.4"

# 数据库
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json", "migrate"] }
deadpool-postgres = "0.12"

# 异步特征
async-trait = "0.1"

# 配置管理
config = "0.14"
toml = "0.8"

# 监控和指标
prometheus = "0.13"

# API文档
utoipa = { version = "4.0", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "4.0", features = ["axum"] }

[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "0.9"
core-audio-types-rs = "0.3.4"
EOF

echo "✅ Cargo.toml 更新完成"

# 创建环境变量文件
echo "🔧 创建环境配置..."

cat > .env << EOF
# 数据库配置
DATABASE_URL=postgresql://$DB_USER:$DB_PASSWORD@localhost:$DB_PORT/$DB_NAME

# 应用配置
RUST_LOG=info
APP_PORT=3000

# Ollama配置
OLLAMA_ENDPOINT=http://localhost:11434

# 存储配置
STORAGE_PATH=./storage
EOF

echo "✅ .env 文件创建完成"

# 创建日志目录
mkdir -p logs
mkdir -p storage/audio
mkdir -p storage/temp

echo "📁 存储目录创建完成"

# 运行数据库迁移
echo "🗄️  运行数据库迁移..."

# 安装 sqlx-cli 如果未安装
if ! command -v sqlx &> /dev/null; then
    echo "📦 安装 sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

# 等待数据库完全就绪
echo "⏳ 确保数据库完全就绪..."
sleep 5

# 运行迁移
export DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@localhost:$DB_PORT/$DB_NAME"
echo "🔄 运行数据库迁移..."
if sqlx migrate run; then
    echo "✅ 数据库迁移完成"
else
    echo "❌ 数据库迁移失败，检查容器状态..."
    docker-compose logs postgres
    echo "尝试手动连接测试:"
    echo "docker exec -it $CONTAINER_NAME psql -U $DB_USER -d $DB_NAME"
    exit 1
fi

# 创建 README 更新
echo "📚 更新项目文档..."

cat > REFACTOR_STATUS.md << 'EOF'
# 重构状态

## 已完成

✅ 数据库设置和Schema设计  
✅ 项目依赖更新  
✅ 基础目录结构创建  
✅ 配置文件设置  
✅ 数据库迁移系统  

## 进行中

🔄 Repository层实现  
🔄 服务层重构  

## 待完成

⏳ API层重构  
⏳ 中间件实现  
⏳ 数据迁移工具  
⏳ 测试覆盖  

## 下一步

1. 实现基础的Repository特征和PostgreSQL实现
2. 重构Ollama服务层
3. 更新API端点以使用新的数据层

## 配置

- 数据库: PostgreSQL (Docker容器 localhost:5432/voice_recorder)
- 配置文件: config.toml
- 环境变量: .env
- 迁移文件: migrations/
- Docker配置: docker-compose.yml

## 运行

```bash
# 启动数据库容器
docker-compose up -d postgres

# 检查数据库连接
sqlx database create

# 运行迁移
sqlx migrate run

# 构建项目
cargo build

# 运行项目
cargo run -- web

# 停止数据库容器
docker-compose down
```
EOF

echo "✅ 重构状态文档创建完成"

echo ""
echo "🎉 重构设置完成！"
echo ""
echo "📋 总结:"
echo "   ✅ Docker PostgreSQL 数据库设置完成"
echo "   ✅ docker-compose.yml 配置文件创建完成"
echo "   ✅ 项目依赖更新完成"
echo "   ✅ 目录结构创建完成"
echo "   ✅ 配置文件创建完成"
echo "   ✅ 数据库迁移完成"
echo ""
echo "🚀 下一步:"
echo "   1. 查看 REFACTOR_PLAN.md 了解详细重构计划"
echo "   2. 查看 REFACTOR_STATUS.md 了解当前状态"
echo "   3. 开始实现 Repository 层"
echo ""
echo "💡 有用的命令:"
echo "   docker-compose up -d postgres    # 启动数据库容器"
echo "   docker-compose down             # 停止数据库容器"
echo "   docker-compose logs postgres    # 查看数据库日志"
echo "   cargo build                     # 构建项目"
echo "   cargo test                      # 运行测试"
echo "   sqlx migrate run                # 运行数据库迁移"
echo "   cargo run -- web                # 启动Web服务"
echo ""
echo "🐳 Docker 数据库管理:"
echo "   容器名称: $CONTAINER_NAME"
echo "   数据库: $DB_NAME"
echo "   用户: $DB_USER"
echo "   端口: localhost:$DB_PORT"
echo "   连接测试: docker exec -it $CONTAINER_NAME psql -U $DB_USER -d $DB_NAME"
echo ""