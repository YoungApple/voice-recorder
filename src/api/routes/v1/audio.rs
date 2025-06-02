// src/api/routes/v1/audio.rs
//! Audio file management API routes
//!
//! This module provides endpoints for uploading, managing, and serving audio files.

use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
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
    traits::{AudioRepository, NewAudioFile},
    RepositoryManager,
};
use crate::services::traits::{AudioService, FileStorageService};

/// Create audio routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(list_audio_files).post(upload_audio))
        .route("/:id", get(get_audio_file).delete(delete_audio_file))
        .route("/:id/download", get(download_audio_file))
        .route("/:id/stream", get(stream_audio_file))
        .route("/:id/metadata", get(get_audio_metadata))
        .route("/:id/transcribe", post(transcribe_audio))
        .route("/upload/chunk", post(upload_audio_chunk))
        .route("/upload/complete", post(complete_chunked_upload))
        .route("/formats", get(get_supported_formats))
        .route("/stats", get(get_audio_stats))
}

#[derive(Debug, Deserialize)]
struct AudioListQuery {
    #[serde(flatten)]
    pagination: PaginationParams,
    #[serde(flatten)]
    search: SearchParams,
    #[serde(flatten)]
    sort: SortParams,
    session_id: Option<Uuid>,
    format: Option<String>,
    min_duration: Option<f64>,
    max_duration: Option<f64>,
    uploaded_after: Option<chrono::DateTime<chrono::Utc>>,
    uploaded_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
struct AudioFileResponse {
    id: Uuid,
    session_id: Uuid,
    filename: String,
    original_filename: String,
    file_path: String,
    file_size: i64,
    duration_seconds: Option<f64>,
    format: String,
    sample_rate: Option<i32>,
    channels: Option<i32>,
    bitrate: Option<i32>,
    metadata: Option<serde_json::Value>,
    checksum: Option<String>,
    upload_status: String,
    transcription_status: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct TranscribeRequest {
    provider: Option<String>, // "openai" or "local"
    language: Option<String>,
    auto_detect_language: Option<bool>,
    model: Option<String>,
    prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChunkUploadRequest {
    upload_id: String,
    chunk_number: i32,
    total_chunks: i32,
    filename: String,
    session_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct CompleteUploadRequest {
    upload_id: String,
    filename: String,
    session_id: Option<Uuid>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct AudioStatsResponse {
    total_files: i64,
    total_size_mb: f64,
    total_duration_hours: f64,
    formats: std::collections::HashMap<String, i64>,
    avg_file_size_mb: f64,
    avg_duration_minutes: f64,
    upload_success_rate: f64,
    transcription_completion_rate: f64,
}

#[derive(Debug, Serialize)]
struct SupportedFormatsResponse {
    audio_formats: Vec<String>,
    max_file_size_mb: u64,
    max_duration_hours: f64,
    supported_sample_rates: Vec<i32>,
    supported_channels: Vec<i32>,
}

/// List audio files with filtering and pagination
async fn list_audio_files<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<AudioListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<AudioFileResponse>>>> {
    let audio_files = state.repositories.audio()
        .find_with_filters(
            query.session_id,
            query.format.as_deref(),
            query.min_duration,
            query.max_duration,
            query.uploaded_after,
            query.uploaded_before,
            Some(query.pagination.limit),
            Some(query.pagination.offset),
            query.sort.sort_by.as_deref(),
            query.sort.sort_order.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list audio files: {}", e)))?;

    let total = state.repositories.audio()
        .count_with_filters(
            query.session_id,
            query.format.as_deref(),
            query.min_duration,
            query.max_duration,
            query.uploaded_after,
            query.uploaded_before,
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count audio files: {}", e)))?;

    let responses: Vec<AudioFileResponse> = audio_files
        .into_iter()
        .map(|audio| AudioFileResponse {
            id: audio.id,
            session_id: audio.session_id,
            filename: audio.filename,
            original_filename: audio.original_filename,
            file_path: audio.file_path,
            file_size: audio.file_size,
            duration_seconds: audio.duration_seconds,
            format: audio.format,
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            bitrate: audio.bitrate,
            metadata: audio.metadata,
            checksum: audio.checksum,
            upload_status: audio.upload_status,
            transcription_status: audio.transcription_status,
            created_at: audio.created_at,
            updated_at: audio.updated_at,
        })
        .collect();

    Ok(Json(ApiResponse {
        data: responses,
        total: Some(total),
        page: Some(query.pagination.page()),
        per_page: Some(query.pagination.limit),
    }))
}

/// Upload an audio file
async fn upload_audio<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    mut multipart: Multipart,
) -> ApiResult<Json<ApiResponse<AudioFileResponse>>> {
    let mut session_id: Option<Uuid> = None;
    let mut metadata: Option<serde_json::Value> = None;
    let mut file_data: Option<(String, Bytes)> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await
        .map_err(|e| ApiError::BadRequest(format!("Invalid multipart data: {}", e)))? {
        
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "session_id" => {
                let value = field.text().await
                    .map_err(|e| ApiError::BadRequest(format!("Invalid session_id: {}", e)))?;
                session_id = Some(Uuid::parse_str(&value)
                    .map_err(|e| ApiError::BadRequest(format!("Invalid session_id format: {}", e)))?);
            }
            "metadata" => {
                let value = field.text().await
                    .map_err(|e| ApiError::BadRequest(format!("Invalid metadata: {}", e)))?;
                metadata = Some(serde_json::from_str(&value)
                    .map_err(|e| ApiError::BadRequest(format!("Invalid metadata JSON: {}", e)))?);
            }
            "file" => {
                let filename = field.file_name()
                    .ok_or_else(|| ApiError::BadRequest("Missing filename".to_string()))?
                    .to_string();
                let data = field.bytes().await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file data: {}", e)))?;
                file_data = Some((filename, data));
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    let (filename, data) = file_data
        .ok_or_else(|| ApiError::BadRequest("Missing file data".to_string()))?;

    // Validate file format
    let format = std::path::Path::new(&filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| ApiError::BadRequest("Unable to determine file format".to_string()))?
        .to_lowercase();

    if !state.config.storage.allowed_formats.contains(&format) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file format: {}. Allowed formats: {:?}",
            format, state.config.storage.allowed_formats
        )));
    }

    // Validate file size
    if data.len() as u64 > state.config.storage.max_file_size {
        return Err(ApiError::BadRequest(format!(
            "File size ({} bytes) exceeds maximum allowed size ({} bytes)",
            data.len(),
            state.config.storage.max_file_size
        )));
    }

    // Create session if not provided
    let session_id = match session_id {
        Some(id) => {
            // Verify session exists
            state.repositories.session()
                .find_by_id(id)
                .await
                .map_err(|e| ApiError::InternalServerError(format!("Failed to verify session: {}", e)))?
                .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;
            id
        }
        None => {
            // Create a new session
            let new_session = crate::repository::traits::NewSession {
                title: Some(format!("Auto-created for {}", filename)),
                description: Some("Automatically created session for audio upload".to_string()),
                tags: vec!["auto-created".to_string()],
                metadata: None,
            };
            
            let session = state.services.session()
                .create_session(new_session)
                .await
                .map_err(|e| ApiError::InternalServerError(format!("Failed to create session: {}", e)))?;
            
            session.id
        }
    };

    // Upload the audio file
    let audio_file = state.services.audio()
        .upload_audio(session_id, filename, data.to_vec(), metadata)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to upload audio: {}", e)))?;

    let response = AudioFileResponse {
        id: audio_file.id,
        session_id: audio_file.session_id,
        filename: audio_file.filename,
        original_filename: audio_file.original_filename,
        file_path: audio_file.file_path,
        file_size: audio_file.file_size,
        duration_seconds: audio_file.duration_seconds,
        format: audio_file.format,
        sample_rate: audio_file.sample_rate,
        channels: audio_file.channels,
        bitrate: audio_file.bitrate,
        metadata: audio_file.metadata,
        checksum: audio_file.checksum,
        upload_status: audio_file.upload_status,
        transcription_status: audio_file.transcription_status,
        created_at: audio_file.created_at,
        updated_at: audio_file.updated_at,
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get a specific audio file by ID
async fn get_audio_file<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<AudioFileResponse>>> {
    let audio_file = state.repositories.audio()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio file: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Audio file not found".to_string()))?;

    let response = AudioFileResponse {
        id: audio_file.id,
        session_id: audio_file.session_id,
        filename: audio_file.filename,
        original_filename: audio_file.original_filename,
        file_path: audio_file.file_path,
        file_size: audio_file.file_size,
        duration_seconds: audio_file.duration_seconds,
        format: audio_file.format,
        sample_rate: audio_file.sample_rate,
        channels: audio_file.channels,
        bitrate: audio_file.bitrate,
        metadata: audio_file.metadata,
        checksum: audio_file.checksum,
        upload_status: audio_file.upload_status,
        transcription_status: audio_file.transcription_status,
        created_at: audio_file.created_at,
        updated_at: audio_file.updated_at,
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Delete an audio file
async fn delete_audio_file<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Check if audio file exists
    let _audio_file = state.repositories.audio()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio file: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Audio file not found".to_string()))?;

    state.services.audio()
        .delete_audio(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete audio file: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Download an audio file
async fn download_audio_file<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let audio_file = state.repositories.audio()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio file: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Audio file not found".to_string()))?;

    let file_data = state.services.file_storage()
        .read_file(&audio_file.file_path)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to read audio file: {}", e)))?;

    let content_type = match audio_file.format.as_str() {
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", audio_file.original_filename),
            ),
            (header::CONTENT_LENGTH, &file_data.len().to_string()),
        ],
        file_data,
    )
        .into_response())
}

/// Stream an audio file
async fn stream_audio_file<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let audio_file = state.repositories.audio()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio file: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Audio file not found".to_string()))?;

    let file_data = state.services.file_storage()
        .read_file(&audio_file.file_path)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to read audio file: {}", e)))?;

    let content_type = match audio_file.format.as_str() {
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::ACCEPT_RANGES, "bytes"),
            (header::CONTENT_LENGTH, &file_data.len().to_string()),
        ],
        file_data,
    )
        .into_response())
}

/// Get audio file metadata
async fn get_audio_metadata<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let audio_file = state.repositories.audio()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio file: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Audio file not found".to_string()))?;

    let metadata = state.services.audio()
        .get_audio_metadata(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio metadata: {}", e)))?;

    Ok(Json(metadata))
}

/// Transcribe an audio file
async fn transcribe_audio<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<TranscribeRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check if audio file exists
    let _audio_file = state.repositories.audio()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio file: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Audio file not found".to_string()))?;

    let transcript = state.services.transcription()
        .transcribe_audio(
            id,
            request.provider.as_deref(),
            request.language.as_deref(),
            request.auto_detect_language.unwrap_or(true),
            request.model.as_deref(),
            request.prompt.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to transcribe audio: {}", e)))?;

    Ok(Json(serde_json::to_value(transcript).unwrap_or_default()))
}

/// Upload audio chunk (for large file uploads)
async fn upload_audio_chunk<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    mut multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    // Implementation for chunked upload
    // This would handle large file uploads by splitting them into chunks
    
    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "message": "Chunk upload not yet implemented",
        "status": "pending"
    })))
}

/// Complete chunked upload
async fn complete_chunked_upload<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<CompleteUploadRequest>,
) -> ApiResult<Json<ApiResponse<AudioFileResponse>>> {
    // Implementation for completing chunked upload
    // This would combine all uploaded chunks into a single file
    
    // For now, return an error
    Err(ApiError::NotImplemented("Chunked upload not yet implemented".to_string()))
}

/// Get supported audio formats
async fn get_supported_formats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<SupportedFormatsResponse>> {
    Ok(Json(SupportedFormatsResponse {
        audio_formats: state.config.storage.allowed_formats.clone(),
        max_file_size_mb: state.config.storage.max_file_size / (1024 * 1024),
        max_duration_hours: 24.0, // TODO: Make this configurable
        supported_sample_rates: vec![8000, 16000, 22050, 44100, 48000, 96000],
        supported_channels: vec![1, 2],
    }))
}

/// Get audio statistics
async fn get_audio_stats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<AudioStatsResponse>> {
    let stats = state.services.audio()
        .get_audio_stats()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get audio stats: {}", e)))?;

    Ok(Json(stats))
}