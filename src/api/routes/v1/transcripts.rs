// src/api/routes/v1/transcripts.rs
//! Transcript management API routes
//!
//! This module provides endpoints for managing audio transcriptions.

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
    traits::{NewTranscript, TranscriptRepository, UpdateTranscript},
    RepositoryManager,
};
use crate::services::traits::TranscriptionService;

/// Create transcript routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(list_transcripts).post(create_transcript))
        .route("/:id", get(get_transcript).patch(update_transcript).delete(delete_transcript))
        .route("/:id/export", get(export_transcript))
        .route("/:id/analyze", post(analyze_transcript))
        .route("/search", get(search_transcripts))
        .route("/stats", get(transcript_stats))
        .route("/batch/create", post(batch_create_transcripts))
        .route("/batch/export", post(batch_export_transcripts))
}

#[derive(Debug, Deserialize)]
struct CreateTranscriptRequest {
    audio_file_id: Uuid,
    provider: Option<String>, // "openai" or "local"
    language: Option<String>,
    auto_detect_language: Option<bool>,
    model: Option<String>,
    prompt: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateTranscriptRequest {
    content: Option<String>,
    language: Option<String>,
    confidence_score: Option<f64>,
    metadata: Option<serde_json::Value>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranscriptListQuery {
    #[serde(flatten)]
    pagination: PaginationParams,
    #[serde(flatten)]
    search: SearchParams,
    #[serde(flatten)]
    sort: SortParams,
    session_id: Option<Uuid>,
    audio_file_id: Option<Uuid>,
    language: Option<String>,
    provider: Option<String>,
    status: Option<String>,
    min_confidence: Option<f64>,
    created_after: Option<chrono::DateTime<chrono::Utc>>,
    created_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct AnalyzeTranscriptRequest {
    analysis_types: Vec<String>, // ["summary", "ideas", "tasks", "structured"]
    provider: Option<String>,
    language: Option<String>,
    model: Option<String>,
    custom_prompts: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct BatchCreateRequest {
    audio_file_ids: Vec<Uuid>,
    provider: Option<String>,
    language: Option<String>,
    auto_detect_language: Option<bool>,
    model: Option<String>,
    prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BatchExportRequest {
    transcript_ids: Vec<Uuid>,
    format: String, // "txt", "json", "srt", "vtt"
    include_metadata: Option<bool>,
    include_timestamps: Option<bool>,
}

#[derive(Debug, Serialize)]
struct TranscriptResponse {
    id: Uuid,
    session_id: Uuid,
    audio_file_id: Uuid,
    content: String,
    language: String,
    confidence_score: Option<f64>,
    provider: String,
    model_used: Option<String>,
    processing_time_ms: Option<i64>,
    word_count: i32,
    character_count: i32,
    status: String,
    metadata: Option<serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    // Related data
    audio_filename: Option<String>,
    session_title: Option<String>,
}

#[derive(Debug, Serialize)]
struct TranscriptStatsResponse {
    total_transcripts: i64,
    total_words: i64,
    total_characters: i64,
    avg_confidence_score: f64,
    languages: std::collections::HashMap<String, i64>,
    providers: std::collections::HashMap<String, i64>,
    status_distribution: std::collections::HashMap<String, i64>,
    avg_processing_time_ms: f64,
    transcripts_per_day: Vec<DailyCount>,
}

#[derive(Debug, Serialize)]
struct DailyCount {
    date: chrono::NaiveDate,
    count: i64,
}

#[derive(Debug, Serialize)]
struct BatchOperationResponse {
    total_requested: usize,
    successful: usize,
    failed: usize,
    results: Vec<BatchResult>,
}

#[derive(Debug, Serialize)]
struct BatchResult {
    id: Option<Uuid>,
    success: bool,
    error: Option<String>,
}

/// List transcripts with filtering and pagination
async fn list_transcripts<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<TranscriptListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TranscriptResponse>>>> {
    let transcripts = state.repositories.transcript()
        .find_with_filters(
            query.session_id,
            query.audio_file_id,
            query.language.as_deref(),
            query.provider.as_deref(),
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
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list transcripts: {}", e)))?;

    let total = state.repositories.transcript()
        .count_with_filters(
            query.session_id,
            query.audio_file_id,
            query.language.as_deref(),
            query.provider.as_deref(),
            query.status.as_deref(),
            query.min_confidence,
            query.created_after,
            query.created_before,
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count transcripts: {}", e)))?;

    // Enrich with related data
    let mut responses = Vec::new();
    for transcript in transcripts {
        let audio_file = state.repositories.audio()
            .find_by_id(transcript.audio_file_id)
            .await
            .ok()
            .flatten();
        
        let session = state.repositories.session()
            .find_by_id(transcript.session_id)
            .await
            .ok()
            .flatten();

        responses.push(TranscriptResponse {
            id: transcript.id,
            session_id: transcript.session_id,
            audio_file_id: transcript.audio_file_id,
            content: transcript.content,
            language: transcript.language,
            confidence_score: transcript.confidence_score,
            provider: transcript.provider,
            model_used: transcript.model_used,
            processing_time_ms: transcript.processing_time_ms,
            word_count: transcript.word_count,
            character_count: transcript.character_count,
            status: transcript.status,
            metadata: transcript.metadata,
            created_at: transcript.created_at,
            updated_at: transcript.updated_at,
            audio_filename: audio_file.map(|a| a.original_filename),
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

/// Create a new transcript
async fn create_transcript<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<CreateTranscriptRequest>,
) -> ApiResult<Json<ApiResponse<TranscriptResponse>>> {
    // Verify audio file exists
    let audio_file = state.repositories.audio()
        .find_by_id(request.audio_file_id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio file: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Audio file not found".to_string()))?;

    // Create transcript using transcription service
    let transcript = state.services.transcription()
        .transcribe_audio(
            request.audio_file_id,
            request.provider.as_deref(),
            request.language.as_deref(),
            request.auto_detect_language.unwrap_or(true),
            request.model.as_deref(),
            request.prompt.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create transcript: {}", e)))?;

    let session = state.repositories.session()
        .find_by_id(transcript.session_id)
        .await
        .ok()
        .flatten();

    let response = TranscriptResponse {
        id: transcript.id,
        session_id: transcript.session_id,
        audio_file_id: transcript.audio_file_id,
        content: transcript.content,
        language: transcript.language,
        confidence_score: transcript.confidence_score,
        provider: transcript.provider,
        model_used: transcript.model_used,
        processing_time_ms: transcript.processing_time_ms,
        word_count: transcript.word_count,
        character_count: transcript.character_count,
        status: transcript.status,
        metadata: transcript.metadata,
        created_at: transcript.created_at,
        updated_at: transcript.updated_at,
        audio_filename: Some(audio_file.original_filename),
        session_title: session.and_then(|s| s.title),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get a specific transcript by ID
async fn get_transcript<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<TranscriptResponse>>> {
    let transcript = state.repositories.transcript()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get transcript: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Transcript not found".to_string()))?;

    let audio_file = state.repositories.audio()
        .find_by_id(transcript.audio_file_id)
        .await
        .ok()
        .flatten();
    
    let session = state.repositories.session()
        .find_by_id(transcript.session_id)
        .await
        .ok()
        .flatten();

    let response = TranscriptResponse {
        id: transcript.id,
        session_id: transcript.session_id,
        audio_file_id: transcript.audio_file_id,
        content: transcript.content,
        language: transcript.language,
        confidence_score: transcript.confidence_score,
        provider: transcript.provider,
        model_used: transcript.model_used,
        processing_time_ms: transcript.processing_time_ms,
        word_count: transcript.word_count,
        character_count: transcript.character_count,
        status: transcript.status,
        metadata: transcript.metadata,
        created_at: transcript.created_at,
        updated_at: transcript.updated_at,
        audio_filename: audio_file.map(|a| a.original_filename),
        session_title: session.and_then(|s| s.title),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Update a transcript
async fn update_transcript<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateTranscriptRequest>,
) -> ApiResult<Json<ApiResponse<TranscriptResponse>>> {
    // Check if transcript exists
    let _transcript = state.repositories.transcript()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get transcript: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Transcript not found".to_string()))?;

    let update_transcript = UpdateTranscript {
        content: request.content,
        language: request.language,
        confidence_score: request.confidence_score,
        status: request.status,
        metadata: request.metadata,
    };

    let updated_transcript = state.repositories.transcript()
        .update(id, update_transcript)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to update transcript: {}", e)))?;

    let audio_file = state.repositories.audio()
        .find_by_id(updated_transcript.audio_file_id)
        .await
        .ok()
        .flatten();
    
    let session = state.repositories.session()
        .find_by_id(updated_transcript.session_id)
        .await
        .ok()
        .flatten();

    let response = TranscriptResponse {
        id: updated_transcript.id,
        session_id: updated_transcript.session_id,
        audio_file_id: updated_transcript.audio_file_id,
        content: updated_transcript.content,
        language: updated_transcript.language,
        confidence_score: updated_transcript.confidence_score,
        provider: updated_transcript.provider,
        model_used: updated_transcript.model_used,
        processing_time_ms: updated_transcript.processing_time_ms,
        word_count: updated_transcript.word_count,
        character_count: updated_transcript.character_count,
        status: updated_transcript.status,
        metadata: updated_transcript.metadata,
        created_at: updated_transcript.created_at,
        updated_at: updated_transcript.updated_at,
        audio_filename: audio_file.map(|a| a.original_filename),
        session_title: session.and_then(|s| s.title),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Delete a transcript
async fn delete_transcript<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Check if transcript exists
    let _transcript = state.repositories.transcript()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get transcript: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Transcript not found".to_string()))?;

    state.repositories.transcript()
        .delete(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete transcript: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Export transcript in various formats
async fn export_transcript<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<axum::response::Response> {
    let transcript = state.repositories.transcript()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get transcript: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Transcript not found".to_string()))?;

    let format = params.get("format").map(|s| s.as_str()).unwrap_or("txt");
    let include_metadata = params.get("include_metadata")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    let (content, content_type, filename) = match format {
        "txt" => {
            let content = if include_metadata {
                format!(
                    "Language: {}\nProvider: {}\nConfidence: {:?}\nCreated: {}\n\n{}",
                    transcript.language,
                    transcript.provider,
                    transcript.confidence_score,
                    transcript.created_at,
                    transcript.content
                )
            } else {
                transcript.content
            };
            (content, "text/plain", format!("transcript_{}.txt", transcript.id))
        }
        "json" => {
            let json_data = if include_metadata {
                serde_json::to_string_pretty(&transcript)
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize transcript: {}", e)))?
            } else {
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": transcript.id,
                    "content": transcript.content,
                    "language": transcript.language
                }))
                .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize transcript: {}", e)))?
            };
            (json_data, "application/json", format!("transcript_{}.json", transcript.id))
        }
        "srt" => {
            // Simple SRT format (would need proper timestamp data)
            let srt_content = format!(
                "1\n00:00:00,000 --> 00:00:10,000\n{}\n",
                transcript.content
            );
            (srt_content, "text/srt", format!("transcript_{}.srt", transcript.id))
        }
        "vtt" => {
            // WebVTT format
            let vtt_content = format!(
                "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n{}\n",
                transcript.content
            );
            (vtt_content, "text/vtt", format!("transcript_{}.vtt", transcript.id))
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported export format: {}. Supported formats: txt, json, srt, vtt",
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

/// Analyze transcript content
async fn analyze_transcript<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<AnalyzeTranscriptRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let transcript = state.repositories.transcript()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get transcript: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Transcript not found".to_string()))?;

    let analysis_results = state.services.analysis()
        .analyze_transcript(
            id,
            &request.analysis_types,
            request.provider.as_deref(),
            request.language.as_deref(),
            request.model.as_deref(),
            request.custom_prompts.as_ref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to analyze transcript: {}", e)))?;

    Ok(Json(serde_json::to_value(analysis_results).unwrap_or_default()))
}

/// Search transcripts
async fn search_transcripts<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<TranscriptListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TranscriptResponse>>>> {
    // Enhanced search functionality
    list_transcripts(State(state), Query(query)).await
}

/// Get transcript statistics
async fn transcript_stats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<TranscriptStatsResponse>> {
    let stats = state.services.transcription()
        .get_transcript_stats()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get transcript stats: {}", e)))?;

    Ok(Json(stats))
}

/// Batch create transcripts
async fn batch_create_transcripts<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchCreateRequest>,
) -> ApiResult<Json<BatchOperationResponse>> {
    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for audio_file_id in &request.audio_file_ids {
        match state.services.transcription()
            .transcribe_audio(
                *audio_file_id,
                request.provider.as_deref(),
                request.language.as_deref(),
                request.auto_detect_language.unwrap_or(true),
                request.model.as_deref(),
                request.prompt.as_deref(),
            )
            .await
        {
            Ok(transcript) => {
                successful += 1;
                results.push(BatchResult {
                    id: Some(transcript.id),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                results.push(BatchResult {
                    id: None,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(BatchOperationResponse {
        total_requested: request.audio_file_ids.len(),
        successful,
        failed,
        results,
    }))
}

/// Batch export transcripts
async fn batch_export_transcripts<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchExportRequest>,
) -> ApiResult<axum::response::Response> {
    // For now, create a ZIP file with all transcripts
    // This would require implementing ZIP creation functionality
    Err(ApiError::NotImplemented("Batch export not yet implemented".to_string()))
}