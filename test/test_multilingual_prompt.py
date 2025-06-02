#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
测试多语言 Ollama prompt 功能
"""

import requests
import json

# Ollama 配置
OLLAMA_ENDPOINT = "http://localhost:11434/api/chat"
MODEL_NAME = "deepseek-r1:8b-0528-qwen3-fp16"

def test_chinese_transcript():
    """测试中文转录文本"""
    chinese_transcript = """
    今天的会议主要讨论了三个议题：
    1. 产品开发进度 - 目前已完成70%，预计下月底完成
    2. 市场推广策略 - 需要加强社交媒体营销
    3. 团队建设 - 计划招聘2名新员工
    
    决定事项：
    - 张三负责完成产品测试
    - 李四制定详细的营销计划
    - 王五负责招聘工作
    """
    
    print("=== 测试中文转录 ===")
    print(f"转录内容: {chinese_transcript[:50]}...")
    
    # 构造请求
    request_body = {
        "model": MODEL_NAME,
        "messages": [
            {
                "role": "user",
                "content": f"你是一个专门分析会议记录和生成结构化洞察的AI助手。你的目标是处理提供的转录文本，并提取以下信息到一个格式良好的JSON对象中：\n\n1.  **Title（标题）**: 为整个笔记提供一个简洁、描述性的标题，总结其主要话题。\n2.  **Summary（摘要）**: 对讨论的主要观点和结果进行简洁概述。\n3.  **Ideas（想法）**: 讨论中产生的潜在想法或建议列表。\n4.  **Tasks（任务）**: 识别出的可执行任务列表，包括标题、可选描述和优先级（Low、Medium、High、Urgent）。\n5.  **Structured Notes（结构化笔记）**: 关键讨论要点或决策列表，格式化为结构化笔记，包含标题、内容、相关标签（字符串列表）和笔记类型（Meeting、Brainstorm、Decision、Action、Reference）。\n\n确保JSON输出有效且严格遵循指定的结构。不要在JSON对象之外包含任何其他文本。\n\n如果提供的转录文本为空或仅包含空白字符，返回一个空的JSON对象 `{{}}`。\n\nTranscript: {chinese_transcript}\n\nJSON Output:"
            }
        ],
        "format": "json",
        "stream": False
    }
    
    try:
        response = requests.post(OLLAMA_ENDPOINT, json=request_body, timeout=60)
        if response.status_code == 200:
            result = response.json()
            print("✅ 中文请求成功")
            print(f"响应: {json.dumps(result, ensure_ascii=False, indent=2)}")
            return True
        else:
            print(f"❌ 中文请求失败: {response.status_code}")
            print(f"错误信息: {response.text}")
            return False
    except Exception as e:
        print(f"❌ 中文请求异常: {e}")
        return False

def test_english_transcript():
    """测试英文转录文本"""
    english_transcript = """
    Today's meeting covered three main topics:
    1. Product development progress - currently 70% complete, expected to finish by end of next month
    2. Marketing strategy - need to strengthen social media marketing
    3. Team building - plan to hire 2 new employees
    
    Decisions made:
    - John will handle product testing
    - Sarah will create detailed marketing plan
    - Mike will handle recruitment
    """
    
    print("\n=== 测试英文转录 ===")
    print(f"转录内容: {english_transcript[:50]}...")
    
    # 构造请求
    request_body = {
        "model": MODEL_NAME,
        "messages": [
            {
                "role": "user",
                "content": f"You are an AI assistant specialized in analyzing meeting transcripts and generating structured insights. Your goal is to process the provided transcript and extract the following information in a well-formatted JSON object:\n\n1.  **Title**: A concise, descriptive title for the entire note, summarizing its main topic.\n2.  **Summary**: A concise overview of the main points and outcomes discussed.\n3.  **Ideas**: A list of potential ideas or suggestions that arose from the discussion.\n4.  **Tasks**: A list of actionable tasks identified, including a title, optional description, and priority (Low, Medium, High, Urgent).\n5.  **Structured Notes**: A list of key discussion points or decisions, formatted as structured notes with a title, content, relevant tags (as a list of strings), and a note type (Meeting, Brainstorm, Decision, Action, Reference).\n\nEnsure the JSON output is valid and strictly follows the specified structure. Do not include any other text outside the JSON object.\n\nIf the provided transcript is empty or contains only whitespace, return an empty JSON object `{{}}`\n\nTranscript: {english_transcript}\n\nJSON Output:"
            }
        ],
        "format": "json",
        "stream": False
    }
    
    try:
        response = requests.post(OLLAMA_ENDPOINT, json=request_body, timeout=60)
        if response.status_code == 200:
            result = response.json()
            print("✅ 英文请求成功")
            print(f"响应: {json.dumps(result, ensure_ascii=False, indent=2)}")
            return True
        else:
            print(f"❌ 英文请求失败: {response.status_code}")
            print(f"错误信息: {response.text}")
            return False
    except Exception as e:
        print(f"❌ 英文请求异常: {e}")
        return False

def test_mixed_transcript():
    """测试中英文混合转录文本"""
    mixed_transcript = """
    Today's meeting 今天的会议主要讨论了 product roadmap:
    1. Q1 goals - 完成用户界面设计
    2. Technical architecture - 使用 microservices 架构
    3. Team allocation - 分配开发团队资源
    
    Action items:
    - Design team 设计团队 will create mockups
    - Backend team 后端团队 will setup infrastructure
    - QA team 测试团队 will prepare test cases
    """
    
    print("\n=== 测试中英文混合转录 ===")
    print(f"转录内容: {mixed_transcript[:50]}...")
    
    # 由于是混合文本，这里应该会检测为中文（因为中文字符占比较高）
    request_body = {
        "model": MODEL_NAME,
        "messages": [
            {
                "role": "user",
                "content": f"你是一个专门分析会议记录和生成结构化洞察的AI助手。你的目标是处理提供的转录文本，并提取以下信息到一个格式良好的JSON对象中：\n\n1.  **Title（标题）**: 为整个笔记提供一个简洁、描述性的标题，总结其主要话题。\n2.  **Summary（摘要）**: 对讨论的主要观点和结果进行简洁概述。\n3.  **Ideas（想法）**: 讨论中产生的潜在想法或建议列表。\n4.  **Tasks（任务）**: 识别出的可执行任务列表，包括标题、可选描述和优先级（Low、Medium、High、Urgent）。\n5.  **Structured Notes（结构化笔记）**: 关键讨论要点或决策列表，格式化为结构化笔记，包含标题、内容、相关标签（字符串列表）和笔记类型（Meeting、Brainstorm、Decision、Action、Reference）。\n\n确保JSON输出有效且严格遵循指定的结构。不要在JSON对象之外包含任何其他文本。\n\n如果提供的转录文本为空或仅包含空白字符，返回一个空的JSON对象 `{{}}`。\n\nTranscript: {mixed_transcript}\n\nJSON Output:"
            }
        ],
        "format": "json",
        "stream": False
    }
    
    try:
        response = requests.post(OLLAMA_ENDPOINT, json=request_body, timeout=60)
        if response.status_code == 200:
            result = response.json()
            print("✅ 混合文本请求成功")
            print(f"响应: {json.dumps(result, ensure_ascii=False, indent=2)}")
            return True
        else:
            print(f"❌ 混合文本请求失败: {response.status_code}")
            print(f"错误信息: {response.text}")
            return False
    except Exception as e:
        print(f"❌ 混合文本请求异常: {e}")
        return False

def main():
    """主函数"""
    print("🚀 开始测试多语言 Ollama prompt 功能")
    print(f"使用模型: {MODEL_NAME}")
    print(f"端点: {OLLAMA_ENDPOINT}")
    
    results = []
    
    # 测试中文
    results.append(test_chinese_transcript())
    
    # 测试英文
    results.append(test_english_transcript())
    
    # 测试混合文本
    results.append(test_mixed_transcript())
    
    # 总结结果
    print("\n=== 测试结果总结 ===")
    success_count = sum(results)
    total_count = len(results)
    print(f"成功: {success_count}/{total_count}")
    
    if success_count == total_count:
        print("🎉 所有测试通过！多语言 prompt 功能正常工作。")
    else:
        print("⚠️  部分测试失败，请检查 Ollama 服务和模型配置。")

if __name__ == "__main__":
    main()