// src/api/routes/v1/ollama.rs
//! Ollama service API routes
//!
//! This module provides endpoints for interacting with the Ollama local AI service.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::api::{
    error::ApiError,
    responses::ApiResponse,
    AppState, ApiResult,
};
use crate::repository::RepositoryManager;
use crate::services::traits::OllamaService;

/// Create Ollama routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/status", get(get_ollama_status))
        .route("/models", get(list_models))
        .route("/models/pull", post(pull_model))
        .route("/models/delete", post(delete_model))
        .route("/generate", post(generate_text))
        .route("/chat", post(chat_completion))
        .route("/embeddings", post(generate_embeddings))
        .route("/detect-language", post(detect_language))
        .route("/analyze", post(analyze_text))
        .route("/config", get(get_ollama_config))
}

#[derive(Debug, Deserialize)]
struct PullModelRequest {
    model: String,
    insecure: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DeleteModelRequest {
    model: String,
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    system: Option<String>,
    template: Option<String>,
    context: Option<Vec<i32>>,
    stream: Option<bool>,
    raw: Option<bool>,
    format: Option<String>, // "json" for structured output
    options: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    role: String, // "system", "user", "assistant"
    content: String,
    images: Option<Vec<String>>, // Base64 encoded images
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: Option<bool>,
    format: Option<String>,
    options: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingsRequest {
    model: String,
    prompt: String,
    options: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct DetectLanguageRequest {
    text: String,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnalyzeTextRequest {
    text: String,
    analysis_type: String, // "summary", "ideas", "tasks", "structured"
    language: Option<String>,
    model: Option<String>,
    custom_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
struct OllamaStatusResponse {
    available: bool,
    base_url: String,
    version: Option<String>,
    models_count: usize,
    default_model: String,
    last_check: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
struct ModelInfo {
    name: String,
    size: Option<u64>,
    digest: Option<String>,
    modified_at: Option<chrono::DateTime<chrono::Utc>>,
    details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct GenerateResponse {
    model: String,
    response: String,
    done: bool,
    context: Option<Vec<i32>>,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
    prompt_eval_duration: Option<u64>,
    eval_count: Option<u32>,
    eval_duration: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    model: String,
    message: ChatMessage,
    done: bool,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
    prompt_eval_duration: Option<u64>,
    eval_count: Option<u32>,
    eval_duration: Option<u64>,
}

#[derive(Debug, Serialize)]
struct EmbeddingsResponse {
    embedding: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct LanguageDetectionResponse {
    language: String,
    confidence: f64,
    detected_languages: Vec<LanguageCandidate>,
}

#[derive(Debug, Serialize)]
struct LanguageCandidate {
    language: String,
    confidence: f64,
}

#[derive(Debug, Serialize)]
struct AnalysisResponse {
    analysis_type: String,
    result: serde_json::Value,
    language: String,
    model_used: String,
    processing_time_ms: u64,
}

#[derive(Debug, Serialize)]
struct OllamaConfigResponse {
    base_url: String,
    default_model: String,
    timeout_seconds: u64,
    max_retries: u32,
    available_models: Vec<String>,
    supported_formats: Vec<String>,
}

/// Get Ollama service status
async fn get_ollama_status<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<ApiResponse<OllamaStatusResponse>>> {
    let is_available = state.services.ollama().is_available().await;
    
    let models = if is_available {
        state.services.ollama().list_models().await.unwrap_or_default()
    } else {
        vec![]
    };

    let response = OllamaStatusResponse {
        available: is_available,
        base_url: state.config.ollama.base_url.clone(),
        version: None, // TODO: Get version from Ollama API
        models_count: models.len(),
        default_model: state.config.ollama.default_model.clone(),
        last_check: chrono::Utc::now(),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// List available models
async fn list_models<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<ApiResponse<Vec<ModelInfo>>>> {
    let models = state.services.ollama()
        .list_models()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list models: {}", e)))?;

    let model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|model| ModelInfo {
            name: model,
            size: None, // TODO: Get model size from Ollama API
            digest: None,
            modified_at: None,
            details: None,
        })
        .collect();

    Ok(Json(ApiResponse {
        data: model_infos,
        total: Some(model_infos.len() as i64),
        page: None,
        per_page: None,
    }))
}

/// Pull a model from Ollama registry
async fn pull_model<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<PullModelRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    state.services.ollama()
        .pull_model(&request.model)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to pull model: {}", e)))?;

    Ok(Json(serde_json::json!({
        "message": format!("Model '{}' pulled successfully", request.model),
        "model": request.model
    })))
}

/// Delete a model
async fn delete_model<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<DeleteModelRequest>,
) -> ApiResult<StatusCode> {
    // Note: Ollama doesn't have a direct delete API, so this might not be implemented
    Err(ApiError::NotImplemented("Model deletion not supported by Ollama".to_string()))
}

/// Generate text using Ollama
async fn generate_text<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<GenerateRequest>,
) -> ApiResult<Json<GenerateResponse>> {
    let start_time = std::time::Instant::now();
    
    let response_text = if request.format.as_deref() == Some("json") {
        state.services.ollama()
            .generate_structured(&request.model, &request.prompt, request.system.as_deref())
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to generate structured text: {}", e)))?
    } else {
        state.services.ollama()
            .generate_text(&request.model, &request.prompt, request.system.as_deref())
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to generate text: {}", e)))?
    };

    let duration = start_time.elapsed();

    let response = GenerateResponse {
        model: request.model,
        response: response_text,
        done: true,
        context: None,
        total_duration: Some(duration.as_nanos() as u64),
        load_duration: None,
        prompt_eval_count: None,
        prompt_eval_duration: None,
        eval_count: None,
        eval_duration: Some(duration.as_nanos() as u64),
    };

    Ok(Json(response))
}

/// Chat completion using Ollama
async fn chat_completion<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<ChatRequest>,
) -> ApiResult<Json<ChatResponse>> {
    let start_time = std::time::Instant::now();
    
    // Convert messages to a single prompt for now
    // TODO: Implement proper chat conversation handling
    let prompt = request.messages
        .iter()
        .map(|msg| format!("{}: {}", msg.role, msg.content))
        .collect::<Vec<_>>()
        .join("\n");

    let response_text = state.services.ollama()
        .generate_text(&request.model, &prompt, None)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to generate chat response: {}", e)))?;

    let duration = start_time.elapsed();

    let response = ChatResponse {
        model: request.model,
        message: ChatMessage {
            role: "assistant".to_string(),
            content: response_text,
            images: None,
        },
        done: true,
        total_duration: Some(duration.as_nanos() as u64),
        load_duration: None,
        prompt_eval_count: None,
        prompt_eval_duration: None,
        eval_count: None,
        eval_duration: Some(duration.as_nanos() as u64),
    };

    Ok(Json(response))
}

/// Generate embeddings using Ollama
async fn generate_embeddings<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<EmbeddingsRequest>,
) -> ApiResult<Json<EmbeddingsResponse>> {
    // Note: This would require an embedding model in Ollama
    // For now, return an error as this feature might not be available
    Err(ApiError::NotImplemented("Embeddings not yet supported".to_string()))
}

/// Detect language of text
async fn detect_language<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<DetectLanguageRequest>,
) -> ApiResult<Json<ApiResponse<LanguageDetectionResponse>>> {
    let model = request.model.unwrap_or_else(|| state.config.ollama.default_model.clone());
    
    let detected_language = state.services.ollama()
        .detect_language(&request.text, &model)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to detect language: {}", e)))?;

    let response = LanguageDetectionResponse {
        language: detected_language.clone(),
        confidence: 0.95, // TODO: Get actual confidence from model
        detected_languages: vec![
            LanguageCandidate {
                language: detected_language,
                confidence: 0.95,
            }
        ],
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Analyze text using Ollama
async fn analyze_text<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<AnalyzeTextRequest>,
) -> ApiResult<Json<ApiResponse<AnalysisResponse>>> {
    let start_time = std::time::Instant::now();
    
    let model = request.model.unwrap_or_else(|| state.config.ollama.default_model.clone());
    
    // Detect language if not provided
    let language = match request.language {
        Some(lang) => lang,
        None => {
            state.services.ollama()
                .detect_language(&request.text, &model)
                .await
                .unwrap_or_else(|_| "en".to_string())
        }
    };

    // Generate analysis based on type
    let result = match request.analysis_type.as_str() {
        "summary" => {
            let prompt = if let Some(custom) = request.custom_prompt {
                custom
            } else {
                format!("Please provide a concise summary of the following text:\n\n{}", request.text)
            };
            
            let summary = state.services.ollama()
                .generate_text(&model, &prompt, None)
                .await
                .map_err(|e| ApiError::InternalServerError(format!("Failed to generate summary: {}", e)))?;
            
            serde_json::json!({
                "summary": summary
            })
        }
        "ideas" => {
            let prompt = format!(
                "Extract key ideas and insights from the following text. Format as JSON with an 'ideas' array:\n\n{}",
                request.text
            );
            
            let ideas_json = state.services.ollama()
                .generate_structured(&model, &prompt, None)
                .await
                .map_err(|e| ApiError::InternalServerError(format!("Failed to extract ideas: {}", e)))?;
            
            serde_json::from_str(&ideas_json)
                .unwrap_or_else(|_| serde_json::json!({ "ideas": [] }))
        }
        "tasks" => {
            let prompt = format!(
                "Extract actionable tasks and to-dos from the following text. Format as JSON with a 'tasks' array:\n\n{}",
                request.text
            );
            
            let tasks_json = state.services.ollama()
                .generate_structured(&model, &prompt, None)
                .await
                .map_err(|e| ApiError::InternalServerError(format!("Failed to extract tasks: {}", e)))?;
            
            serde_json::from_str(&tasks_json)
                .unwrap_or_else(|_| serde_json::json!({ "tasks": [] }))
        }
        "structured" => {
            let prompt = format!(
                "Analyze the following text and provide a structured breakdown including summary, key points, ideas, and tasks. Format as JSON:\n\n{}",
                request.text
            );
            
            let structured_json = state.services.ollama()
                .generate_structured(&model, &prompt, None)
                .await
                .map_err(|e| ApiError::InternalServerError(format!("Failed to generate structured analysis: {}", e)))?;
            
            serde_json::from_str(&structured_json)
                .unwrap_or_else(|_| serde_json::json!({
                    "summary": "",
                    "key_points": [],
                    "ideas": [],
                    "tasks": []
                }))
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported analysis type: {}. Supported types: summary, ideas, tasks, structured",
                request.analysis_type
            )));
        }
    };

    let duration = start_time.elapsed();

    let response = AnalysisResponse {
        analysis_type: request.analysis_type,
        result,
        language,
        model_used: model,
        processing_time_ms: duration.as_millis() as u64,
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get Ollama configuration
async fn get_ollama_config<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<ApiResponse<OllamaConfigResponse>>> {
    let available_models = state.services.ollama()
        .list_models()
        .await
        .unwrap_or_default();

    let response = OllamaConfigResponse {
        base_url: state.config.ollama.base_url.clone(),
        default_model: state.config.ollama.default_model.clone(),
        timeout_seconds: state.config.ollama.timeout_seconds,
        max_retries: state.config.ollama.max_retries,
        available_models,
        supported_formats: vec!["text".to_string(), "json".to_string()],
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}