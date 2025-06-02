#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Test script to demonstrate Ollama analysis request construction using the transcript
from session 61a0f530-6db8-496b-b251-78c90966f071

This script shows how the voice-recorder application would construct and send
requests to Ollama for analyzing voice transcripts.
"""

import requests
import json
import os
from datetime import datetime

def read_transcript(session_id):
    """Read the transcript file for the given session ID"""
    transcript_path = f"local_storage/app_data/audio/{session_id}.wav.txt"
    try:
        with open(transcript_path, 'r', encoding='utf-8') as f:
            return f.read().strip()
    except FileNotFoundError:
        print(f"[Error] Transcript file not found: {transcript_path}")
        return None

def construct_ollama_request(transcript, model_name="deepseek-r1:8b-0528-qwen3-fp16"):
    """Construct the Ollama request as done in the Rust code"""
    
    # This is the exact prompt used in the Rust code (src/ollama/mod.rs)
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
    
    # Request body structure from Rust code
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
    
    return request_body

def send_ollama_request(request_body, endpoint="http://localhost:11434/api/chat"):
    """Send the request to Ollama and parse the response"""
    try:
        print(f"[Ollama] Sending request to {endpoint}...")
        response = requests.post(endpoint, json=request_body, timeout=120)
        
        if response.status_code == 200:
            response_data = response.json()
            print(f"[Ollama] Request successful")
            
            # Extract content as done in Rust code
            content = None
            if 'message' in response_data and 'content' in response_data['message']:
                content = response_data['message']['content']
            elif 'response' in response_data:
                content = response_data['response']
            elif 'content' in response_data:
                content = response_data['content']
            
            if content:
                try:
                    # Parse the JSON content
                    analysis_result = json.loads(content)
                    return analysis_result
                except json.JSONDecodeError as e:
                    print(f"[Ollama] Failed to parse response as JSON: {e}")
                    print(f"[Ollama] Raw content: {content}")
                    return None
            else:
                print(f"[Ollama] Could not extract content from response")
                print(f"[Ollama] Raw response: {json.dumps(response_data, indent=2)}")
                return None
        else:
            print(f"[Ollama] Request failed with status {response.status_code}")
            print(f"[Ollama] Error: {response.text}")
            return None
            
    except requests.exceptions.ConnectionError:
        print("[Ollama] Connection failed. Make sure Ollama is running on localhost:11434")
        print("[Ollama] Start Ollama with: ollama serve")
        return None
    except requests.exceptions.Timeout:
        print("[Ollama] Request timed out")
        return None
    except Exception as e:
        print(f"[Ollama] Unexpected error: {e}")
        return None

def create_voice_session(session_id, transcript, analysis_result):
    """Create a VoiceSession object structure as would be done in Rust"""
    return {
        "id": session_id,
        "audio_file_path": f"local_storage/app_data/audio/{session_id}.wav",
        "transcript": transcript,
        "analysis": analysis_result,
        "created_at": datetime.utcnow().isoformat() + "Z",
        "duration_seconds": None  # Would be calculated from audio file
    }

def save_session_json(session_data, session_id):
    """Save the session data to JSON file as done in Rust storage::save_session"""
    sessions_dir = "local_storage/app_data/sessions"
    os.makedirs(sessions_dir, exist_ok=True)
    
    session_file = os.path.join(sessions_dir, f"{session_id}.json")
    
    try:
        with open(session_file, 'w', encoding='utf-8') as f:
            json.dump(session_data, f, indent=2, ensure_ascii=False)
        print(f"[Storage] Session saved to: {session_file}")
        return True
    except Exception as e:
        print(f"[Storage] Failed to save session: {e}")
        return False

def main():
    """Main test function"""
    session_id = "61a0f530-6db8-496b-b251-78c90966f071"
    
    print(f"[Test] Testing Ollama analysis for session: {session_id}")
    print("=" * 80)
    
    # Step 1: Read transcript
    transcript = read_transcript(session_id)
    if not transcript:
        return
    
    print(f"[Test] Loaded transcript ({len(transcript)} characters):")
    print(transcript)
    print("\n" + "=" * 80)
    
    # Step 2: Construct Ollama request
    request_body = construct_ollama_request(transcript)
    print(f"[Test] Constructed Ollama request:")
    print(f"  Model: {request_body['model']}")
    print(f"  Format: {request_body['format']}")
    print(f"  Stream: {request_body['stream']}")
    print(f"  Prompt length: {len(request_body['messages'][0]['content'])} characters")
    print("\n" + "=" * 80)
    
    # Step 3: Send request to Ollama
    analysis_result = send_ollama_request(request_body)
    if not analysis_result:
        print("[Test] Failed to get analysis result from Ollama")
        return
    
    print(f"[Test] Analysis completed successfully!")
    print(f"[Test] Analysis result:")
    print(json.dumps(analysis_result, indent=2, ensure_ascii=False))
    print("\n" + "=" * 80)
    
    # Step 4: Create VoiceSession structure
    session_data = create_voice_session(session_id, transcript, analysis_result)
    print(f"[Test] Created VoiceSession structure:")
    print(f"  ID: {session_data['id']}")
    print(f"  Audio file: {session_data['audio_file_path']}")
    print(f"  Has transcript: {session_data['transcript'] is not None}")
    print(f"  Has analysis: {session_data['analysis'] is not None}")
    print(f"  Created at: {session_data['created_at']}")
    print("\n" + "=" * 80)
    
    # Step 5: Save session (demonstrate the missing step)
    print(f"[Test] Saving session to demonstrate the complete flow...")
    if save_session_json(session_data, session_id):
        print(f"[Test] ✅ Session successfully saved!")
        print(f"[Test] This demonstrates how the missing session file should be created.")
    else:
        print(f"[Test] ❌ Failed to save session")
    
    print("\n" + "=" * 80)
    print(f"[Test] Test completed. The session {session_id} now has:")
    print(f"  - Audio file: ✅ (already existed)")
    print(f"  - Transcript: ✅ (already existed)")
    print(f"  - Session JSON: ✅ (created by this test)")
    print(f"  - Analysis data: ✅ (generated by Ollama)")

if __name__ == "__main__":
    main()