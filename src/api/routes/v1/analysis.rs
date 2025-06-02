// src/api/routes/v1/analysis.rs
//! Analysis results management API routes
//!
//! This module provides endpoints for managing AI analysis results.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, patch, post},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{
    error::ApiError,
    responses::{ApiResponse, PaginationParams, SearchParams, SortParams},
    AppState, ApiResult,
};
use crate::repository::{
    traits::{AnalysisRepository, NewAnalysisResult, UpdateAnalysisResult},
    RepositoryManager,
};
use crate::services::traits::AnalysisService;

/// Create analysis routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(list_analysis_results).post(create_analysis))
        .route("/:id", get(get_analysis_result).patch(update_analysis_result).delete(delete_analysis_result))
        .route("/:id/export", get(export_analysis_result))
        .route("/transcript/:transcript_id", post(analyze_transcript))
        .route("/text", post(analyze_text))
        .route("/batch", post(batch_analyze))
        .route("/search", get(search_analysis_results))
        .route("/stats", get(analysis_stats))
        .route("/types", get(get_analysis_types))
        .route("/providers", get(get_analysis_providers))
}

#[derive(Debug, Deserialize)]
struct CreateAnalysisRequest {
    transcript_id: Option<Uuid>,
    text_content: Option<String>,
    analysis_type: String, // "summary", "ideas", "tasks", "structured", "custom"
    provider: Option<String>, // "openai" or "ollama"
    language: Option<String>,
    model: Option<String>,
    custom_prompt: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateAnalysisResultRequest {
    result_data: Option<serde_json::Value>,
    confidence_score: Option<f64>,
    status: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AnalysisListQuery {
    #[serde(flatten)]
    pagination: PaginationParams,
    #[serde(flatten)]
    search: SearchParams,
    #[serde(flatten)]
    sort: SortParams,
    session_id: Option<Uuid>,
    transcript_id: Option<Uuid>,
    analysis_type: Option<String>,
    provider: Option<String>,
    language: Option<String>,
    status: Option<String>,
    min_confidence: Option<f64>,
    created_after: Option<chrono::DateTime<chrono::Utc>>,
    created_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct AnalyzeTranscriptRequest {
    analysis_types: Vec<String>,
    provider: Option<String>,
    language: Option<String>,
    model: Option<String>,
    custom_prompts: Option<std::collections::HashMap<String, String>>,
    save_results: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AnalyzeTextRequest {
    text: String,
    analysis_types: Vec<String>,
    provider: Option<String>,
    language: Option<String>,
    model: Option<String>,
    custom_prompts: Option<std::collections::HashMap<String, String>>,
    save_results: Option<bool>,
    session_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct BatchAnalyzeRequest {
    transcript_ids: Vec<Uuid>,
    analysis_types: Vec<String>,
    provider: Option<String>,
    language: Option<String>,
    model: Option<String>,
    custom_prompts: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct AnalysisResultResponse {
    id: Uuid,
    session_id: Uuid,
    transcript_id: Option<Uuid>,
    analysis_type: String,
    provider: String,
    model_used: Option<String>,
    language: String,
    result_data: serde_json::Value,
    confidence_score: Option<f64>,
    processing_time_ms: Option<i64>,
    token_usage: Option<serde_json::Value>,
    status: String,
    metadata: Option<serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    // Related data
    transcript_content: Option<String>,
    session_title: Option<String>,
}

#[derive(Debug, Serialize)]
struct AnalysisStatsResponse {
    total_analyses: i64,
    analysis_types: std::collections::HashMap<String, i64>,
    providers: std::collections::HashMap<String, i64>,
    languages: std::collections::HashMap<String, i64>,
    status_distribution: std::collections::HashMap<String, i64>,
    avg_confidence_score: f64,
    avg_processing_time_ms: f64,
    total_tokens_used: i64,
    analyses_per_day: Vec<DailyCount>,
    success_rate: f64,
}

#[derive(Debug, Serialize)]
struct DailyCount {
    date: chrono::NaiveDate,
    count: i64,
}

#[derive(Debug, Serialize)]
struct BatchAnalysisResponse {
    total_requested: usize,
    successful: usize,
    failed: usize,
    results: Vec<BatchAnalysisResult>,
}

#[derive(Debug, Serialize)]
struct BatchAnalysisResult {
    transcript_id: Uuid,
    analysis_results: Vec<AnalysisResultResponse>,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct AnalysisTypesResponse {
    types: Vec<AnalysisTypeInfo>,
}

#[derive(Debug, Serialize)]
struct AnalysisTypeInfo {
    name: String,
    display_name: String,
    description: String,
    supported_providers: Vec<String>,
    default_prompt: Option<String>,
    output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ProvidersResponse {
    providers: Vec<ProviderInfo>,
}

#[derive(Debug, Serialize)]
struct ProviderInfo {
    name: String,
    display_name: String,
    available: bool,
    supported_models: Vec<String>,
    default_model: String,
    capabilities: Vec<String>,
}

/// List analysis results with filtering and pagination
async fn list_analysis_results<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<AnalysisListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<AnalysisResultResponse>>>> {
    let analysis_results = state.repositories.analysis()
        .find_with_filters(
            query.session_id,
            query.transcript_id,
            query.analysis_type.as_deref(),
            query.provider.as_deref(),
            query.language.as_deref(),
            query.status.as_deref(),
            query.min_confidence,
            query.created_after,
            query.created_before,
            Some(query.pagination.limit),
            Some(query.pagination.offset),
            query.sort.sort_by.as_deref(),
            query.sort.sort_order.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list analysis results: {}", e)))?;

    let total = state.repositories.analysis()
        .count_with_filters(
            query.session_id,
            query.transcript_id,
            query.analysis_type.as_deref(),
            query.provider.as_deref(),
            query.language.as_deref(),
            query.status.as_deref(),
            query.min_confidence,
            query.created_after,
            query.created_before,
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count analysis results: {}", e)))?;

    // Enrich with related data
    let mut responses = Vec::new();
    for analysis in analysis_results {
        let transcript = if let Some(transcript_id) = analysis.transcript_id {
            state.repositories.transcript()
                .find_by_id(transcript_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };
        
        let session = state.repositories.session()
            .find_by_id(analysis.session_id)
            .await
            .ok()
            .flatten();

        responses.push(AnalysisResultResponse {
            id: analysis.id,
            session_id: analysis.session_id,
            transcript_id: analysis.transcript_id,
            analysis_type: analysis.analysis_type,
            provider: analysis.provider,
            model_used: analysis.model_used,
            language: analysis.language,
            result_data: analysis.result_data,
            confidence_score: analysis.confidence_score,
            processing_time_ms: analysis.processing_time_ms,
            token_usage: analysis.token_usage,
            status: analysis.status,
            metadata: analysis.metadata,
            created_at: analysis.created_at,
            updated_at: analysis.updated_at,
            transcript_content: transcript.map(|t| t.content),
            session_title: session.and_then(|s| s.title),
        });
    }

    Ok(Json(ApiResponse {
        data: responses,
        total: Some(total),
        page: Some(query.pagination.page()),
        per_page: Some(query.pagination.limit),
    }))
}

/// Create a new analysis
async fn create_analysis<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<CreateAnalysisRequest>,
) -> ApiResult<Json<ApiResponse<AnalysisResultResponse>>> {
    let analysis_result = if let Some(transcript_id) = request.transcript_id {
        // Analyze existing transcript
        let transcript = state.repositories.transcript()
            .find_by_id(transcript_id)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to get transcript: {}", e)))?
            .ok_or_else(|| ApiError::NotFound("Transcript not found".to_string()))?;

        state.services.analysis()
            .analyze_transcript(
                transcript_id,
                &[request.analysis_type],
                request.provider.as_deref(),
                request.language.as_deref(),
                request.model.as_deref(),
                request.custom_prompt.as_ref().map(|p| {
                    let mut prompts = std::collections::HashMap::new();
                    prompts.insert("custom".to_string(), p.clone());
                    prompts
                }).as_ref(),
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to analyze transcript: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InternalServerError("No analysis result returned".to_string()))?
    } else if let Some(text_content) = request.text_content {
        // Analyze raw text
        state.services.analysis()
            .analyze_text(
                &text_content,
                &[request.analysis_type],
                request.provider.as_deref(),
                request.language.as_deref(),
                request.model.as_deref(),
                request.custom_prompt.as_ref().map(|p| {
                    let mut prompts = std::collections::HashMap::new();
                    prompts.insert("custom".to_string(), p.clone());
                    prompts
                }).as_ref(),
                None, // session_id would need to be provided
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to analyze text: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InternalServerError("No analysis result returned".to_string()))?
    } else {
        return Err(ApiError::BadRequest("Either transcript_id or text_content must be provided".to_string()));
    };

    let transcript = if let Some(transcript_id) = analysis_result.transcript_id {
        state.repositories.transcript()
            .find_by_id(transcript_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    
    let session = state.repositories.session()
        .find_by_id(analysis_result.session_id)
        .await
        .ok()
        .flatten();

    let response = AnalysisResultResponse {
        id: analysis_result.id,
        session_id: analysis_result.session_id,
        transcript_id: analysis_result.transcript_id,
        analysis_type: analysis_result.analysis_type,
        provider: analysis_result.provider,
        model_used: analysis_result.model_used,
        language: analysis_result.language,
        result_data: analysis_result.result_data,
        confidence_score: analysis_result.confidence_score,
        processing_time_ms: analysis_result.processing_time_ms,
        token_usage: analysis_result.token_usage,
        status: analysis_result.status,
        metadata: analysis_result.metadata,
        created_at: analysis_result.created_at,
        updated_at: analysis_result.updated_at,
        transcript_content: transcript.map(|t| t.content),
        session_title: session.and_then(|s| s.title),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get a specific analysis result by ID
async fn get_analysis_result<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<AnalysisResultResponse>>> {
    let analysis_result = state.repositories.analysis()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get analysis result: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Analysis result not found".to_string()))?;

    let transcript = if let Some(transcript_id) = analysis_result.transcript_id {
        state.repositories.transcript()
            .find_by_id(transcript_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    
    let session = state.repositories.session()
        .find_by_id(analysis_result.session_id)
        .await
        .ok()
        .flatten();

    let response = AnalysisResultResponse {
        id: analysis_result.id,
        session_id: analysis_result.session_id,
        transcript_id: analysis_result.transcript_id,
        analysis_type: analysis_result.analysis_type,
        provider: analysis_result.provider,
        model_used: analysis_result.model_used,
        language: analysis_result.language,
        result_data: analysis_result.result_data,
        confidence_score: analysis_result.confidence_score,
        processing_time_ms: analysis_result.processing_time_ms,
        token_usage: analysis_result.token_usage,
        status: analysis_result.status,
        metadata: analysis_result.metadata,
        created_at: analysis_result.created_at,
        updated_at: analysis_result.updated_at,
        transcript_content: transcript.map(|t| t.content),
        session_title: session.and_then(|s| s.title),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Update an analysis result
async fn update_analysis_result<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateAnalysisResultRequest>,
) -> ApiResult<Json<ApiResponse<AnalysisResultResponse>>> {
    // Check if analysis result exists
    let _analysis_result = state.repositories.analysis()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get analysis result: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Analysis result not found".to_string()))?;

    let update_analysis = UpdateAnalysisResult {
        result_data: request.result_data,
        confidence_score: request.confidence_score,
        status: request.status,
        metadata: request.metadata,
    };

    let updated_analysis = state.repositories.analysis()
        .update(id, update_analysis)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to update analysis result: {}", e)))?;

    let transcript = if let Some(transcript_id) = updated_analysis.transcript_id {
        state.repositories.transcript()
            .find_by_id(transcript_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    
    let session = state.repositories.session()
        .find_by_id(updated_analysis.session_id)
        .await
        .ok()
        .flatten();

    let response = AnalysisResultResponse {
        id: updated_analysis.id,
        session_id: updated_analysis.session_id,
        transcript_id: updated_analysis.transcript_id,
        analysis_type: updated_analysis.analysis_type,
        provider: updated_analysis.provider,
        model_used: updated_analysis.model_used,
        language: updated_analysis.language,
        result_data: updated_analysis.result_data,
        confidence_score: updated_analysis.confidence_score,
        processing_time_ms: updated_analysis.processing_time_ms,
        token_usage: updated_analysis.token_usage,
        status: updated_analysis.status,
        metadata: updated_analysis.metadata,
        created_at: updated_analysis.created_at,
        updated_at: updated_analysis.updated_at,
        transcript_content: transcript.map(|t| t.content),
        session_title: session.and_then(|s| s.title),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Delete an analysis result
async fn delete_analysis_result<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Check if analysis result exists
    let _analysis_result = state.repositories.analysis()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get analysis result: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Analysis result not found".to_string()))?;

    state.repositories.analysis()
        .delete(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete analysis result: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Export analysis result
async fn export_analysis_result<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<axum::response::Response> {
    let analysis_result = state.repositories.analysis()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get analysis result: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Analysis result not found".to_string()))?;

    let format = params.get("format").map(|s| s.as_str()).unwrap_or("json");
    let include_metadata = params.get("include_metadata")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);

    let (content, content_type, filename) = match format {
        "json" => {
            let json_data = if include_metadata {
                serde_json::to_string_pretty(&analysis_result)
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize analysis: {}", e)))?
            } else {
                serde_json::to_string_pretty(&analysis_result.result_data)
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize analysis: {}", e)))?
            };
            (json_data, "application/json", format!("analysis_{}.json", analysis_result.id))
        }
        "txt" => {
            let text_content = format!(
                "Analysis Type: {}\nProvider: {}\nLanguage: {}\nCreated: {}\n\nResults:\n{}\n",
                analysis_result.analysis_type,
                analysis_result.provider,
                analysis_result.language,
                analysis_result.created_at,
                serde_json::to_string_pretty(&analysis_result.result_data).unwrap_or_default()
            );
            (text_content, "text/plain", format!("analysis_{}.txt", analysis_result.id))
        }
        "csv" => {
            // Simple CSV export for structured data
            let csv_content = "Type,Provider,Language,Created,Result\n".to_string() +
                &format!(
                    "{},{},{},{},\"{}\"\n",
                    analysis_result.analysis_type,
                    analysis_result.provider,
                    analysis_result.language,
                    analysis_result.created_at,
                    serde_json::to_string(&analysis_result.result_data).unwrap_or_default().replace('"', """")
                );
            (csv_content, "text/csv", format!("analysis_{}.csv", analysis_result.id))
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported export format: {}. Supported formats: json, txt, csv",
                format
            )));
        }
    };

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (
                axum::http::header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        content,
    )
        .into_response())
}

/// Analyze a transcript
async fn analyze_transcript<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(transcript_id): Path<Uuid>,
    Json(request): Json<AnalyzeTranscriptRequest>,
) -> ApiResult<Json<ApiResponse<Vec<AnalysisResultResponse>>>> {
    let analysis_results = state.services.analysis()
        .analyze_transcript(
            transcript_id,
            &request.analysis_types,
            request.provider.as_deref(),
            request.language.as_deref(),
            request.model.as_deref(),
            request.custom_prompts.as_ref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to analyze transcript: {}", e)))?;

    let responses: Vec<AnalysisResultResponse> = analysis_results
        .into_iter()
        .map(|analysis| AnalysisResultResponse {
            id: analysis.id,
            session_id: analysis.session_id,
            transcript_id: analysis.transcript_id,
            analysis_type: analysis.analysis_type,
            provider: analysis.provider,
            model_used: analysis.model_used,
            language: analysis.language,
            result_data: analysis.result_data,
            confidence_score: analysis.confidence_score,
            processing_time_ms: analysis.processing_time_ms,
            token_usage: analysis.token_usage,
            status: analysis.status,
            metadata: analysis.metadata,
            created_at: analysis.created_at,
            updated_at: analysis.updated_at,
            transcript_content: None, // Would need to fetch if required
            session_title: None, // Would need to fetch if required
        })
        .collect();

    Ok(Json(ApiResponse {
        data: responses,
        total: Some(responses.len() as i64),
        page: None,
        per_page: None,
    }))
}

/// Analyze raw text
async fn analyze_text<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<AnalyzeTextRequest>,
) -> ApiResult<Json<ApiResponse<Vec<AnalysisResultResponse>>>> {
    let analysis_results = state.services.analysis()
        .analyze_text(
            &request.text,
            &request.analysis_types,
            request.provider.as_deref(),
            request.language.as_deref(),
            request.model.as_deref(),
            request.custom_prompts.as_ref(),
            request.session_id,
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to analyze text: {}", e)))?;

    let responses: Vec<AnalysisResultResponse> = analysis_results
        .into_iter()
        .map(|analysis| AnalysisResultResponse {
            id: analysis.id,
            session_id: analysis.session_id,
            transcript_id: analysis.transcript_id,
            analysis_type: analysis.analysis_type,
            provider: analysis.provider,
            model_used: analysis.model_used,
            language: analysis.language,
            result_data: analysis.result_data,
            confidence_score: analysis.confidence_score,
            processing_time_ms: analysis.processing_time_ms,
            token_usage: analysis.token_usage,
            status: analysis.status,
            metadata: analysis.metadata,
            created_at: analysis.created_at,
            updated_at: analysis.updated_at,
            transcript_content: None,
            session_title: None,
        })
        .collect();

    Ok(Json(ApiResponse {
        data: responses,
        total: Some(responses.len() as i64),
        page: None,
        per_page: None,
    }))
}

/// Batch analyze multiple transcripts
async fn batch_analyze<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchAnalyzeRequest>,
) -> ApiResult<Json<BatchAnalysisResponse>> {
    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for transcript_id in &request.transcript_ids {
        match state.services.analysis()
            .analyze_transcript(
                *transcript_id,
                &request.analysis_types,
                request.provider.as_deref(),
                request.language.as_deref(),
                request.model.as_deref(),
                request.custom_prompts.as_ref(),
            )
            .await
        {
            Ok(analysis_results) => {
                successful += 1;
                let responses: Vec<AnalysisResultResponse> = analysis_results
                    .into_iter()
                    .map(|analysis| AnalysisResultResponse {
                        id: analysis.id,
                        session_id: analysis.session_id,
                        transcript_id: analysis.transcript_id,
                        analysis_type: analysis.analysis_type,
                        provider: analysis.provider,
                        model_used: analysis.model_used,
                        language: analysis.language,
                        result_data: analysis.result_data,
                        confidence_score: analysis.confidence_score,
                        processing_time_ms: analysis.processing_time_ms,
                        token_usage: analysis.token_usage,
                        status: analysis.status,
                        metadata: analysis.metadata,
                        created_at: analysis.created_at,
                        updated_at: analysis.updated_at,
                        transcript_content: None,
                        session_title: None,
                    })
                    .collect();
                
                results.push(BatchAnalysisResult {
                    transcript_id: *transcript_id,
                    analysis_results: responses,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                results.push(BatchAnalysisResult {
                    transcript_id: *transcript_id,
                    analysis_results: vec![],
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(BatchAnalysisResponse {
        total_requested: request.transcript_ids.len(),
        successful,
        failed,
        results,
    }))
}

/// Search analysis results
async fn search_analysis_results<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<AnalysisListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<AnalysisResultResponse>>>> {
    // Enhanced search functionality
    list_analysis_results(State(state), Query(query)).await
}

/// Get analysis statistics
async fn analysis_stats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<AnalysisStatsResponse>> {
    let stats = state.services.analysis()
        .get_analysis_stats()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get analysis stats: {}", e)))?;

    Ok(Json(stats))
}

/// Get available analysis types
async fn get_analysis_types<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<AnalysisTypesResponse>> {
    let types = vec![
        AnalysisTypeInfo {
            name: "summary".to_string(),
            display_name: "Summary".to_string(),
            description: "Generate a concise summary of the content".to_string(),
            supported_providers: vec!["openai".to_string(), "ollama".to_string()],
            default_prompt: Some("Please provide a concise summary of the following text:".to_string()),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string" }
                }
            })),
        },
        AnalysisTypeInfo {
            name: "ideas".to_string(),
            display_name: "Ideas Extraction".to_string(),
            description: "Extract key ideas and insights from the content".to_string(),
            supported_providers: vec!["openai".to_string(), "ollama".to_string()],
            default_prompt: Some("Extract key ideas and insights from the following text. Format as JSON with an 'ideas' array:".to_string()),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "ideas": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "description": { "type": "string" },
                                "category": { "type": "string" }
                            }
                        }
                    }
                }
            })),
        },
        AnalysisTypeInfo {
            name: "tasks".to_string(),
            display_name: "Task Extraction".to_string(),
            description: "Extract actionable tasks and to-dos from the content".to_string(),
            supported_providers: vec!["openai".to_string(), "ollama".to_string()],
            default_prompt: Some("Extract actionable tasks and to-dos from the following text. Format as JSON with a 'tasks' array:".to_string()),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "description": { "type": "string" },
                                "priority": { "type": "string" },
                                "due_date": { "type": "string" }
                            }
                        }
                    }
                }
            })),
        },
        AnalysisTypeInfo {
            name: "structured".to_string(),
            display_name: "Structured Analysis".to_string(),
            description: "Comprehensive structured analysis including summary, ideas, and tasks".to_string(),
            supported_providers: vec!["openai".to_string(), "ollama".to_string()],
            default_prompt: Some("Analyze the following text and provide a structured breakdown including summary, key points, ideas, and tasks. Format as JSON:".to_string()),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string" },
                    "key_points": { "type": "array", "items": { "type": "string" } },
                    "ideas": { "type": "array" },
                    "tasks": { "type": "array" }
                }
            })),
        },
    ];

    Ok(Json(AnalysisTypesResponse { types }))
}

/// Get available analysis providers
async fn get_analysis_providers<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<ProvidersResponse>> {
    let ollama_available = state.services.ollama().is_available().await;
    let openai_configured = state.config.is_openai_configured();

    let mut providers = vec![];

    if openai_configured {
        providers.push(ProviderInfo {
            name: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            available: true,
            supported_models: vec![
                "gpt-4".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
            default_model: state.config.openai.analysis_model.clone(),
            capabilities: vec![
                "text_analysis".to_string(),
                "structured_output".to_string(),
                "multilingual".to_string(),
            ],
        });
    }

    let ollama_models = if ollama_available {
        state.services.ollama().list_models().await.unwrap_or_default()
    } else {
        vec![]
    };

    providers.push(ProviderInfo {
        name: "ollama".to_string(),
        display_name: "Ollama (Local)".to_string(),
        available: ollama_available,
        supported_models: ollama_models,
        default_model: state.config.ollama.default_model.clone(),
        capabilities: vec![
            "text_analysis".to_string(),
            "structured_output".to_string(),
            "language_detection".to_string(),
        ],
    });

    Ok(Json(ProvidersResponse { providers }))
}