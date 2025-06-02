# Voice Recorder CLI Tool - 安装和使用指南

这是一个基于Rust开发的语音录制和AI分析工具，专为macOS设计。它能够监听键盘事件、录制语音、调用AI模型进行转录和分析，并提供精美的Web管理界面。

## 🚀 功能特性

- **🎤 实时语音录制**: 使用键盘快捷键控制录制
- **🤖 AI转录**: 支持OpenAI Whisper和本地模型
- **📊 智能分析**: 自动提取想法、任务和结构化笔记
- **💾 本地存储**: 精巧的数据存储格式
- **🌐 Web界面**: 现代化的管理界面
- **⚙️ 灵活配置**: 支持多种AI提供商

## 📋 系统要求

- macOS 10.15+
- Rust 1.70+
- 麦克风权限
- 可访读权限（用于键盘监听）

## 🛠️ 安装步骤

### 1. 安装Rust开发环境

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. 克隆或创建项目

创建项目目录并添加源代码：

```bash
mkdir voice-recorder
cd voice-recorder
```

将提供的代码保存到相应文件中：
- `Cargo.toml` - 项目配置
- `src/main.rs` - 主程序
- `src/config.rs` - 配置管理
- `src/storage.rs` - 数据存储
- `src/keyboard.rs` - 键盘监听
- `src/audio.rs` - 音频录制
- `src/ai.rs` - AI处理
- `src/web.rs` - Web服务
- `web/index.html` - Web界面

### 3. 创建Web目录

```bash
mkdir web
# 将web界面HTML保存为 web/index.html
```

### 4. 编译项目

```bash
cargo build --release
```

### 5. 设置系统权限

在macOS上，需要授予以下权限：

1. **麦克风权限**:
   - 系统偏好设置 → 安全性与隐私 → 隐私 → 麦克风
   - 添加终端应用或你的程序

2. **辅助功能权限**（用于键盘监听）:
   - 系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能
   - 添加终端应用

## 🎯 使用方法

### 基本命令

```bash
# 安装到系统路径（可选）
cargo install --path .

# 或者直接运行
./target/release/voice-recorder
```

### 1. 配置AI服务

#### 使用OpenAI:
```bash
voice-recorder config set-openai "your-openai-api-key"
```

#### 使用本地模型:
```bash
voice-recorder config set-local "http://localhost:8000"
```

#### 查看当前配置:
```bash
voice-recorder config show
```

### 2. 开始录制会话

```bash
voice-recorder record
```

录制控制：
- **按 'r' 键**: 开始录制 🎤
- **按 'e' 键**: 结束录制 ⏹️
- **按 'q' 键**: 退出程序 👋

### 3. 查看录制历史

```bash
# 列出所有会话
voice-recorder list

# 查看特定会话详情
voice-recorder show <session-id>
```

### 4. 启动Web管理界面

```bash
voice-recorder web --port 3000
```

然后在浏览器中访问: `http://localhost:3000`

## 🌐 Web界面功能

### 📊 Dashboard
- 统计概览（会话数、想法数、任务数、笔记数）
- 最近录制会话预览
- 快速访问功能

### 📝 Sessions
- 完整的录制会话列表
- 详细的转录文本
- AI分析结果展示：
  - 💡 **想法提取**: 关键洞察和想法
  - ✅ **任务识别**: 待办事项和优先级
  - 📝 **结构化笔记**: 会议、决策、行动记录

### ⚙️ Configuration
- AI提供商设置
- API密钥管理
- 使用说明

## 📁 数据存储格式

程序使用精巧的JSON格式存储数据：

```
~/.voice-recorder/
├── config.json          # 配置文件
├── audio/               # 音频文件
│   ├── uuid1.wav
│   └── uuid2.wav
└── sessions/            # 会话数据
    ├── uuid1.json
    └── uuid2.json
```

### 会话数据结构:

```json
{
  "id": "uuid",
  "timestamp": "2023-12-07T10:30:00Z",
  "audio_file_path": "/path/to/audio.wav",
  "transcript": "转录文本...",
  "title": "会话标题",
  "duration_ms": 45000,
  "analysis": {
    "ideas": ["想法1", "想法2"],
    "tasks": [
      {
        "title": "任务标题",
        "description": "任务描述",
        "priority": "High",
        "due_date": null
      }
    ],
    "structured_notes": [
      {
        "title": "笔记标题",
        "content": "笔记内容",
        "tags": ["标签1", "标签2"],
        "note_type": "Meeting"
      }
    ],
    "summary": "会话总结"
  }
}
```

## 🔧 高级配置

### 音频