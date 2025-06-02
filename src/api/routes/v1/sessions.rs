// src/api/routes/v1/sessions.rs
//! Session management API routes
//!
//! This module provides endpoints for managing voice recording sessions.

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
    traits::{NewSession, SessionRepository, UpdateSession},
    RepositoryManager,
};
use crate::services::traits::SessionService;

/// Create session routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(list_sessions).post(create_session))
        .route("/:id", get(get_session).patch(update_session).delete(delete_session))
        .route("/:id/audio", get(list_session_audio))
        .route("/:id/transcripts", get(list_session_transcripts))
        .route("/:id/analysis", get(list_session_analysis))
        .route("/:id/export", get(export_session))
        .route("/search", get(search_sessions))
        .route("/stats", get(session_stats))
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    title: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateSessionRequest {
    title: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
    metadata: Option<serde_json::Value>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionListQuery {
    #[serde(flatten)]
    pagination: PaginationParams,
    #[serde(flatten)]
    search: SearchParams,
    #[serde(flatten)]
    sort: SortParams,
    status: Option<String>,
    tags: Option<String>, // Comma-separated tags
    created_after: Option<chrono::DateTime<chrono::Utc>>,
    created_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    id: Uuid,
    title: Option<String>,
    description: Option<String>,
    status: String,
    tags: Vec<String>,
    metadata: Option<serde_json::Value>,
    audio_count: i64,
    transcript_count: i64,
    analysis_count: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
struct SessionStatsResponse {
    total_sessions: i64,
    active_sessions: i64,
    completed_sessions: i64,
    archived_sessions: i64,
    total_audio_files: i64,
    total_transcripts: i64,
    total_analysis_results: i64,
    storage_used_mb: f64,
    avg_session_duration_minutes: f64,
}

/// List all sessions with filtering and pagination
async fn list_sessions<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<SessionListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<SessionResponse>>>> {
    let sessions = state.services.session()
        .list_sessions(
            query.pagination.limit,
            query.pagination.offset,
            query.search.q.as_deref(),
            query.status.as_deref(),
            query.tags.as_deref().map(|t| t.split(',').map(|s| s.trim().to_string()).collect()),
            query.created_after,
            query.created_before,
            query.sort.sort_by.as_deref(),
            query.sort.sort_order.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list sessions: {}", e)))?;

    let total = state.repositories.session()
        .count_sessions()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count sessions: {}", e)))?;

    let session_responses = futures::future::try_join_all(
        sessions.into_iter().map(|session| async {
            let audio_count = state.repositories.audio()
                .count_by_session(session.id)
                .await
                .unwrap_or(0);
            
            let transcript_count = state.repositories.transcript()
                .count_by_session(session.id)
                .await
                .unwrap_or(0);
            
            let analysis_count = state.repositories.analysis()
                .count_by_session(session.id)
                .await
                .unwrap_or(0);

            Ok::<SessionResponse, ApiError>(SessionResponse {
                id: session.id,
                title: session.title,
                description: session.description,
                status: session.status,
                tags: session.tags,
                metadata: session.metadata,
                audio_count,
                transcript_count,
                analysis_count,
                created_at: session.created_at,
                updated_at: session.updated_at,
            })
        })
    ).await?;

    Ok(Json(ApiResponse {
        data: session_responses,
        total: Some(total),
        page: Some(query.pagination.page()),
        per_page: Some(query.pagination.limit),
    }))
}

/// Create a new session
async fn create_session<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<Json<ApiResponse<SessionResponse>>> {
    let new_session = NewSession {
        title: request.title,
        description: request.description,
        tags: request.tags.unwrap_or_default(),
        metadata: request.metadata,
    };

    let session = state.services.session()
        .create_session(new_session)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create session: {}", e)))?;

    let response = SessionResponse {
        id: session.id,
        title: session.title,
        description: session.description,
        status: session.status,
        tags: session.tags,
        metadata: session.metadata,
        audio_count: 0,
        transcript_count: 0,
        analysis_count: 0,
        created_at: session.created_at,
        updated_at: session.updated_at,
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get a specific session by ID
async fn get_session<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<SessionResponse>>> {
    let session = state.repositories.session()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    let audio_count = state.repositories.audio()
        .count_by_session(session.id)
        .await
        .unwrap_or(0);
    
    let transcript_count = state.repositories.transcript()
        .count_by_session(session.id)
        .await
        .unwrap_or(0);
    
    let analysis_count = state.repositories.analysis()
        .count_by_session(session.id)
        .await
        .unwrap_or(0);

    let response = SessionResponse {
        id: session.id,
        title: session.title,
        description: session.description,
        status: session.status,
        tags: session.tags,
        metadata: session.metadata,
        audio_count,
        transcript_count,
        analysis_count,
        created_at: session.created_at,
        updated_at: session.updated_at,
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Update a session
async fn update_session<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateSessionRequest>,
) -> ApiResult<Json<ApiResponse<SessionResponse>>> {
    // Check if session exists
    let _session = state.repositories.session()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    let update_session = UpdateSession {
        title: request.title,
        description: request.description,
        status: request.status,
        tags: request.tags,
        metadata: request.metadata,
    };

    let updated_session = state.services.session()
        .update_session(id, update_session)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to update session: {}", e)))?;

    let audio_count = state.repositories.audio()
        .count_by_session(updated_session.id)
        .await
        .unwrap_or(0);
    
    let transcript_count = state.repositories.transcript()
        .count_by_session(updated_session.id)
        .await
        .unwrap_or(0);
    
    let analysis_count = state.repositories.analysis()
        .count_by_session(updated_session.id)
        .await
        .unwrap_or(0);

    let response = SessionResponse {
        id: updated_session.id,
        title: updated_session.title,
        description: updated_session.description,
        status: updated_session.status,
        tags: updated_session.tags,
        metadata: updated_session.metadata,
        audio_count,
        transcript_count,
        analysis_count,
        created_at: updated_session.created_at,
        updated_at: updated_session.updated_at,
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Delete a session
async fn delete_session<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Check if session exists
    let _session = state.repositories.session()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    state.services.session()
        .delete_session(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete session: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// List audio files for a session
async fn list_session_audio<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    // Check if session exists
    let _session = state.repositories.session()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    let audio_files = state.repositories.audio()
        .find_by_session(id, Some(pagination.limit), Some(pagination.offset))
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list audio files: {}", e)))?;

    let total = state.repositories.audio()
        .count_by_session(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count audio files: {}", e)))?;

    // Convert to JSON values for now (will be replaced with proper response types)
    let audio_responses: Vec<serde_json::Value> = audio_files
        .into_iter()
        .map(|audio| serde_json::to_value(audio).unwrap_or_default())
        .collect();

    Ok(Json(ApiResponse {
        data: audio_responses,
        total: Some(total),
        page: Some(pagination.page()),
        per_page: Some(pagination.limit),
    }))
}

/// List transcripts for a session
async fn list_session_transcripts<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    // Check if session exists
    let _session = state.repositories.session()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    let transcripts = state.repositories.transcript()
        .find_by_session(id, Some(pagination.limit), Some(pagination.offset))
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list transcripts: {}", e)))?;

    let total = state.repositories.transcript()
        .count_by_session(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count transcripts: {}", e)))?;

    // Convert to JSON values for now (will be replaced with proper response types)
    let transcript_responses: Vec<serde_json::Value> = transcripts
        .into_iter()
        .map(|transcript| serde_json::to_value(transcript).unwrap_or_default())
        .collect();

    Ok(Json(ApiResponse {
        data: transcript_responses,
        total: Some(total),
        page: Some(pagination.page()),
        per_page: Some(pagination.limit),
    }))
}

/// List analysis results for a session
async fn list_session_analysis<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    // Check if session exists
    let _session = state.repositories.session()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    let analysis_results = state.repositories.analysis()
        .find_by_session(id, Some(pagination.limit), Some(pagination.offset))
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list analysis results: {}", e)))?;

    let total = state.repositories.analysis()
        .count_by_session(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count analysis results: {}", e)))?;

    // Convert to JSON values for now (will be replaced with proper response types)
    let analysis_responses: Vec<serde_json::Value> = analysis_results
        .into_iter()
        .map(|analysis| serde_json::to_value(analysis).unwrap_or_default())
        .collect();

    Ok(Json(ApiResponse {
        data: analysis_responses,
        total: Some(total),
        page: Some(pagination.page()),
        per_page: Some(pagination.limit),
    }))
}

/// Export session data
async fn export_session<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let export_data = state.services.session()
        .export_session(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to export session: {}", e)))?;

    Ok(Json(export_data))
}

/// Search sessions
async fn search_sessions<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<SessionListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<SessionResponse>>>> {
    // This is similar to list_sessions but with enhanced search capabilities
    list_sessions(State(state), Query(query)).await
}

/// Get session statistics
async fn session_stats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<SessionStatsResponse>> {
    let stats = state.services.session()
        .get_session_stats()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session stats: {}", e)))?;

    Ok(Json(stats))
}