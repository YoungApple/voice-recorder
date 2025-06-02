# Voice Recorder CLI Tool

A powerful voice recording and AI analysis tool built with Rust, designed for macOS. It features keyboard event monitoring, voice recording, AI transcription, PostgreSQL database storage, and a beautiful React-based web management interface.

## 🌟 Features

- **🎤 Real-time Voice Recording**: Control recording with keyboard shortcuts
- **🤖 AI Transcription**: Support for multiple AI providers including OpenAI Whisper, Whisper.cpp, and Ollama
- **📊 Smart Analysis**: Automatic extraction of ideas, tasks, and structured notes
- **🗄️ PostgreSQL Database**: Robust data persistence with full ACID compliance
- **🌐 Modern Web Interface**: React + TypeScript + Tailwind CSS management interface
- **🔌 RESTful API**: Complete REST API with comprehensive endpoints
- **⚙️ Flexible Configuration**: Support for multiple AI providers and models
- **🔍 Advanced Search**: Full-text search across sessions, transcripts, and analysis results
- **📈 Analytics Dashboard**: Statistics and insights about your recordings

## 📋 System Requirements

- macOS 10.15+
- Rust 1.70+
- PostgreSQL 12+
- Node.js 18+ (for web interface)
- Microphone permissions
- Accessibility permissions (for keyboard monitoring)

## 🚀 Quick Start

### Prerequisites

1. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Install PostgreSQL**:
   ```bash
   # Using Homebrew
   brew install postgresql
   brew services start postgresql
   
   # Or use Docker (recommended)
   ./scripts/docker-db.sh start
   ```

3. **Install Node.js** (for web interface):
   ```bash
   # Using Homebrew
   brew install node
   ```

### Setup

1. **Clone and Build**:
   ```bash
   git clone https://github.com/yourusername/voice-recorder.git
   cd voice-recorder
   cargo build --release
   ```

2. **Database Setup**:
   ```bash
   # Install sqlx-cli
   cargo install sqlx-cli --no-default-features --features postgres
   
   # Create database and run migrations
   export DATABASE_URL="postgresql://voice_recorder:password@localhost/voice_recorder"
   sqlx database create
   sqlx migrate run
   ```

3. **Web Interface Setup**:
   ```bash
   cd web
   npm install
   npm run build
   cd ..
   ```

4. **Set Permissions**:
   - Grant microphone access in System Preferences → Security & Privacy → Privacy → Microphone
   - Grant accessibility access in System Preferences → Security & Privacy → Privacy → Accessibility

5. **Configure AI Service**:
   ```bash
   # Configure OpenAI
   ./target/release/voice-recorder config set-openai-key "your-openai-api-key"
   
   # Configure Ollama
   ./target/release/voice-recorder config set-ollama-endpoint "http://localhost:11434/api/chat"
   ./target/release/voice-recorder config set-ollama-model-name "deepseek-coder"
   ```

6. **Start the Application**:
   ```bash
   # Start the web server (includes API and web interface)
   ./target/release/voice-recorder web --port 3000
   
   # Or start CLI recording mode
   ./target/release/voice-recorder start
   ```

## 🎯 Usage

### Recording Controls
- Press 'r': Start recording 🎤
- Press 'e': End recording ⏹️
- Press 'q': Quit program 👋

### Available Commands

```bash
# Start the voice recorder
./target/release/voice-recorder start

# Transcribe an audio file
./target/release/voice-recorder transcribe --file <path>

# Analyze a transcript
./target/release/voice-recorder analyze --file <path>

# Play an audio file
./target/release/voice-recorder play --file <path>

# List all recorded sessions
./target/release/voice-recorder list

# Show details of a specific session
./target/release/voice-recorder show --id <session-id>

# Delete a specific session
./target/release/voice-recorder delete --id <session-id>

# Export a session
./target/release/voice-recorder export --id <session-id> --format <format>

# Test Ollama analysis
./target/release/voice-recorder test-ollama --id <session-id>

# Start web interface
./target/release/voice-recorder web --port 3000
```

### Web Interface
```bash
# Start the web server
./target/release/voice-recorder web --port 3000

# Development mode (with hot reload)
cd web
npm run dev
```
Visit `http://localhost:3000` in your browser

## 🏗️ Architecture

### Backend (Rust)
- **Axum Web Framework**: High-performance async web server
- **PostgreSQL**: Primary database with full ACID compliance
- **SQLx**: Type-safe SQL queries with compile-time verification
- **Repository Pattern**: Clean separation of data access logic
- **Service Layer**: Business logic abstraction
- **RESTful API**: Complete CRUD operations for all entities

### Frontend (React + TypeScript)
- **React 19**: Modern React with latest features
- **TypeScript**: Type-safe frontend development
- **Tailwind CSS**: Utility-first CSS framework
- **Vite**: Fast build tool and development server
- **Axios**: HTTP client for API communication

### Database Schema

The application uses PostgreSQL with the following main tables:
- `sessions` - Recording sessions
- `audio_files` - Audio file metadata
- `transcripts` - AI transcription results
- `analysis_results` - AI analysis outputs
- `ideas` - Extracted ideas and insights
- `tasks` - Action items and todos
- `structured_notes` - Organized notes and summaries

## 📁 Data Structure

### Database Storage (Primary)
Data is stored in PostgreSQL with proper relationships and constraints.

### File Storage
Audio files are stored in `./local_storage/app_data/audio/` with metadata in the database.

## 🔧 Configuration

Configuration is managed through environment variables and config files:

### Database Configuration
```bash
DATABASE_URL=postgresql://voice_recorder:password@localhost/voice_recorder
```

### Server Configuration
```toml
[server]
host = "127.0.0.1"
port = 3000
cors_origins = ["http://localhost:3000", "http://localhost:5173"]
request_timeout_secs = 30
max_body_size = 52428800  # 50MB
```

### AI Providers
- **OpenAI Whisper**: For transcription and analysis
- **Ollama**: Local AI models (llama2, deepseek-coder, etc.)
- **Whisper.cpp**: Local whisper implementation

### Audio Settings
- Sample rate: 16000 Hz
- Channels: Mono
- Format: WAV
- Max file size: 50MB

## 🔌 API Endpoints

The application provides a comprehensive REST API:

### Sessions
- `GET /api/v1/sessions` - List all sessions
- `POST /api/v1/sessions` - Create new session
- `GET /api/v1/sessions/{id}` - Get session details
- `PATCH /api/v1/sessions/{id}` - Update session
- `DELETE /api/v1/sessions/{id}` - Delete session

### Transcripts
- `GET /api/v1/transcripts` - List transcripts
- `POST /api/v1/transcripts` - Create transcript
- `GET /api/v1/transcripts/{id}` - Get transcript
- `PATCH /api/v1/transcripts/{id}` - Update transcript

### Analysis
- `GET /api/v1/analysis` - List analysis results
- `POST /api/v1/analysis` - Create analysis
- `GET /api/v1/analysis/stats` - Get analysis statistics
- `GET /api/v1/analysis/types` - Get available analysis types

### Ideas & Tasks
- `GET /api/v1/ideas` - List extracted ideas
- `GET /api/v1/tasks` - List extracted tasks
- `GET /api/v1/notes` - List structured notes

## 🛠️ Development

### Running in Development Mode

1. **Start PostgreSQL**:
   ```bash
   ./scripts/docker-db.sh start
   ```

2. **Start Backend**:
   ```bash
   cargo run -- web --port 3000
   ```

3. **Start Frontend** (in another terminal):
   ```bash
   cd web
   npm run dev
   ```

### Database Management

```bash
# Run migrations
sqlx migrate run

# Create new migration
sqlx migrate add <migration_name>

# Database backup
./scripts/docker-db.sh backup

# Database restore
./scripts/docker-db.sh restore backup.sql
```

## 📚 Documentation

For detailed installation and usage instructions, please refer to [installation_guide.md](installation_guide.md).

---

# 语音录制器 CLI 工具

这是一个基于Rust开发的语音录制和AI分析工具，专为macOS设计。它能够监听键盘事件、录制语音、调用AI模型进行转录和分析，使用PostgreSQL数据库存储，并提供基于React的现代化Web管理界面。

## 🌟 功能特性

- **🎤 实时语音录制**: 使用键盘快捷键控制录制
- **🤖 AI转录**: 支持多种AI提供商，包括OpenAI Whisper、Whisper.cpp和Ollama
- **📊 智能分析**: 自动提取想法、任务和结构化笔记
- **🗄️ PostgreSQL数据库**: 强大的数据持久化，完整的ACID合规性
- **🌐 现代化Web界面**: React + TypeScript + Tailwind CSS管理界面
- **🔌 RESTful API**: 完整的REST API，包含全面的端点
- **⚙️ 灵活配置**: 支持多种AI提供商和模型
- **🔍 高级搜索**: 跨会话、转录和分析结果的全文搜索
- **📈 分析仪表板**: 录制统计和洞察

## 📋 系统要求

- macOS 10.15+
- Rust 1.70+
- PostgreSQL 12+
- Node.js 18+（用于Web界面）
- 麦克风权限
- 可访问权限（用于键盘监听）

## 🚀 快速开始

### 前置要求

1. **安装Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **安装PostgreSQL**:
   ```bash
   # 使用Homebrew
   brew install postgresql
   brew services start postgresql
   
   # 或使用Docker（推荐）
   ./scripts/docker-db.sh start
   ```

3. **安装Node.js**（用于Web界面）:
   ```bash
   # 使用Homebrew
   brew install node
   ```

### 设置

1. **克隆并编译**:
   ```bash
   git clone https://github.com/yourusername/voice-recorder.git
   cd voice-recorder
   cargo build --release
   ```

2. **数据库设置**:
   ```bash
   # 安装sqlx-cli
   cargo install sqlx-cli --no-default-features --features postgres
   
   # 创建数据库并运行迁移
   export DATABASE_URL="postgresql://voice_recorder:password@localhost/voice_recorder"
   sqlx database create
   sqlx migrate run
   ```

3. **Web界面设置**:
   ```bash
   cd web
   npm install
   npm run build
   cd ..
   ```

4. **设置权限**:
   - 在系统偏好设置 → 安全性与隐私 → 隐私 → 麦克风中授予权限
   - 在系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能中授予权限

5. **配置AI服务**:
   ```bash
   # 配置OpenAI
   ./target/release/voice-recorder config set-openai-key "your-openai-api-key"
   
   # 配置Ollama
   ./target/release/voice-recorder config set-ollama-endpoint "http://localhost:11434/api/chat"
   ./target/release/voice-recorder config set-ollama-model-name "deepseek-coder"
   ```

6. **启动应用程序**:
   ```bash
   # 启动Web服务器（包含API和Web界面）
   ./target/release/voice-recorder web --port 3000
   
   # 或启动CLI录制模式
   ./target/release/voice-recorder start
   ```

## 🎯 使用方法

### 录制控制
- 按 'r' 键: 开始录制 🎤
- 按 'e' 键: 结束录制 ⏹️
- 按 'q' 键: 退出程序 👋

### 可用命令

```bash
# 启动语音录制器
./target/release/voice-recorder start

# 转录音频文件
./target/release/voice-recorder transcribe --file <path>

# 分析转录文本
./target/release/voice-recorder analyze --file <path>

# 播放音频文件
./target/release/voice-recorder play --file <path>

# 列出所有录制会话
./target/release/voice-recorder list

# 显示特定会话详情
./target/release/voice-recorder show --id <session-id>

# 删除特定会话
./target/release/voice-recorder delete --id <session-id>

# 导出会话
./target/release/voice-recorder export --id <session-id> --format <format>

# 测试Ollama分析
./target/release/voice-recorder test-ollama --id <session-id>

# 启动Web界面
./target/release/voice-recorder web --port 3000
```

### Web界面
```bash
# 启动Web服务器
./target/release/voice-recorder web --port 3000

# 开发模式（热重载）
cd web
npm run dev
```
在浏览器中访问 `http://localhost:3000`

## 🏗️ 架构

### 后端（Rust）
- **Axum Web框架**: 高性能异步Web服务器
- **PostgreSQL**: 主数据库，完整的ACID合规性
- **SQLx**: 类型安全的SQL查询，编译时验证
- **Repository模式**: 数据访问逻辑的清晰分离
- **服务层**: 业务逻辑抽象
- **RESTful API**: 所有实体的完整CRUD操作

### 前端（React + TypeScript）
- **React 19**: 具有最新功能的现代React
- **TypeScript**: 类型安全的前端开发
- **Tailwind CSS**: 实用优先的CSS框架
- **Vite**: 快速构建工具和开发服务器
- **Axios**: API通信的HTTP客户端

### 数据库Schema

应用程序使用PostgreSQL，包含以下主要表：
- `sessions` - 录制会话
- `audio_files` - 音频文件元数据
- `transcripts` - AI转录结果
- `analysis_results` - AI分析输出
- `ideas` - 提取的想法和洞察
- `tasks` - 行动项和待办事项
- `structured_notes` - 组织化的笔记和摘要

## 📁 数据结构

### 数据库存储（主要）
数据存储在PostgreSQL中，具有适当的关系和约束。

### 文件存储
音频文件存储在 `./local_storage/app_data/audio/` 中，元数据在数据库中。

## 🔧 配置

配置通过环境变量和配置文件管理：

### 数据库配置
```bash
DATABASE_URL=postgresql://voice_recorder:password@localhost/voice_recorder
```

### 服务器配置
```toml
[server]
host = "127.0.0.1"
port = 3000
cors_origins = ["http://localhost:3000", "http://localhost:5173"]
request_timeout_secs = 30
max_body_size = 52428800  # 50MB
```

### AI提供商
- **OpenAI Whisper**: 用于转录和分析
- **Ollama**: 本地AI模型（llama2、deepseek-coder等）
- **Whisper.cpp**: 本地whisper实现

### 音频设置
- 采样率：16000 Hz
- 声道：单声道
- 格式：WAV
- 最大文件大小：50MB

## 🔌 API端点

应用程序提供全面的REST API：

### 会话
- `GET /api/v1/sessions` - 列出所有会话
- `POST /api/v1/sessions` - 创建新会话
- `GET /api/v1/sessions/{id}` - 获取会话详情
- `PATCH /api/v1/sessions/{id}` - 更新会话
- `DELETE /api/v1/sessions/{id}` - 删除会话

### 转录
- `GET /api/v1/transcripts` - 列出转录
- `POST /api/v1/transcripts` - 创建转录
- `GET /api/v1/transcripts/{id}` - 获取转录
- `PATCH /api/v1/transcripts/{id}` - 更新转录

### 分析
- `GET /api/v1/analysis` - 列出分析结果
- `POST /api/v1/analysis` - 创建分析
- `GET /api/v1/analysis/stats` - 获取分析统计
- `GET /api/v1/analysis/types` - 获取可用分析类型

### 想法和任务
- `GET /api/v1/ideas` - 列出提取的想法
- `GET /api/v1/tasks` - 列出提取的任务
- `GET /api/v1/notes` - 列出结构化笔记

## 🛠️ 开发

### 开发模式运行

1. **启动PostgreSQL**:
   ```bash
   ./scripts/docker-db.sh start
   ```

2. **启动后端**:
   ```bash
   cargo run -- web --port 3000
   ```

3. **启动前端**（在另一个终端）:
   ```bash
   cd web
   npm run dev
   ```

### 数据库管理

```bash
# 运行迁移
sqlx migrate run

# 创建新迁移
sqlx migrate add <migration_name>

# 数据库备份
./scripts/docker-db.sh backup

# 数据库恢复
./scripts/docker-db.sh restore backup.sql
```

## 📚 文档

详细的安装和使用说明，请参考 [installation_guide.md](installation_guide.md)。