// src/api/routes/v1/notes.rs
//! Structured notes management API routes
//!
//! This module provides endpoints for managing structured notes from analysis results.

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
    traits::{StructuredNoteRepository, NewStructuredNote, UpdateStructuredNote},
    RepositoryManager,
};
use crate::services::traits::StructuredNoteService;

/// Create structured notes routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(list_notes).post(create_note))
        .route("/:id", get(get_note).patch(update_note).delete(delete_note))
        .route("/:id/export", get(export_note))
        .route("/:id/share", post(share_note))
        .route("/session/:session_id", get(list_session_notes))
        .route("/analysis/:analysis_id", get(list_analysis_notes))
        .route("/batch", post(batch_create_notes).delete(batch_delete_notes))
        .route("/search", get(search_notes))
        .route("/stats", get(notes_stats))
        .route("/templates", get(get_note_templates))
        .route("/tags", get(get_note_tags))
        .route("/merge", post(merge_notes))
        .route("/duplicate", post(find_duplicate_notes))
        .route("/generate", post(generate_note_from_analysis))
}

#[derive(Debug, Deserialize)]
struct CreateNoteRequest {
    session_id: Uuid,
    analysis_id: Option<Uuid>,
    title: String,
    content: serde_json::Value, // Structured content (JSON)
    note_type: String, // "summary", "meeting_notes", "research", "custom"
    template_id: Option<String>,
    tags: Option<Vec<String>>,
    is_public: Option<bool>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateNoteRequest {
    title: Option<String>,
    content: Option<serde_json::Value>,
    note_type: Option<String>,
    template_id: Option<String>,
    tags: Option<Vec<String>>,
    is_public: Option<bool>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct NotesListQuery {
    #[serde(flatten)]
    pagination: PaginationParams,
    #[serde(flatten)]
    search: SearchParams,
    #[serde(flatten)]
    sort: SortParams,
    session_id: Option<Uuid>,
    analysis_id: Option<Uuid>,
    note_type: Option<String>,
    template_id: Option<String>,
    tags: Option<String>, // Comma-separated tags
    is_public: Option<bool>,
    created_after: Option<chrono::DateTime<chrono::Utc>>,
    created_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct BatchCreateNotesRequest {
    notes: Vec<CreateNoteRequest>,
}

#[derive(Debug, Deserialize)]
struct BatchDeleteNotesRequest {
    note_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ShareNoteRequest {
    share_type: String, // "public", "link", "email"
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    password: Option<String>,
    recipients: Option<Vec<String>>, // For email sharing
}

#[derive(Debug, Deserialize)]
struct MergeNotesRequest {
    source_note_ids: Vec<Uuid>,
    target_note: CreateNoteRequest,
    merge_strategy: Option<String>, // "append", "merge_sections", "custom"
    delete_source_notes: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GenerateNoteRequest {
    analysis_id: Uuid,
    template_id: Option<String>,
    note_type: String,
    custom_prompt: Option<String>,
    include_sections: Option<Vec<String>>, // Which sections to include
}

#[derive(Debug, Serialize)]
struct NoteResponse {
    id: Uuid,
    session_id: Uuid,
    analysis_id: Option<Uuid>,
    title: String,
    content: serde_json::Value,
    note_type: String,
    template_id: Option<String>,
    tags: Vec<String>,
    is_public: bool,
    share_token: Option<String>,
    view_count: i64,
    metadata: Option<serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    // Related data
    session_title: Option<String>,
    analysis_type: Option<String>,
    template_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct NotesStatsResponse {
    total_notes: i64,
    note_types: std::collections::HashMap<String, i64>,
    templates: std::collections::HashMap<String, i64>,
    tags: std::collections::HashMap<String, i64>,
    public_notes: i64,
    private_notes: i64,
    total_views: i64,
    avg_content_length: f64,
    notes_per_day: Vec<DailyCount>,
    popular_templates: Vec<TemplateStats>,
}

#[derive(Debug, Serialize)]
struct DailyCount {
    date: chrono::NaiveDate,
    count: i64,
}

#[derive(Debug, Serialize)]
struct TemplateStats {
    template_id: String,
    template_name: String,
    usage_count: i64,
    avg_rating: f64,
}

#[derive(Debug, Serialize)]
struct BatchCreateResponse {
    created: usize,
    failed: usize,
    notes: Vec<NoteResponse>,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BatchDeleteResponse {
    deleted: usize,
    failed: usize,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ShareNoteResponse {
    share_url: String,
    share_token: String,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    share_type: String,
}

#[derive(Debug, Serialize)]
struct MergeNotesResponse {
    merged_note: NoteResponse,
    source_notes_deleted: usize,
}

#[derive(Debug, Serialize)]
struct DuplicateNotesResponse {
    duplicates: Vec<DuplicateGroup>,
}

#[derive(Debug, Serialize)]
struct DuplicateGroup {
    similarity_score: f64,
    notes: Vec<NoteResponse>,
}

#[derive(Debug, Serialize)]
struct TemplatesResponse {
    templates: Vec<NoteTemplate>,
}

#[derive(Debug, Serialize)]
struct NoteTemplate {
    id: String,
    name: String,
    description: String,
    note_type: String,
    schema: serde_json::Value, // JSON schema for the content structure
    default_content: serde_json::Value,
    sections: Vec<TemplateSection>,
    usage_count: i64,
    is_system: bool,
}

#[derive(Debug, Serialize)]
struct TemplateSection {
    id: String,
    name: String,
    description: String,
    required: bool,
    field_type: String, // "text", "markdown", "list", "table", "json"
    default_value: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct TagsResponse {
    tags: Vec<TagInfo>,
}

#[derive(Debug, Serialize)]
struct TagInfo {
    name: String,
    count: i64,
    related_tags: Vec<String>,
}

/// List structured notes with filtering and pagination
async fn list_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<NotesListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<NoteResponse>>>> {
    let tags_filter = query.tags.as_ref().map(|t| {
        t.split(',')
            .map(|tag| tag.trim().to_string())
            .collect::<Vec<_>>()
    });

    let notes = state.repositories.structured_note()
        .find_with_filters(
            query.session_id,
            query.analysis_id,
            query.note_type.as_deref(),
            query.template_id.as_deref(),
            tags_filter.as_ref().map(|v| v.as_slice()),
            query.is_public,
            query.created_after,
            query.created_before,
            query.search.q.as_deref(),
            Some(query.pagination.limit),
            Some(query.pagination.offset),
            query.sort.sort_by.as_deref(),
            query.sort.sort_order.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list notes: {}", e)))?;

    let total = state.repositories.structured_note()
        .count_with_filters(
            query.session_id,
            query.analysis_id,
            query.note_type.as_deref(),
            query.template_id.as_deref(),
            tags_filter.as_ref().map(|v| v.as_slice()),
            query.is_public,
            query.created_after,
            query.created_before,
            query.search.q.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count notes: {}", e)))?;

    // Enrich with related data
    let mut responses = Vec::new();
    for note in notes {
        let session = state.repositories.session()
            .find_by_id(note.session_id)
            .await
            .ok()
            .flatten();
        
        let analysis = if let Some(analysis_id) = note.analysis_id {
            state.repositories.analysis()
                .find_by_id(analysis_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        responses.push(NoteResponse {
            id: note.id,
            session_id: note.session_id,
            analysis_id: note.analysis_id,
            title: note.title,
            content: note.content,
            note_type: note.note_type,
            template_id: note.template_id.clone(),
            tags: note.tags,
            is_public: note.is_public,
            share_token: note.share_token,
            view_count: note.view_count,
            metadata: note.metadata,
            created_at: note.created_at,
            updated_at: note.updated_at,
            session_title: session.and_then(|s| s.title),
            analysis_type: analysis.map(|a| a.analysis_type),
            template_name: note.template_id.clone(), // Would need template lookup for actual name
        });
    }

    Ok(Json(ApiResponse {
        data: responses,
        total: Some(total),
        page: Some(query.pagination.page()),
        per_page: Some(query.pagination.limit),
    }))
}

/// Create a new structured note
async fn create_note<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<CreateNoteRequest>,
) -> ApiResult<Json<ApiResponse<NoteResponse>>> {
    // Validate session exists
    let _session = state.repositories.session()
        .find_by_id(request.session_id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    // Validate analysis exists if provided
    if let Some(analysis_id) = request.analysis_id {
        let _analysis = state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to get analysis: {}", e)))?
            .ok_or_else(|| ApiError::NotFound("Analysis not found".to_string()))?;
    }

    let new_note = NewStructuredNote {
        session_id: request.session_id,
        analysis_id: request.analysis_id,
        title: request.title,
        content: request.content,
        note_type: request.note_type,
        template_id: request.template_id,
        tags: request.tags.unwrap_or_default(),
        is_public: request.is_public.unwrap_or(false),
        metadata: request.metadata,
    };

    let note = state.repositories.structured_note()
        .create(new_note)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create note: {}", e)))?;

    let response = create_note_response(&state, note).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get a specific structured note by ID
async fn get_note<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<NoteResponse>>> {
    let note = state.repositories.structured_note()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get note: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Note not found".to_string()))?;

    // Increment view count
    let _ = state.repositories.structured_note()
        .increment_view_count(id)
        .await;

    let response = create_note_response(&state, note).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Update a structured note
async fn update_note<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateNoteRequest>,
) -> ApiResult<Json<ApiResponse<NoteResponse>>> {
    // Check if note exists
    let _note = state.repositories.structured_note()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get note: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Note not found".to_string()))?;

    let update_note = UpdateStructuredNote {
        title: request.title,
        content: request.content,
        note_type: request.note_type,
        template_id: request.template_id,
        tags: request.tags,
        is_public: request.is_public,
        metadata: request.metadata,
    };

    let updated_note = state.repositories.structured_note()
        .update(id, update_note)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to update note: {}", e)))?;

    let response = create_note_response(&state, updated_note).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Delete a structured note
async fn delete_note<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Check if note exists
    let _note = state.repositories.structured_note()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get note: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Note not found".to_string()))?;

    state.repositories.structured_note()
        .delete(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete note: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Export structured note
async fn export_note<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<axum::response::Response> {
    let note = state.repositories.structured_note()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get note: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Note not found".to_string()))?;

    let format = params.get("format").map(|s| s.as_str()).unwrap_or("json");
    let include_metadata = params.get("include_metadata")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);

    let (content, content_type, filename) = match format {
        "json" => {
            let json_data = if include_metadata {
                serde_json::to_string_pretty(&note)
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize note: {}", e)))?
            } else {
                serde_json::to_string_pretty(&serde_json::json!({
                    "title": note.title,
                    "content": note.content,
                    "note_type": note.note_type,
                    "tags": note.tags
                }))
                .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize note: {}", e)))?
            };
            (json_data, "application/json", format!("note_{}.json", note.id))
        }
        "md" => {
            let markdown_content = convert_note_to_markdown(&note)
                .map_err(|e| ApiError::InternalServerError(format!("Failed to convert to markdown: {}", e)))?;
            (markdown_content, "text/markdown", format!("note_{}.md", note.id))
        }
        "html" => {
            let html_content = convert_note_to_html(&note)
                .map_err(|e| ApiError::InternalServerError(format!("Failed to convert to HTML: {}", e)))?;
            (html_content, "text/html", format!("note_{}.html", note.id))
        }
        "pdf" => {
            // Would need a PDF generation library
            return Err(ApiError::BadRequest("PDF export not yet implemented".to_string()));
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported export format: {}. Supported formats: json, md, html",
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

/// Share a structured note
async fn share_note<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<ShareNoteRequest>,
) -> ApiResult<Json<ShareNoteResponse>> {
    let note = state.repositories.structured_note()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get note: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Note not found".to_string()))?;

    let share_token = uuid::Uuid::new_v4().to_string();
    let base_url = state.config.server.base_url.clone();
    let share_url = format!("{}/shared/notes/{}", base_url, share_token);

    // Update note with share token
    let update_note = UpdateStructuredNote {
        is_public: Some(true),
        ..Default::default()
    };
    
    state.repositories.structured_note()
        .update(id, update_note)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to update note sharing: {}", e)))?;

    // Store share information (would need a shares table)
    // For now, just return the response

    Ok(Json(ShareNoteResponse {
        share_url,
        share_token,
        expires_at: request.expires_at,
        share_type: request.share_type,
    }))
}

/// List notes for a specific session
async fn list_session_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<NotesListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<NoteResponse>>>> {
    let mut modified_query = query;
    modified_query.session_id = Some(session_id);
    list_notes(State(state), Query(modified_query)).await
}

/// List notes for a specific analysis
async fn list_analysis_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(analysis_id): Path<Uuid>,
    Query(query): Query<NotesListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<NoteResponse>>>> {
    let mut modified_query = query;
    modified_query.analysis_id = Some(analysis_id);
    list_notes(State(state), Query(modified_query)).await
}

/// Batch create notes
async fn batch_create_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchCreateNotesRequest>,
) -> ApiResult<Json<BatchCreateResponse>> {
    let mut created_notes = Vec::new();
    let mut errors = Vec::new();
    let mut created_count = 0;
    let mut failed_count = 0;

    for note_request in request.notes {
        match create_note_internal(&state, note_request).await {
            Ok(note) => {
                created_notes.push(note);
                created_count += 1;
            }
            Err(e) => {
                errors.push(e.to_string());
                failed_count += 1;
            }
        }
    }

    Ok(Json(BatchCreateResponse {
        created: created_count,
        failed: failed_count,
        notes: created_notes,
        errors,
    }))
}

/// Batch delete notes
async fn batch_delete_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchDeleteNotesRequest>,
) -> ApiResult<Json<BatchDeleteResponse>> {
    let mut deleted_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for note_id in request.note_ids {
        match state.repositories.structured_note().delete(note_id).await {
            Ok(_) => deleted_count += 1,
            Err(e) => {
                errors.push(format!("Failed to delete note {}: {}", note_id, e));
                failed_count += 1;
            }
        }
    }

    Ok(Json(BatchDeleteResponse {
        deleted: deleted_count,
        failed: failed_count,
        errors,
    }))
}

/// Search notes
async fn search_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<NotesListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<NoteResponse>>>> {
    // Enhanced search functionality
    list_notes(State(state), Query(query)).await
}

/// Get notes statistics
async fn notes_stats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<NotesStatsResponse>> {
    let stats = state.services.structured_note()
        .get_notes_stats()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get notes stats: {}", e)))?;

    Ok(Json(stats))
}

/// Get note templates
async fn get_note_templates<R: RepositoryManager>(
    State(_state): State<AppState<R>>,
) -> ApiResult<Json<TemplatesResponse>> {
    // For now, return predefined templates
    // In a real implementation, these would be stored in the database
    let templates = vec![
        NoteTemplate {
            id: "meeting_notes".to_string(),
            name: "Meeting Notes".to_string(),
            description: "Structured template for meeting notes".to_string(),
            note_type: "meeting_notes".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "meeting_info": {
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "format": "date-time" },
                            "attendees": { "type": "array", "items": { "type": "string" } },
                            "agenda": { "type": "string" }
                        }
                    },
                    "discussion_points": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "topic": { "type": "string" },
                                "discussion": { "type": "string" },
                                "decisions": { "type": "array", "items": { "type": "string" } }
                            }
                        }
                    },
                    "action_items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "task": { "type": "string" },
                                "assignee": { "type": "string" },
                                "due_date": { "type": "string", "format": "date" }
                            }
                        }
                    }
                }
            }),
            default_content: serde_json::json!({
                "meeting_info": {
                    "date": "",
                    "attendees": [],
                    "agenda": ""
                },
                "discussion_points": [],
                "action_items": []
            }),
            sections: vec![
                TemplateSection {
                    id: "meeting_info".to_string(),
                    name: "Meeting Information".to_string(),
                    description: "Basic meeting details".to_string(),
                    required: true,
                    field_type: "json".to_string(),
                    default_value: None,
                },
                TemplateSection {
                    id: "discussion_points".to_string(),
                    name: "Discussion Points".to_string(),
                    description: "Key topics discussed".to_string(),
                    required: false,
                    field_type: "list".to_string(),
                    default_value: None,
                },
                TemplateSection {
                    id: "action_items".to_string(),
                    name: "Action Items".to_string(),
                    description: "Tasks and follow-ups".to_string(),
                    required: false,
                    field_type: "table".to_string(),
                    default_value: None,
                },
            ],
            usage_count: 0,
            is_system: true,
        },
        NoteTemplate {
            id: "research_summary".to_string(),
            name: "Research Summary".to_string(),
            description: "Template for research findings and analysis".to_string(),
            note_type: "research".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "research_topic": { "type": "string" },
                    "methodology": { "type": "string" },
                    "key_findings": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "finding": { "type": "string" },
                                "evidence": { "type": "string" },
                                "significance": { "type": "string" }
                            }
                        }
                    },
                    "conclusions": { "type": "string" },
                    "next_steps": { "type": "array", "items": { "type": "string" } }
                }
            }),
            default_content: serde_json::json!({
                "research_topic": "",
                "methodology": "",
                "key_findings": [],
                "conclusions": "",
                "next_steps": []
            }),
            sections: vec![
                TemplateSection {
                    id: "research_topic".to_string(),
                    name: "Research Topic".to_string(),
                    description: "Main research question or topic".to_string(),
                    required: true,
                    field_type: "text".to_string(),
                    default_value: None,
                },
                TemplateSection {
                    id: "methodology".to_string(),
                    name: "Methodology".to_string(),
                    description: "Research approach and methods".to_string(),
                    required: false,
                    field_type: "markdown".to_string(),
                    default_value: None,
                },
                TemplateSection {
                    id: "key_findings".to_string(),
                    name: "Key Findings".to_string(),
                    description: "Important discoveries and insights".to_string(),
                    required: true,
                    field_type: "list".to_string(),
                    default_value: None,
                },
            ],
            usage_count: 0,
            is_system: true,
        },
    ];

    Ok(Json(TemplatesResponse { templates }))
}

/// Get note tags
async fn get_note_tags<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<TagsResponse>> {
    let tags = state.repositories.structured_note()
        .get_tags()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get tags: {}", e)))?;

    let tags_info: Vec<TagInfo> = tags
        .into_iter()
        .map(|(name, count)| TagInfo {
            name,
            count,
            related_tags: vec![], // Could be enhanced with tag relationships
        })
        .collect();

    Ok(Json(TagsResponse { tags: tags_info }))
}

/// Merge multiple notes into one
async fn merge_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<MergeNotesRequest>,
) -> ApiResult<Json<MergeNotesResponse>> {
    // Validate source notes exist
    let mut source_notes = Vec::new();
    for note_id in &request.source_note_ids {
        let note = state.repositories.structured_note()
            .find_by_id(*note_id)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to get note: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("Note {} not found", note_id)))?;
        source_notes.push(note);
    }

    // Create the merged note
    let merged_note = create_note_internal(&state, request.target_note).await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create merged note: {}", e)))?;

    // Delete source notes if requested
    let mut deleted_count = 0;
    if request.delete_source_notes.unwrap_or(false) {
        for note_id in &request.source_note_ids {
            if let Ok(_) = state.repositories.structured_note().delete(*note_id).await {
                deleted_count += 1;
            }
        }
    }

    Ok(Json(MergeNotesResponse {
        merged_note,
        source_notes_deleted: deleted_count,
    }))
}

/// Find duplicate notes
async fn find_duplicate_notes<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<DuplicateNotesResponse>> {
    let threshold = params.get("threshold")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.8);

    let duplicates = state.services.structured_note()
        .find_duplicates(threshold)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to find duplicates: {}", e)))?;

    Ok(Json(DuplicateNotesResponse { duplicates }))
}

/// Generate note from analysis
async fn generate_note_from_analysis<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<GenerateNoteRequest>,
) -> ApiResult<Json<ApiResponse<NoteResponse>>> {
    let generated_note = state.services.structured_note()
        .generate_from_analysis(
            request.analysis_id,
            &request.note_type,
            request.template_id.as_deref(),
            request.custom_prompt.as_deref(),
            request.include_sections.as_ref().map(|v| v.as_slice()),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to generate note: {}", e)))?;

    let response = create_note_response(&state, generated_note).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

// Helper functions

async fn create_note_response<R: RepositoryManager>(
    state: &AppState<R>,
    note: crate::repository::traits::StructuredNote,
) -> Result<NoteResponse, ApiError> {
    let session = state.repositories.session()
        .find_by_id(note.session_id)
        .await
        .ok()
        .flatten();
    
    let analysis = if let Some(analysis_id) = note.analysis_id {
        state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    Ok(NoteResponse {
        id: note.id,
        session_id: note.session_id,
        analysis_id: note.analysis_id,
        title: note.title,
        content: note.content,
        note_type: note.note_type,
        template_id: note.template_id.clone(),
        tags: note.tags,
        is_public: note.is_public,
        share_token: note.share_token,
        view_count: note.view_count,
        metadata: note.metadata,
        created_at: note.created_at,
        updated_at: note.updated_at,
        session_title: session.and_then(|s| s.title),
        analysis_type: analysis.map(|a| a.analysis_type),
        template_name: note.template_id, // Would need template lookup for actual name
    })
}

async fn create_note_internal<R: RepositoryManager>(
    state: &AppState<R>,
    request: CreateNoteRequest,
) -> Result<NoteResponse, ApiError> {
    // Validate session exists
    let _session = state.repositories.session()
        .find_by_id(request.session_id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get session: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    // Validate analysis exists if provided
    if let Some(analysis_id) = request.analysis_id {
        let _analysis = state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to get analysis: {}", e)))?
            .ok_or_else(|| ApiError::NotFound("Analysis not found".to_string()))?;
    }

    let new_note = NewStructuredNote {
        session_id: request.session_id,
        analysis_id: request.analysis_id,
        title: request.title,
        content: request.content,
        note_type: request.note_type,
        template_id: request.template_id,
        tags: request.tags.unwrap_or_default(),
        is_public: request.is_public.unwrap_or(false),
        metadata: request.metadata,
    };

    let note = state.repositories.structured_note()
        .create(new_note)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create note: {}", e)))?;

    create_note_response(state, note).await
}

fn convert_note_to_markdown(note: &crate::repository::traits::StructuredNote) -> Result<String, Box<dyn std::error::Error>> {
    let mut markdown = format!("# {}\n\n", note.title);
    
    // Add metadata
    markdown.push_str(&format!("**Type:** {}\n", note.note_type));
    if !note.tags.is_empty() {
        markdown.push_str(&format!("**Tags:** {}\n", note.tags.join(", ")));
    }
    markdown.push_str(&format!("**Created:** {}\n\n", note.created_at));
    
    // Convert JSON content to markdown
    if let Ok(content_obj) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(note.content.clone()) {
        for (key, value) in content_obj {
            markdown.push_str(&format!("## {}\n\n", key.replace('_', " ").to_title_case()));
            markdown.push_str(&json_value_to_markdown(&value));
            markdown.push_str("\n\n");
        }
    } else {
        markdown.push_str("## Content\n\n");
        markdown.push_str(&note.content.to_string());
    }
    
    Ok(markdown)
}

fn convert_note_to_html(note: &crate::repository::traits::StructuredNote) -> Result<String, Box<dyn std::error::Error>> {
    let mut html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>{}</title>
    <meta charset="UTF-8">
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .metadata {{ background: #f5f5f5; padding: 10px; border-radius: 5px; margin-bottom: 20px; }}
        .tag {{ background: #e1f5fe; padding: 2px 8px; border-radius: 3px; margin-right: 5px; }}
    </style>
</head>
<body>
    <h1>{}</h1>
"#, note.title, note.title);
    
    // Add metadata
    html.push_str("<div class=\"metadata\">\n");
    html.push_str(&format!("<strong>Type:</strong> {}\n<br>", note.note_type));
    if !note.tags.is_empty() {
        html.push_str("<strong>Tags:</strong> ");
        for tag in &note.tags {
            html.push_str(&format!("<span class=\"tag\">{}</span>", tag));
        }
        html.push_str("\n<br>");
    }
    html.push_str(&format!("<strong>Created:</strong> {}\n", note.created_at));
    html.push_str("</div>\n");
    
    // Convert JSON content to HTML
    if let Ok(content_obj) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(note.content.clone()) {
        for (key, value) in content_obj {
            html.push_str(&format!("<h2>{}</h2>\n", key.replace('_', " ").to_title_case()));
            html.push_str(&json_value_to_html(&value));
        }
    } else {
        html.push_str("<h2>Content</h2>\n");
        html.push_str(&format!("<pre>{}</pre>", note.content.to_string()));
    }
    
    html.push_str("</body>\n</html>");
    Ok(html)
}

fn json_value_to_markdown(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            arr.iter()
                .map(|v| format!("- {}", json_value_to_markdown(v)))
                .collect::<Vec<_>>()
                .join("\n")
        }
        serde_json::Value::Object(obj) => {
            obj.iter()
                .map(|(k, v)| format!("**{}:** {}", k, json_value_to_markdown(v)))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => value.to_string(),
    }
}

fn json_value_to_html(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("<p>{}</p>", s),
        serde_json::Value::Array(arr) => {
            let items = arr.iter()
                .map(|v| format!("<li>{}</li>", json_value_to_html(v)))
                .collect::<Vec<_>>()
                .join("");
            format!("<ul>{}</ul>", items)
        }
        serde_json::Value::Object(obj) => {
            let items = obj.iter()
                .map(|(k, v)| format!("<p><strong>{}:</strong> {}</p>", k, json_value_to_html(v)))
                .collect::<Vec<_>>()
                .join("");
            format!("<div>{}</div>", items)
        }
        _ => format!("<p>{}</p>", value.to_string()),
    }
}

// Helper trait for title case conversion
trait ToTitleCase {
    fn to_title_case(&self) -> String;
}

impl ToTitleCase for str {
    fn to_title_case(&self) -> String {
        self.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}