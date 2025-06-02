# Voice Recorder 项目重构计划

## 概述

本文档详细描述了 Voice Recorder 项目的重构计划，主要包含三个核心改进方向：

1. **接口调用重构** - 统一API接口设计和错误处理
2. **Ollama本地调用优化** - 改进本地AI模型集成
3. **存储层抽象** - 设计数据库Schema和接口抽象

## 当前架构分析

### 现有问题

1. **接口层问题**
   - Web API 路由分散在 `web.rs` 中，缺乏统一的接口抽象
   - 错误处理不一致，返回格式不统一
   - 缺乏中间件支持（认证、日志、限流等）

2. **Ollama集成问题**
   - 直接在业务逻辑中调用 HTTP 客户端
   - 缺乏连接池和重试机制
   - 语言检测逻辑耦合在分析函数中

3. **存储层问题**
   - 当前使用文件系统存储，缺乏数据库支持
   - 数据结构分散，缺乏统一的数据访问层
   - 没有数据迁移和版本管理机制

## 重构方案

### 1. 接口调用重构

#### 1.1 创建统一的API层

```
src/
├── api/
│   ├── mod.rs           # API模块入口
│   ├── handlers/        # 请求处理器
│   │   ├── mod.rs
│   │   ├── session.rs   # 会话相关API
│   │   ├── audio.rs     # 音频相关API
│   │   ├── analysis.rs  # 分析相关API
│   │   └── config.rs    # 配置相关API
│   ├── middleware/      # 中间件
│   │   ├── mod.rs
│   │   ├── auth.rs      # 认证中间件
│   │   ├── logging.rs   # 日志中间件
│   │   └── cors.rs      # CORS中间件
│   ├── models/          # API数据模型
│   │   ├── mod.rs
│   │   ├── request.rs   # 请求模型
│   │   ├── response.rs  # 响应模型
│   │   └── error.rs     # 错误模型
│   └── routes.rs        # 路由定义
```

#### 1.2 统一响应格式

```rust
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
}

#[derive(Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}
```

#### 1.3 错误处理机制

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Session not found: {id}")]
    SessionNotFound { id: String },
    
    #[error("Audio processing failed: {reason}")]
    AudioProcessingError { reason: String },
    
    #[error("AI analysis failed: {reason}")]
    AnalysisError { reason: String },
    
    #[error("Storage error: {reason}")]
    StorageError { reason: String },
}
```

### 2. Ollama本地调用优化

#### 2.1 创建Ollama服务层

```
src/
├── services/
│   ├── mod.rs
│   ├── ollama/
│   │   ├── mod.rs
│   │   ├── client.rs    # Ollama客户端
│   │   ├── models.rs    # 数据模型
│   │   ├── prompts.rs   # Prompt管理
│   │   └── language.rs  # 语言检测
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── recorder.rs  # 录音服务
│   │   └── processor.rs # 音频处理
│   └── analysis/
│       ├── mod.rs
│       └── analyzer.rs  # 分析服务
```

#### 2.2 Ollama客户端重构

```rust
pub struct OllamaService {
    client: Arc<reqwest::Client>,
    config: OllamaConfig,
    connection_pool: Arc<ConnectionPool>,
}

impl OllamaService {
    pub async fn new(config: OllamaConfig) -> Result<Self> {
        // 初始化连接池和客户端
    }
    
    pub async fn analyze_text(&self, text: &str) -> Result<AnalysisResult> {
        let language = self.detect_language(text);
        let prompt = self.get_prompt_for_language(language);
        
        // 重试机制
        let mut attempts = 0;
        let max_attempts = 3;
        
        while attempts < max_attempts {
            match self.send_request(&prompt, text).await {
                Ok(result) => return Ok(result),
                Err(e) if attempts < max_attempts - 1 => {
                    log::warn!("Ollama request failed, retrying: {}", e);
                    attempts += 1;
                    tokio::time::sleep(Duration::from_secs(2_u64.pow(attempts))).await;
                }
                Err(e) => return Err(e),
            }
        }
        
        unreachable!()
    }
}
```

#### 2.3 Prompt管理系统

```rust
pub struct PromptManager {
    prompts: HashMap<Language, HashMap<PromptType, String>>,
}

#[derive(Hash, Eq, PartialEq)]
pub enum Language {
    English,
    Chinese,
}

#[derive(Hash, Eq, PartialEq)]
pub enum PromptType {
    Analysis,
    Summary,
    TaskExtraction,
}

impl PromptManager {
    pub fn get_prompt(&self, language: Language, prompt_type: PromptType) -> Option<&String> {
        self.prompts.get(&language)?.get(&prompt_type)
    }
    
    pub fn load_from_config() -> Result<Self> {
        // 从配置文件或数据库加载prompt
    }
}
```

### 3. 存储层抽象和数据库Schema设计

#### 3.1 数据库Schema设计

```sql
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
    tags TEXT[], -- PostgreSQL数组类型
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- 索引
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
```

#### 3.2 数据访问层(Repository Pattern)

```
src/
├── repository/
│   ├── mod.rs
│   ├── traits.rs        # Repository特征定义
│   ├── session.rs       # 会话仓库
│   ├── audio.rs         # 音频仓库
│   ├── transcript.rs    # 转录仓库
│   ├── analysis.rs      # 分析仓库
│   └── database.rs      # 数据库连接管理
```

```rust
// traits.rs
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: &NewSession) -> Result<Session>;
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Session>>;
    async fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>>;
    async fn update(&self, id: &Uuid, updates: &SessionUpdate) -> Result<Session>;
    async fn delete(&self, id: &Uuid) -> Result<()>;
}

#[async_trait]
pub trait AnalysisRepository: Send + Sync {
    async fn create(&self, analysis: &NewAnalysisResult) -> Result<AnalysisResult>;
    async fn find_by_session_id(&self, session_id: &Uuid) -> Result<Option<AnalysisResult>>;
    async fn update(&self, id: &Uuid, updates: &AnalysisUpdate) -> Result<AnalysisResult>;
    async fn delete(&self, id: &Uuid) -> Result<()>;
}

// 其他Repository特征...
```

#### 3.3 数据模型重构

```rust
// models/session.rs
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub status: SessionStatus,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSession {
    pub title: String,
    pub duration_ms: i64,
    pub metadata: Option<serde_json::Value>,
}

// models/analysis.rs
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnalysisResult {
    pub id: Uuid,
    pub session_id: Uuid,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub provider: String,
    pub model_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processing_time_ms: Option<i32>,
    
    // 关联数据（通过JOIN或单独查询获取）
    #[sqlx(skip)]
    pub ideas: Vec<Idea>,
    #[sqlx(skip)]
    pub tasks: Vec<Task>,
    #[sqlx(skip)]
    pub structured_notes: Vec<StructuredNote>,
}
```

#### 3.4 数据库迁移系统

```
migrations/
├── 001_initial_schema.sql
├── 002_add_indexes.sql
├── 003_add_metadata_columns.sql
└── ...
```

```rust
// src/database/migrations.rs
pub struct MigrationManager {
    pool: Arc<sqlx::PgPool>,
}

impl MigrationManager {
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&*self.pool).await?;
        Ok(())
    }
    
    pub async fn check_migration_status(&self) -> Result<Vec<MigrationInfo>> {
        // 检查迁移状态
    }
}
```

## 实施计划

### 阶段1：基础设施重构（1-2周）

1. **数据库设置**
   - 添加 `sqlx` 和 `tokio-postgres` 依赖
   - 创建数据库Schema和迁移文件
   - 实现数据库连接管理

2. **Repository层实现**
   - 实现基础的Repository特征
   - 创建PostgreSQL实现
   - 添加单元测试

### 阶段2：服务层重构（1-2周）

1. **Ollama服务重构**
   - 实现连接池和重试机制
   - 分离语言检测和Prompt管理
   - 添加性能监控

2. **分析服务重构**
   - 重构分析流程
   - 添加结果缓存
   - 实现异步处理队列

### 阶段3：API层重构（1周）

1. **统一API接口**
   - 重构现有API端点
   - 实现统一错误处理
   - 添加API文档

2. **中间件实现**
   - 添加日志中间件
   - 实现请求限流
   - 添加健康检查端点

### 阶段4：数据迁移和测试（1周）

1. **数据迁移**
   - 实现从文件系统到数据库的迁移工具
   - 验证数据完整性

2. **集成测试**
   - 端到端测试
   - 性能测试
   - 压力测试

## 技术栈更新

### 新增依赖

```toml
[dependencies]
# 数据库
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }
tokio-postgres = "0.7"

# 错误处理
thiserror = "1.0"

# 异步特征
async-trait = "0.1"

# 连接池
deadpool-postgres = "0.12"

# 配置管理
config = "0.14"

# 监控和指标
prometheus = "0.13"
tracing = "0.1"
tracing-subscriber = "0.3"

# API文档
utoipa = "4.0"
utoipa-swagger-ui = "4.0"
```

## 配置文件更新

```toml
# config.toml
[database]
url = "postgresql://username:password@localhost/voice_recorder"
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
```

## 总结

这个重构计划将显著提升项目的可维护性、可扩展性和性能：

1. **接口层**：统一的API设计和错误处理机制
2. **服务层**：解耦的业务逻辑和改进的Ollama集成
3. **数据层**：强类型的数据库Schema和Repository模式

重构后的架构将支持：
- 更好的错误处理和日志记录
- 数据库事务和一致性保证
- 水平扩展和负载均衡
- 完整的API文档和测试覆盖
- 监控和性能指标收集