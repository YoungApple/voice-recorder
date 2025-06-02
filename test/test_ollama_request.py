#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Test script to construct and send Ollama request using transcript from session 61a0f530-6db8-496b-b251-78c90966f071
"""

import requests
import json

# Transcript content from the audio file
transcript = """火力全開 一口氣推出四篇系列文章
第一篇標題赤裸裸的寫 帶頭堅持集體領導
這是什麼意思
這是告訴全黨軍隊要回到矛盾那一套
不再維襲命是從
但是外界就有疑問
這是不是軍方要造反
事實證明不是要 而是早就懂
你仔細對比那段時間以後的黨的變化
就能發現偉大領袖 最高土帥 定於一尊
這些詞突然在人民日報 新華社 央視集體消失
習近平的照片不再刷屏
學習強國的打卡內容都冷淡下來
這不是降溫 是風存
總結就是一句話 劉援被囚 是導火索
張又霞救人 是引爆器
軍隊報射 打頭陣 標語 變風向 警衛局換班
這是一場仇本"""

# Ollama endpoint configuration
ollama_endpoint = "http://localhost:11434/api/chat"
model_name = "deepseek-r1:8b-0528-qwen3-fp16"  # Available model from ollama list

# Construct the prompt as used in the Rust code
prompt = f"""You are an AI assistant specialized in analyzing meeting transcripts and generating structured insights. Your goal is to process the provided transcript and extract the following information in a well-formatted JSON object:

1.  **Title**: A concise, descriptive title for the entire note, summarizing its main topic.
2.  **Summary**: A concise overview of the main points and outcomes discussed.
3.  **Ideas**: A list of potential ideas or suggestions that arose from the discussion.
4.  **Tasks**: A list of actionable tasks identified, including a title, optional description, and priority (Low, Medium, High, Urgent).
5.  **Structured Notes**: A list of key discussion points or decisions, formatted as structured notes with a title, content, relevant tags (as a list of strings), and a note type (Meeting, Brainstorm, Decision, Action, Reference).

Ensure the JSON output is valid and strictly follows the specified structure. Do not include any other text outside the JSON object.

If the provided transcript is empty or contains only whitespace, return an empty JSON object `{{}}`.

Transcript: {transcript}

JSON Output:"""

# Construct the request body as used in the Rust code
request_body = {
    "model": model_name,
    "messages": [
        {
            "role": "user",
            "content": prompt
        }
    ],
    "format": "json",  # Request JSON output from Ollama
    "stream": False    # Ensure non-streaming response for easier parsing
}

def test_ollama_request():
    """Test the Ollama request with the transcript content"""
    print("[Test] Testing Ollama request with session 61a0f530-6db8-496b-b251-78c90966f071 transcript")
    print(f"[Test] Endpoint: {ollama_endpoint}")
    print(f"[Test] Model: {model_name}")
    print(f"[Test] Transcript length: {len(transcript)} characters")
    print("\n" + "="*80)
    
    # Print the request body for debugging
    print("[Test] Request body:")
    print(json.dumps(request_body, indent=2, ensure_ascii=False))
    print("\n" + "="*80)
    
    try:
        # Send the request
        print("[Test] Sending request to Ollama...")
        response = requests.post(ollama_endpoint, json=request_body, timeout=60)
        
        print(f"[Test] Response status: {response.status_code}")
        
        if response.status_code == 200:
            response_data = response.json()
            print("[Test] Raw response:")
            print(json.dumps(response_data, indent=2, ensure_ascii=False))
            
            # Extract the content as done in Rust code
            content = None
            if 'message' in response_data and 'content' in response_data['message']:
                content = response_data['message']['content']
            elif 'response' in response_data:
                content = response_data['response']
            elif 'content' in response_data:
                content = response_data['content']
            
            if content:
                print("\n" + "="*80)
                print("[Test] Extracted content:")
                print(content)
                
                # Try to parse as JSON
                try:
                    parsed_content = json.loads(content)
                    print("\n" + "="*80)
                    print("[Test] Parsed JSON content:")
                    print(json.dumps(parsed_content, indent=2, ensure_ascii=False))
                except json.JSONDecodeError as e:
                    print(f"[Test] Failed to parse content as JSON: {e}")
            else:
                print("[Test] Could not extract content from response")
                
        else:
            print(f"[Test] Request failed with status {response.status_code}")
            print(f"[Test] Error response: {response.text}")
            
    except requests.exceptions.ConnectionError:
        print("[Test] Failed to connect to Ollama. Make sure Ollama is running on localhost:11434")
        print("[Test] You can start Ollama with: ollama serve")
    except requests.exceptions.Timeout:
        print("[Test] Request timed out. The model might be taking too long to respond.")
    except Exception as e:
        print(f"[Test] Unexpected error: {e}")

if __name__ == "__main__":
    test_ollama_request()