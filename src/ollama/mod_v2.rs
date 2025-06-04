use anyhow::{Context, Result};
use log::info;
use reqwest::Client;
use regex;
use serde_json::{json, Value};

use crate::storage::AnalysisResult;

// 检测文本主要语言 (复用原有函数)
pub fn detect_language_v2(text: &str) -> &'static str {
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

// 获取英文 prompt (复用原有函数)
pub fn get_english_prompt_v2(transcript: &str) -> String {
    format!("You are an AI assistant specialized in analyzing meeting transcripts and generating structured insights. Your goal is to process the provided transcript and extract the following information in a well-formatted JSON object:

1.  **Title**: A concise, descriptive title for the entire note, summarizing its main topic.
2.  **Summary**: A concise overview of the main points and outcomes discussed.
3.  **Ideas**: A list of potential ideas or suggestions that arose from the discussion.
4.  **Tasks**: A list of actionable tasks identified, including a title, optional description, and priority (Low, Medium, High, Urgent).
5.  **Structured Notes**: A list of key discussion points or decisions, formatted as structured notes with a title, content, relevant tags (as a list of strings), and a note type (Meeting, Brainstorm, Decision, Action, Reference).

IMPORTANT INSTRUCTIONS:
- Ensure the JSON output is valid and strictly follows the specified structure.
- Do NOT include any other text outside the JSON object.
- Do NOT include any thinking process, explanations, or notes about your analysis.
- ONLY output the final JSON result directly.
- Do NOT use <think> tags or any similar markup.
- If the provided transcript is empty or contains only whitespace, return an empty JSON object `{{}}`.

Transcript: {}

JSON Output:", transcript)
}

// 获取中文 prompt (复用原有函数)
pub fn get_chinese_prompt_v2(transcript: &str) -> String {
    format!("你是一个专业的文本分析助手，专门处理各种类型的文本内容并生成结构化分析。请客观地分析提供的文本内容，并提取以下信息到一个格式良好的JSON对象中：

1.  **title（标题）**: 为文本内容提供一个简洁、描述性的标题，总结其主要话题。
2.  **summary（摘要）**: 对文本的主要观点和内容进行客观、简洁的概述。
3.  **ideas（观点）**: 文本中提到的主要观点、论述或见解列表。
4.  **tasks（要点）**: 文本中提及的重要事项或关键信息，包括标题、可选描述和重要程度（Low、Medium、High、Urgent）。
5.  **structured_notes（结构化笔记）**: 文本的关键信息点，格式化为结构化笔记，包含标题、内容、相关标签（字符串列表）和类型（Meeting、Brainstorm、Decision、Action、Reference）。

重要指示：
- JSON输出格式必须正确且严格遵循指定结构
- 保持客观中立的分析态度
- 不要在JSON对象之外包含任何其他文本
- 不要包含任何思考过程、解释或分析笔记
- 只输出最终的JSON结果
- 不要使用<think>标签或任何类似的标记
- 如果文本为空或仅包含空白字符，返回空的JSON对象 `{{}}`

无论文本内容如何，都请直接输出结构化的JSON分析结果。

Transcript: {}

JSON Output:", transcript)
}

pub async fn analyze_with_ollama_v2(transcript: &str, endpoint: &str) -> Result<AnalysisResult, anyhow::Error> {
    // 使用指定的模型
    let model_name = "deepseek-r1:8b-0528-qwen3-fp16";
    
    if transcript.trim().is_empty() {
        info!("[Ollama V2] Transcript is empty, returning empty analysis result.");
        return Ok(AnalysisResult::default());
    }

    let client = Client::new();
    
    // 检测转录文本的语言
    let language = detect_language_v2(transcript);
    info!("[Ollama V2] Detected language: {}", language);
    
    // 根据语言选择对应的 prompt
    let prompt = match language {
        "zh" => get_chinese_prompt_v2(transcript),
        _ => get_english_prompt_v2(transcript), // 默认使用英文
    };

    info!("[Ollama V2] Using model: {}", model_name);

    let request_body = json!({
        "model": model_name,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "stream": false // 确保非流式响应，便于解析
    });

    // 使用 /api/chat 端点而不是 /api/generate
    let endpoint = format!("{}/api/chat", endpoint.trim_end_matches('/'));
    info!("[Ollama V2] Sending request to: {}", endpoint);

    let response = client
        .post(&endpoint)
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(120)) // 增加超时时间到2分钟
        .send()
        .await
        .with_context(|| format!("Failed to connect to Ollama endpoint: {}", endpoint))?;

    let status = response.status();
    let result_text = response.text().await
        .with_context(|| format!("Failed to read response body from {}. Status: {}", endpoint, status))?;

    // 解析响应
    let parsed_outer_json: Value = match serde_json::from_str(&result_text) {
        Ok(value) => value,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to parse the outer JSON response from Ollama: {}. Response text: {}", e, result_text));
        }
    };

    // 从响应中提取 JSON 内容
    let actual_json_data_str = parsed_outer_json
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .or_else(|| parsed_outer_json.get("response").and_then(|r| r.as_str())) // 备选路径
        .or_else(|| parsed_outer_json.get("content").and_then(|c| c.as_str())); // 备选路径
        
    let actual_json_data_str = match actual_json_data_str {
        Some(s) => s,
        None => {
            // 如果整个响应本身就是 JSON 对象
            if parsed_outer_json.is_object() && parsed_outer_json.get("summary").is_some() {
                 info!("[Ollama V2] Successfully parsed entire response as JSON.");
                 return Ok(serde_json::from_value(parsed_outer_json)?);
            } else if let Ok(analysis_json) = serde_json::from_str::<serde_json::Value>(&result_text) {
                    info!("[Ollama V2] Successfully parsed entire response as JSON.");
                    return Ok(parse_analysis_json(&analysis_json));
                }
                info!("[Ollama V2] Could not extract JSON content string from Ollama's response. Full response: {}", result_text);
                return Err(anyhow::anyhow!("Could not extract or parse JSON content from Ollama's response. Full response: {}", result_text));
            }
        };
    
    // 清理响应中可能存在的思考过程或非 JSON 内容
    let cleaned_json_str = clean_llm_response(actual_json_data_str);
    info!("[Ollama V2] Extracted JSON data string (after cleaning): {}", cleaned_json_str);

    // 尝试解析提取的 JSON 字符串
    let analysis_json: Value = match serde_json::from_str(&cleaned_json_str) {
        Ok(value) => value,
        Err(e) => {
             return Err(anyhow::anyhow!("Failed to parse the inner JSON content from Ollama: {}. Content string: {}", e, cleaned_json_str));
        }
    };
    
    // 解析 JSON 到 AnalysisResult 结构体
    let analysis = parse_analysis_json(&analysis_json);
    
    Ok(analysis)
}

// 辅助函数：解析 JSON 到 AnalysisResult 结构体
fn parse_analysis_json(analysis_json: &Value) -> AnalysisResult {
    AnalysisResult {
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
                    _ => crate::storage::Priority::Medium, // 默认优先级
                };
                Some(crate::storage::Task {
                    title,
                    description,
                    priority,
                    due_date: None,
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
                    _ => crate::storage::NoteType::Reference, // 默认笔记类型
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
    }
}

// 清理 LLM 响应中可能存在的思考过程或非 JSON 内容
fn clean_llm_response(response: &str) -> String {
    let trimmed = response.trim();
    
    // 移除可能的 <think> 标签及其内容
    let re_think = regex::Regex::new(r"(?s)<think>.*?</think>").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
    let without_think = re_think.replace_all(trimmed, "").to_string();
    
    // 尝试找到 JSON 对象的开始位置（第一个 {）
    if let Some(json_start) = without_think.find('{') {
        // 尝试找到 JSON 对象的结束位置（最后一个 }）
        if let Some(json_end) = without_think.rfind('}') {
            if json_end > json_start {
                return without_think[json_start..=json_end].to_string();
            }
        }
    }
    
    // 如果没有找到完整的 JSON 对象，返回清理后的原始响应
    without_think
}