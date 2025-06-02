// src/services/ollama.rs
//! Ollama service implementation for local AI model integration
//!
//! This module provides the implementation for interacting with Ollama,
//! a local AI model server that can run various language models.

use async_trait::async_trait;
use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use super::traits::{OllamaService, OllamaModel, OllamaModelDetails, OllamaOptions};

/// Ollama service implementation
pub struct OllamaServiceImpl {
    client: Client,
    base_url: String,
}

impl OllamaServiceImpl {
    /// Create a new Ollama service instance
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minutes timeout for model operations
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
    
    /// Build URL for Ollama API endpoint
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}/api/{}", self.base_url, endpoint.trim_start_matches('/'))
    }
}

#[async_trait]
impl OllamaService for OllamaServiceImpl {
    async fn is_available(&self) -> bool {
        match self.client.get(&self.build_url("tags")).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
    
    async fn list_models(&self) -> Result<Vec<OllamaModel>> {
        let response = self
            .client
            .get(&self.build_url("tags"))
            .send()
            .await
            .context("Failed to send request to Ollama")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Ollama API returned error: {}",
                response.status()
            ));
        }
        
        let response_data: OllamaTagsResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;
        
        Ok(response_data.models)
    }
    
    async fn pull_model(&self, model_name: &str) -> Result<()> {
        let request_body = OllamaPullRequest {
            name: model_name.to_string(),
            stream: Some(false),
        };
        
        let response = self
            .client
            .post(&self.build_url("pull"))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send pull request to Ollama")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to pull model '{}': {}",
                model_name,
                error_text
            ));
        }
        
        Ok(())
    }
    
    async fn generate(
        &self,
        model: &str,
        prompt: &str,
        options: Option<OllamaOptions>,
    ) -> Result<String> {
        let request_body = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: Some(false),
            options: options.map(|opts| serde_json::to_value(opts).unwrap_or_default()),
        };
        
        let response = self
            .client
            .post(&self.build_url("generate"))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send generate request to Ollama")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Ollama generation failed: {}",
                error_text
            ));
        }
        
        let response_data: OllamaGenerateResponse = response
            .json()
            .await
            .context("Failed to parse Ollama generate response")?;
        
        Ok(response_data.response)
    }
    
    async fn generate_structured<T>(
        &self,
        model: &str,
        prompt: &str,
        options: Option<OllamaOptions>,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Add JSON format instruction to the prompt
        let structured_prompt = format!(
            "{}

Please respond with valid JSON only, no additional text or explanation.",
            prompt
        );
        
        let response_text = self.generate(model, &structured_prompt, options).await?;
        
        // Try to extract JSON from the response
        let json_text = self.extract_json_from_response(&response_text)
            .unwrap_or(response_text.as_str());
        
        serde_json::from_str(json_text)
            .context("Failed to parse structured response as JSON")
    }
    
    async fn get_model_info(&self, model_name: &str) -> Result<Option<OllamaModel>> {
        let models = self.list_models().await?;
        Ok(models.into_iter().find(|m| m.name == model_name))
    }
}

impl OllamaServiceImpl {
    /// Extract JSON content from a response that might contain additional text
    fn extract_json_from_response(&self, response: &str) -> Option<&str> {
        // Look for JSON object boundaries
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end > start {
                    return Some(&response[start..=end]);
                }
            }
        }
        
        // Look for JSON array boundaries
        if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                if end > start {
                    return Some(&response[start..=end]);
                }
            }
        }
        
        None
    }
}

// Ollama API request/response types

#[derive(Debug, Serialize)]
struct OllamaPullRequest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[serde(default)]
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    load_duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_eval_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_eval_duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eval_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eval_duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

/// Enhanced Ollama service with language detection integration
pub struct EnhancedOllamaService {
    ollama: OllamaServiceImpl,
}

impl EnhancedOllamaService {
    pub fn new(base_url: &str) -> Self {
        Self {
            ollama: OllamaServiceImpl::new(base_url),
        }
    }
    
    /// Analyze text with automatic language detection and appropriate prompting
    pub async fn analyze_with_language_detection(
        &self,
        model: &str,
        content: &str,
        options: Option<OllamaOptions>,
    ) -> Result<serde_json::Value> {
        // Detect language (reuse the logic from ollama/mod.rs)
        let language = self.detect_language(content);
        
        // Get appropriate prompt based on language
        let prompt = if language == "chinese" {
            self.get_chinese_prompt(content)
        } else {
            self.get_english_prompt(content)
        };
        
        // Generate structured response
        self.ollama.generate_structured(model, &prompt, options).await
    }
    
    /// Detect language based on Chinese character ratio
    fn detect_language(&self, text: &str) -> &'static str {
        let total_chars = text.chars().count();
        if total_chars == 0 {
            return "english";
        }
        
        let chinese_chars = text.chars()
            .filter(|c| {
                let code = *c as u32;
                // Chinese character ranges
                (0x4E00..=0x9FFF).contains(&code) ||  // CJK Unified Ideographs
                (0x3400..=0x4DBF).contains(&code) ||  // CJK Extension A
                (0x20000..=0x2A6DF).contains(&code) || // CJK Extension B
                (0x2A700..=0x2B73F).contains(&code) || // CJK Extension C
                (0x2B740..=0x2B81F).contains(&code) || // CJK Extension D
                (0x2B820..=0x2CEAF).contains(&code) || // CJK Extension E
                (0xF900..=0xFAFF).contains(&code) ||   // CJK Compatibility Ideographs
                (0x2F800..=0x2FA1F).contains(&code)    // CJK Compatibility Supplement
            })
            .count();
        
        let chinese_ratio = chinese_chars as f64 / total_chars as f64;
        
        if chinese_ratio > 0.3 {
            "chinese"
        } else {
            "english"
        }
    }
    
    /// Get Chinese prompt for analysis
    fn get_chinese_prompt(&self, content: &str) -> String {
        format!(
            r#"你是一个专业的文本分析助手。请以客观中立的分析态度，对以下转录内容进行分析，并以JSON格式返回结果。

转录内容：
{}

请提供以下分析结果（必须是有效的JSON格式）：
{{
  "title": "为这段内容生成一个简洁的标题",
  "summary": "提供内容的简要摘要",
  "ideas": [
    {{
      "content": "提取的想法或观点",
      "category": "想法的分类",
      "priority": 1
    }}
  ],
  "tasks": [
    {{
      "title": "任务标题",
      "description": "任务描述",
      "priority": "high",
      "due_date": null
    }}
  ],
  "structured_notes": [
    {{
      "title": "笔记标题",
      "content": "笔记内容",
      "note_type": "summary",
      "tags": ["标签1", "标签2"]
    }}
  ]
}}

请确保返回的是有效的JSON格式，不要包含任何其他文本。"#,
            content
        )
    }
    
    /// Get English prompt for analysis
    fn get_english_prompt(&self, content: &str) -> String {
        format!(
            r#"You are a professional text analysis assistant. Please analyze the following transcript content objectively and return the results in JSON format.

Transcript content:
{}

Please provide the following analysis results (must be valid JSON format):
{{
  "title": "Generate a concise title for this content",
  "summary": "Provide a brief summary of the content",
  "ideas": [
    {{
      "content": "Extracted idea or insight",
      "category": "Category of the idea",
      "priority": 1
    }}
  ],
  "tasks": [
    {{
      "title": "Task title",
      "description": "Task description",
      "priority": "high",
      "due_date": null
    }}
  ],
  "structured_notes": [
    {{
      "title": "Note title",
      "content": "Note content",
      "note_type": "summary",
      "tags": ["tag1", "tag2"]
    }}
  ]
}}

Please ensure the response is valid JSON format without any additional text."#,
            content
        )
    }
}

#[async_trait]
impl OllamaService for EnhancedOllamaService {
    async fn is_available(&self) -> bool {
        self.ollama.is_available().await
    }
    
    async fn list_models(&self) -> Result<Vec<OllamaModel>> {
        self.ollama.list_models().await
    }
    
    async fn pull_model(&self, model_name: &str) -> Result<()> {
        self.ollama.pull_model(model_name).await
    }
    
    async fn generate(
        &self,
        model: &str,
        prompt: &str,
        options: Option<OllamaOptions>,
    ) -> Result<String> {
        self.ollama.generate(model, prompt, options).await
    }
    
    async fn generate_structured<T>(
        &self,
        model: &str,
        prompt: &str,
        options: Option<OllamaOptions>,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.ollama.generate_structured(model, prompt, options).await
    }
    
    async fn get_model_info(&self, model_name: &str) -> Result<Option<OllamaModel>> {
        self.ollama.get_model_info(model_name).await
    }
}