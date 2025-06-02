#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
测试语言检测和多语言 prompt 功能
使用实际的会话转录文件
"""

import requests
import json
import os

# 配置
OLLAMA_ENDPOINT = "http://localhost:11434/api/chat"
MODEL_NAME = "deepseek-r1:8b-0528-qwen3-fp16"
BACKEND_URL = "http://localhost:3000"
AUDIO_DIR = "local_storage/app_data/audio"
SESSIONS_DIR = "local_storage/app_data/sessions"

def read_transcript_file(session_id):
    """读取转录文件"""
    transcript_path = os.path.join(AUDIO_DIR, f"{session_id}.wav.txt")
    if os.path.exists(transcript_path):
        with open(transcript_path, 'r', encoding='utf-8') as f:
            return f.read().strip()
    return None

def detect_language_python(text):
    """Python 版本的语言检测（模拟 Rust 逻辑）"""
    chinese_chars = 0
    total_chars = 0
    
    for char in text:
        if not char.isspace():
            total_chars += 1
            code = ord(char)
            # 中文字符范围检测
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
        return "en"
    
    # 如果中文字符占比超过30%，认为是中文
    if chinese_chars / total_chars > 0.3:
        return "zh"
    else:
        return "en"

def get_prompt_by_language(language):
    """根据语言获取对应的 prompt"""
    if language == "zh":
        return """你是一个专业的文本分析助手，专门处理各种类型的文本内容并生成结构化分析。请客观地分析提供的文本内容，并提取以下信息到一个格式良好的JSON对象中：

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
        return """You are an AI assistant specialized in analyzing meeting transcripts and generating structured insights. Your goal is to process the provided transcript and extract the following information in a well-formatted JSON object:

1.  **Title**: A concise, descriptive title for the entire note, summarizing its main topic.
2.  **Summary**: A concise overview of the main points and outcomes discussed.
3.  **Ideas**: A list of potential ideas or suggestions that arose from the discussion.
4.  **Tasks**: A list of actionable tasks identified, including a title, optional description, and priority (Low, Medium, High, Urgent).
5.  **Structured Notes**: A list of key discussion points or decisions, formatted as structured notes with a title, content, relevant tags (as a list of strings), and a note type (Meeting, Brainstorm, Decision, Action, Reference).

Ensure the JSON output is valid and strictly follows the specified structure. Do not include any other text outside the JSON object.

If the provided transcript is empty or contains only whitespace, return an empty JSON object `{{}}`."""

def test_with_existing_transcript():
    """使用现有的转录文件测试"""
    session_id = "61a0f530-6db8-496b-b251-78c90966f071"
    
    print(f"=== 测试现有转录文件: {session_id} ===")
    
    # 读取转录文件
    transcript = read_transcript_file(session_id)
    if not transcript:
        print(f"❌ 无法读取转录文件: {session_id}")
        return False
    
    print(f"转录内容: {transcript[:100]}...")
    
    # 检测语言
    detected_language = detect_language_python(transcript)
    print(f"检测到的语言: {detected_language}")
    
    # 获取对应的 prompt
    base_prompt = get_prompt_by_language(detected_language)
    full_prompt = f"{base_prompt}\n\nTranscript: {transcript}\n\nJSON Output:"
    
    # 构造 Ollama 请求
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
        print("🚀 发送 Ollama 请求...")
        response = requests.post(OLLAMA_ENDPOINT, json=request_body, timeout=120)
        
        if response.status_code == 200:
            result = response.json()
            print("✅ Ollama 分析成功")
            
            # 提取分析结果
            if "message" in result and "content" in result["message"]:
                analysis_content = result["message"]["content"]
                print(f"分析结果: {analysis_content[:200]}...")
                
                # 尝试解析 JSON
                try:
                    analysis_json = json.loads(analysis_content)
                    print("✅ JSON 解析成功")
                    print(f"标题: {analysis_json.get('title', 'N/A')}")
                    print(f"摘要: {analysis_json.get('summary', 'N/A')[:100]}...")
                    return True
                except json.JSONDecodeError as e:
                    print(f"❌ JSON 解析失败: {e}")
                    return False
            else:
                print("❌ 响应格式不正确")
                return False
        else:
            print(f"❌ Ollama 请求失败: {response.status_code}")
            print(f"错误信息: {response.text}")
            return False
            
    except Exception as e:
        print(f"❌ 请求异常: {e}")
        return False

def test_language_detection():
    """测试语言检测功能"""
    print("=== 测试语言检测功能 ===")
    
    test_cases = [
        ("Hello world, this is a test.", "en"),
        ("你好世界，这是一个测试。", "zh"),
        ("Today's meeting 今天的会议 discussed important topics.", "zh"),  # 混合文本，中文占比高
        ("Meeting with 一些中文 but mostly English content here.", "en"),  # 混合文本，英文占比高
        ("", "en"),  # 空文本
        ("   \n\t  ", "en"),  # 只有空白字符
    ]
    
    all_passed = True
    for text, expected in test_cases:
        detected = detect_language_python(text)
        status = "✅" if detected == expected else "❌"
        print(f"{status} 文本: '{text[:30]}...' -> 检测: {detected}, 期望: {expected}")
        if detected != expected:
            all_passed = False
    
    return all_passed

def main():
    """主函数"""
    print("🚀 开始测试语言检测和多语言 prompt 功能")
    print(f"使用模型: {MODEL_NAME}")
    print(f"Ollama 端点: {OLLAMA_ENDPOINT}")
    
    results = []
    
    # 测试语言检测
    print("\n" + "="*50)
    results.append(test_language_detection())
    
    # 测试现有转录文件
    print("\n" + "="*50)
    results.append(test_with_existing_transcript())
    
    # 总结结果
    print("\n" + "="*50)
    print("=== 测试结果总结 ===")
    success_count = sum(results)
    total_count = len(results)
    print(f"成功: {success_count}/{total_count}")
    
    if success_count == total_count:
        print("🎉 所有测试通过！语言检测和多语言 prompt 功能正常工作。")
    else:
        print("⚠️  部分测试失败，请检查配置和服务状态。")

if __name__ == "__main__":
    main()