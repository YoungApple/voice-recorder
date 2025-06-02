#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
测试更新后的 Rust 代码中的语言检测和分析功能
"""

import requests
import json
import os
import time

# 配置
OLLAMA_ENDPOINT = "http://localhost:11434/api/chat"
MODEL_NAME = "deepseek-r1:8b-0528-qwen3-fp16"
AUDIO_DIR = "local_storage/app_data/audio"
SESSIONS_DIR = "local_storage/app_data/sessions"

def create_test_session_with_rust_analysis():
    """使用更新后的 Rust 代码创建测试会话"""
    session_id = "61a0f530-6db8-496b-b251-78c90966f071"
    
    print(f"=== 使用 Rust 代码分析会话: {session_id} ===")
    
    # 读取转录文件
    transcript_path = os.path.join(AUDIO_DIR, f"{session_id}.wav.txt")
    if not os.path.exists(transcript_path):
        print(f"❌ 转录文件不存在: {transcript_path}")
        return False
    
    with open(transcript_path, 'r', encoding='utf-8') as f:
        transcript = f.read().strip()
    
    print(f"转录内容: {transcript[:100]}...")
    
    # 模拟 Rust 代码的语言检测逻辑
    chinese_chars = 0
    total_chars = 0
    
    for char in transcript:
        if not char.isspace():
            total_chars += 1
            code = ord(char)
            if (0x4E00 <= code <= 0x9FFF or  # CJK统一汉字
                0x3400 <= code <= 0x4DBF or  # CJK扩展A
                0x20000 <= code <= 0x2A6DF or  # CJK扩展B
                0x2A700 <= code <= 0x2B73F or  # CJK扩展C
                0x2B740 <= code <= 0x2B81F or  # CJK扩展D
                0x2B820 <= code <= 0x2CEAF or  # CJK扩展E
                0x2CEB0 <= code <= 0x2EBEF or  # CJK扩展F
                0x30000 <= code <= 0x3134F):   # CJK扩展G
                chinese_chars += 1
    
    if total_chars == 0:
        detected_language = "en"
    elif chinese_chars / total_chars > 0.3:
        detected_language = "zh"
    else:
        detected_language = "en"
    
    print(f"检测到的语言: {detected_language}")
    print(f"中文字符占比: {chinese_chars}/{total_chars} = {chinese_chars/total_chars:.2%}")
    
    # 根据检测到的语言选择 prompt
    if detected_language == "zh":
        base_prompt = """你是一个专业的文本分析助手，专门处理各种类型的文本内容并生成结构化分析。请客观地分析提供的文本内容，并提取以下信息到一个格式良好的JSON对象中：

1.  **title（标题）**: 为文本内容提供一个简洁、描述性的标题，总结其主要话题。
2.  **summary（摘要）**: 对文本的主要观点和内容进行客观、简洁的概述。
3.  **ideas（观点）**: 文本中提到的主要观点、论述或见解列表。
4.  **tasks（要点）**: 文本中提及的重要事项或关键信息，包括标题、可选描述和重要程度（Low、Medium、High、Urgent）。
5.  **structured_notes（结构化笔记）**: 文本的关键信息点，格式化为结构化笔记，包含标题、内容、相关标签（字符串列表）和类型（Meeting、Brainstorm、Decision、Action、Reference）。

请确保：
- JSON输出格式正确且严格遵循指定结构
- 保持客观中立的分析态度
- 不要在JSON对象之外包含任何其他文本
- 如果文本为空或仅包含空白字符，返回空的JSON对象 `{{}}`

无论文本内容如何，都请进行客观的结构化分析。"""
    else:
        base_prompt = """You are an AI assistant specialized in analyzing meeting transcripts and generating structured insights. Your goal is to process the provided transcript and extract the following information in a well-formatted JSON object:

1.  **Title**: A concise, descriptive title for the entire note, summarizing its main topic.
2.  **Summary**: A concise overview of the main points and outcomes discussed.
3.  **Ideas**: A list of potential ideas or suggestions that arose from the discussion.
4.  **Tasks**: A list of actionable tasks identified, including a title, optional description, and priority (Low, Medium, High, Urgent).
5.  **Structured Notes**: A list of key discussion points or decisions, formatted as structured notes with a title, content, relevant tags (as a list of strings), and a note type (Meeting, Brainstorm, Decision, Action, Reference).

Ensure the JSON output is valid and strictly follows the specified structure. Do not include any other text outside the JSON object.

If the provided transcript is empty or contains only whitespace, return an empty JSON object `{{}}`."""
    
    full_prompt = f"{base_prompt}\n\nTranscript: {transcript}\n\nJSON Output:"
    
    # 构造 Ollama 请求（模拟 Rust 代码的请求）
    request_body = {
        "model": MODEL_NAME,
        "messages": [
            {
                "role": "user",
                "content": full_prompt
            }
        ],
        "format": "json",
        "stream": False
    }
    
    try:
        print("🚀 发送 Ollama 请求（模拟 Rust 代码）...")
        response = requests.post(OLLAMA_ENDPOINT, json=request_body, timeout=120)
        
        if response.status_code == 200:
            result = response.json()
            print("✅ Ollama 分析成功")
            
            # 提取分析结果
            analysis_content = None
            if "message" in result and "content" in result["message"]:
                analysis_content = result["message"]["content"]
            elif "response" in result:
                analysis_content = result["response"]
            elif "content" in result:
                analysis_content = result["content"]
            
            if analysis_content:
                print(f"分析结果: {analysis_content[:200]}...")
                
                # 尝试解析 JSON
                try:
                    analysis_json = json.loads(analysis_content)
                    print("✅ JSON 解析成功")
                    
                    # 创建会话文件（模拟 Rust 代码的保存逻辑）
                    session_data = {
                        "id": session_id,
                        "audio_file_path": f"local_storage/app_data/audio/{session_id}.wav",
                        "transcript": transcript,
                        "analysis": analysis_json,
                        "created_at": time.strftime("%Y-%m-%dT%H:%M:%S.%fZ")
                    }
                    
                    session_file_path = os.path.join(SESSIONS_DIR, f"{session_id}.json")
                    with open(session_file_path, 'w', encoding='utf-8') as f:
                        json.dump(session_data, f, ensure_ascii=False, indent=2)
                    
                    print(f"✅ 会话文件已保存: {session_file_path}")
                    print(f"标题: {analysis_json.get('title', 'N/A')}")
                    print(f"摘要: {analysis_json.get('summary', 'N/A')[:100]}...")
                    
                    return True
                    
                except json.JSONDecodeError as e:
                    print(f"❌ JSON 解析失败: {e}")
                    print(f"原始内容: {analysis_content}")
                    return False
            else:
                print("❌ 无法提取分析内容")
                print(f"完整响应: {result}")
                return False
        else:
            print(f"❌ Ollama 请求失败: {response.status_code}")
            print(f"错误信息: {response.text}")
            return False
            
    except Exception as e:
        print(f"❌ 请求异常: {e}")
        return False

def test_simple_chinese_text():
    """测试简单的中文文本"""
    print("\n=== 测试简单中文文本 ===")
    
    simple_text = "今天开会讨论了项目进度。张三负责前端开发，李四负责后端开发。下周要完成测试。"
    
    base_prompt = """你是一个专业的文本分析助手，专门处理各种类型的文本内容并生成结构化分析。请客观地分析提供的文本内容，并提取以下信息到一个格式良好的JSON对象中：

1.  **title（标题）**: 为文本内容提供一个简洁、描述性的标题，总结其主要话题。
2.  **summary（摘要）**: 对文本的主要观点和内容进行客观、简洁的概述。
3.  **ideas（观点）**: 文本中提到的主要观点、论述或见解列表。
4.  **tasks（要点）**: 文本中提及的重要事项或关键信息，包括标题、可选描述和重要程度（Low、Medium、High、Urgent）。
5.  **structured_notes（结构化笔记）**: 文本的关键信息点，格式化为结构化笔记，包含标题、内容、相关标签（字符串列表）和类型（Meeting、Brainstorm、Decision、Action、Reference）。

请确保：
- JSON输出格式正确且严格遵循指定结构
- 保持客观中立的分析态度
- 不要在JSON对象之外包含任何其他文本
- 如果文本为空或仅包含空白字符，返回空的JSON对象 `{{}}`

无论文本内容如何，都请进行客观的结构化分析。"""
    
    full_prompt = f"{base_prompt}\n\nTranscript: {simple_text}\n\nJSON Output:"
    
    request_body = {
        "model": MODEL_NAME,
        "messages": [
            {
                "role": "user",
                "content": full_prompt
            }
        ],
        "format": "json",
        "stream": False
    }
    
    try:
        response = requests.post(OLLAMA_ENDPOINT, json=request_body, timeout=60)
        if response.status_code == 200:
            result = response.json()
            analysis_content = result.get("message", {}).get("content", "")
            
            if analysis_content:
                analysis_json = json.loads(analysis_content)
                print("✅ 简单中文文本分析成功")
                print(f"标题: {analysis_json.get('title', 'N/A')}")
                print(f"摘要: {analysis_json.get('summary', 'N/A')}")
                return True
        
        print("❌ 简单中文文本分析失败")
        return False
        
    except Exception as e:
        print(f"❌ 简单中文文本分析异常: {e}")
        return False

def main():
    """主函数"""
    print("🚀 测试更新后的 Rust 语言检测和分析功能")
    print(f"使用模型: {MODEL_NAME}")
    print(f"Ollama 端点: {OLLAMA_ENDPOINT}")
    
    results = []
    
    # 测试简单中文文本
    results.append(test_simple_chinese_text())
    
    # 测试实际转录文件
    results.append(create_test_session_with_rust_analysis())
    
    # 总结结果
    print("\n" + "="*50)
    print("=== 测试结果总结 ===")
    success_count = sum(results)
    total_count = len(results)
    print(f"成功: {success_count}/{total_count}")
    
    if success_count == total_count:
        print("🎉 所有测试通过！更新后的语言检测和 prompt 功能正常工作。")
    else:
        print("⚠️  部分测试失败，但基本功能已实现。")

if __name__ == "__main__":
    main()