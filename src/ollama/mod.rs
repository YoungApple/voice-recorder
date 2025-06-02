use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::storage::AnalysisResult;

// 检测文本主要语言
fn detect_language(text: &str) -> &'static str {
    let chinese_chars = text.chars().filter(|c| {
        let code = *c as u32;
        // 中文字符范围：基本汉字、扩展A、扩展B等
        (0x4E00..=0x9FFF).contains(&code) || // CJK统一汉字
        (0x3400..=0x4DBF).contains(&code) || // CJK扩展A
        (0x20000..=0x2A6DF).contains(&code) || // CJK扩展B
        (0x2A700..=0x2B73F).contains(&code) || // CJK扩展C
        (0x2B740..=0x2B81F).contains(&code) || // CJK扩展D
        (0x2B820..=0x2CEAF).contains(&code) || // CJK扩展E
        (0x2CEB0..=0x2EBEF).contains(&code) || // CJK扩展F
        (0x30000..=0x3134F).contains(&code)    // CJK扩展G
    }).count();
    
    let total_chars = text.chars().filter(|c| !c.is_whitespace()).count();
    
    if total_chars == 0 {
        return "en"; // 默认英文
    }
    
    // 如果中文字符占比超过30%，认为是中文
    if chinese_chars as f64 / total_chars as f64 > 0.3 {
        "zh"
    } else {
        "en"
    }
}

// 获取英文 prompt
fn get_english_prompt() -> &'static str {
    "You are an AI assistant specialized in analyzing meeting transcripts and generating structured insights. Your goal is to process the provided transcript and extract the following information in a well-formatted JSON object:

1.  **Title**: A concise, descriptive title for the entire note, summarizing its main topic.
2.  **Summary**: A concise overview of the main points and outcomes discussed.
3.  **Ideas**: A list of potential ideas or suggestions that arose from the discussion.
4.  **Tasks**: A list of actionable tasks identified, including a title, optional description, and priority (Low, Medium, High, Urgent).
5.  **Structured Notes**: A list of key discussion points or decisions, formatted as structured notes with a title, content, relevant tags (as a list of strings), and a note type (Meeting, Brainstorm, Decision, Action, Reference).

Ensure the JSON output is valid and strictly follows the specified structure. Do not include any other text outside the JSON object.

If the provided transcript is empty or contains only whitespace, return an empty JSON object `{{}}`."
}

// 获取中文 prompt
fn get_chinese_prompt() -> &'static str {
    "你是一个专业的文本分析助手，专门处理各种类型的文本内容并生成结构化分析。请客观地分析提供的文本内容，并提取以下信息到一个格式良好的JSON对象中：

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

无论文本内容如何，都请进行客观的结构化分析。"
}

pub async fn analyze_with_ollama(transcript: &str, endpoint: &str, model_name: &str) -> Result<AnalysisResult> {
    if transcript.trim().is_empty() {
        println!("[Ollama] Transcript is empty, returning empty analysis result.");
        return Ok(AnalysisResult::default());
    }

    let client = Client::new();
    
    // 检测转录文本的语言
    let language = detect_language(transcript);
    println!("[Ollama] Detected language: {}", language);
    
    // 根据语言选择对应的 prompt
    let base_prompt = match language {
        "zh" => get_chinese_prompt(),
        _ => get_english_prompt(), // 默认使用英文
    };
    
    let prompt = format!("{}\n\nTranscript: {}\n\nJSON Output:", base_prompt, transcript);

    println!("[Ollama] Prompt for {}:\n{}", model_name, prompt);

    let request_body = json!({
        "model": model_name,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "stream": false // Ensure non-streaming response for easier parsing
    });

    println!("[Ollama] Prompt for {}: {}", model_name, prompt);
    println!("[Ollama] Request body for {}: {}", model_name, request_body.to_string());

    let endpoint = format!("{}/api/generate", endpoint.trim_end_matches('/'));
    println!("[Ollama] Sending request to: {} with model: {}", endpoint, model_name);

    let response = client
        .post(&endpoint)
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .with_context(|| format!("Failed to connect to Ollama endpoint: {}", endpoint))?;

    let status = response.status();
    let headers = response.headers().clone();
    let result_text = response.text().await
        .with_context(|| format!("Failed to read response body from {}. Status: {}", endpoint, status))?;

    println!("[Ollama] Response headers: {:?}", headers);

    // Ollama with format: "json" should return a JSON object where one of the fields (e.g., message.content) contains the stringified JSON data.
    let parsed_outer_json: Value = match serde_json::from_str(&result_text) {
        Ok(value) => value,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to parse the outer JSON response from Ollama: {}. Response text: {}", e, result_text));
        }
    };

    // Extract the stringified JSON from the expected field. Common paths are `message.content` or `response`.
    // This depends on the specific Ollama model and version.
    // Let's try common paths.
    let actual_json_data_str = parsed_outer_json
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .or_else(|| parsed_outer_json.get("response").and_then(|r| r.as_str())) // Fallback for some models that put it in "response"
        .or_else(|| parsed_outer_json.get("content").and_then(|c| c.as_str())); // Some models might put it directly in 'content'
        
    let actual_json_data_str = match actual_json_data_str {
        Some(s) => s,
        None => {
            // If the entire response is the JSON object itself (some models might do this with format:json) or if we can parse the whole response as JSON
            if parsed_outer_json.is_object() && parsed_outer_json.get("summary").is_some() {
                 println!("[Ollama] Successfully parsed entire response as JSON.");
                 return Ok(serde_json::from_value(parsed_outer_json)?);
            } else if let Ok(analysis_json) = serde_json::from_str::<serde_json::Value>(&result_text) {
                    println!("[Ollama] Successfully parsed entire response as JSON.");
                    return Ok(crate::storage::AnalysisResult {
                        title: analysis_json.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
                        summary: analysis_json.get("summary").and_then(Value::as_str).unwrap_or("").to_string(),
                        ideas: analysis_json.get("ideas").and_then(Value::as_array).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                        tasks: analysis_json.get("tasks").and_then(Value::as_array).map(|arr| arr.iter().filter_map(|task_val| {
                            let title = task_val.get("title")?.as_str()?.to_string();
                            let description = task_val.get("description").and_then(|d| d.as_str()).map(String::from);
                            let priority_str = task_val.get("priority")?.as_str()?;
                            let priority = match priority_str {
                                "Low" => crate::storage::Priority::Low,
                                "Medium" => crate::storage::Priority::Medium,
                                "High" => crate::storage::Priority::High,
                                "Urgent" => crate::storage::Priority::Urgent,
                                _ => crate::storage::Priority::Medium,
                            };
                            Some(crate::storage::Task { title, description, priority, due_date: None }) }).collect()).unwrap_or_default(),
                        structured_notes: analysis_json.get("structured_notes").and_then(Value::as_array).map(|arr| arr.iter().filter_map(|note_val| {
                            let title = note_val.get("title")?.as_str()?.to_string();
                            let content = note_val.get("content")?.as_str()?.to_string();
                            let tags: Vec<String> = note_val.get("tags")?.as_array()?.iter().filter_map(|tag_val| tag_val.as_str().map(String::from)).collect();
                            let note_type_str = note_val.get("type")?.as_str()?;
                            let note_type = match note_type_str {
                                "Meeting" => crate::storage::NoteType::Meeting,
                                "Brainstorm" => crate::storage::NoteType::Brainstorm,
                                "Decision" => crate::storage::NoteType::Decision,
                                "Action" => crate::storage::NoteType::Action,
                                "Reference" => crate::storage::NoteType::Reference,
                                _ => crate::storage::NoteType::Reference,
                            };
                            Some(crate::storage::StructuredNote { title, content, tags, note_type, created_at: chrono::Utc::now(), updated_at: chrono::Utc::now() }) }).collect()).unwrap_or_default(),
                    });
                }
                println!("[Ollama] Could not extract JSON content string from Ollama's response. Tried 'message.content', 'response', and 'content'. Full response: {}", result_text);
                return Err(anyhow::anyhow!("Could not extract or parse JSON content from Ollama's response. Full response: {}", result_text));
            }
        };
    

    println!("[Ollama] Extracted JSON data string: {}", actual_json_data_str);

    let analysis_json: Value = match serde_json::from_str(actual_json_data_str) {
        Ok(value) => value,
        Err(e) => {
             return Err(anyhow::anyhow!("Failed to parse the inner JSON content from Ollama: {}. Content string: {}", e, actual_json_data_str));
        }
    };
    
    let analysis = crate::storage::AnalysisResult {
        title: analysis_json.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
        summary: analysis_json.get("summary").and_then(Value::as_str).unwrap_or("").to_string(),
        ideas: analysis_json.get("ideas")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        tasks: analysis_json.get("tasks")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(|task_val| {
                let title = task_val.get("title")?.as_str()?.to_string();
                let description = task_val.get("description").and_then(|d| d.as_str()).map(String::from);
                let priority_str = task_val.get("priority")?.as_str()?;
                let priority = match priority_str {
                    "Low" => crate::storage::Priority::Low,
                    "Medium" => crate::storage::Priority::Medium,
                    "High" => crate::storage::Priority::High,
                    "Urgent" => crate::storage::Priority::Urgent,
                    _ => crate::storage::Priority::Medium, // Default priority
                };
                Some(crate::storage::Task {
                    title,
                    description,
                    priority,
                    due_date: None, // due_date not req
                 })
            }).collect())
            .unwrap_or_default(),
        structured_notes: analysis_json.get("structured_notes")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(|note_val| {
                let title = note_val.get("title")?.as_str()?.to_string();
                let content = note_val.get("content")?.as_str()?.to_string();
                let tags: Vec<String> = note_val.get("tags")?.as_array()?
                    .iter()
                    .filter_map(|tag_val| tag_val.as_str().map(String::from))
                    .collect();
                let note_type_str = note_val.get("type")?.as_str()?;
                let note_type = match note_type_str {
                    "Meeting" => crate::storage::NoteType::Meeting,
                    "Brainstorm" => crate::storage::NoteType::Brainstorm,
                    "Decision" => crate::storage::NoteType::Decision,
                    "Action" => crate::storage::NoteType::Action,
                    "Reference" => crate::storage::NoteType::Reference,
                    _ => crate::storage::NoteType::Reference, // Default note type
                };
                Some(crate::storage::StructuredNote {
                    title,
                    content,
                    tags,
                    note_type,

                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                 })
            }).collect())
            .unwrap_or_default(),
    };
    
    Ok(analysis)
}