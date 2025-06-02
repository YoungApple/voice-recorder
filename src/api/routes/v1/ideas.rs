// src/api/routes/v1/ideas.rs
//! Ideas management API routes
//!
//! This module provides endpoints for managing extracted ideas from analysis results.

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
    traits::{IdeaRepository, NewIdea, UpdateIdea},
    RepositoryManager,
};
use crate::services::traits::IdeaService;

/// Create ideas routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(list_ideas).post(create_idea))
        .route("/:id", get(get_idea).patch(update_idea).delete(delete_idea))
        .route("/:id/export", get(export_idea))
        .route("/session/:session_id", get(list_session_ideas))
        .route("/analysis/:analysis_id", get(list_analysis_ideas))
        .route("/batch", post(batch_create_ideas).delete(batch_delete_ideas))
        .route("/search", get(search_ideas))
        .route("/stats", get(ideas_stats))
        .route("/categories", get(get_idea_categories))
        .route("/tags", get(get_idea_tags))
        .route("/merge", post(merge_ideas))
        .route("/duplicate", post(find_duplicate_ideas))
}

#[derive(Debug, Deserialize)]
struct CreateIdeaRequest {
    session_id: Uuid,
    analysis_id: Option<Uuid>,
    title: String,
    description: Option<String>,
    category: Option<String>,
    priority: Option<String>, // "low", "medium", "high", "critical"
    status: Option<String>, // "new", "in_progress", "completed", "archived"
    tags: Option<Vec<String>>,
    source_text: Option<String>,
    confidence_score: Option<f64>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateIdeaRequest {
    title: Option<String>,
    description: Option<String>,
    category: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    tags: Option<Vec<String>>,
    source_text: Option<String>,
    confidence_score: Option<f64>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct IdeasListQuery {
    #[serde(flatten)]
    pagination: PaginationParams,
    #[serde(flatten)]
    search: SearchParams,
    #[serde(flatten)]
    sort: SortParams,
    session_id: Option<Uuid>,
    analysis_id: Option<Uuid>,
    category: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    tags: Option<String>, // Comma-separated tags
    min_confidence: Option<f64>,
    created_after: Option<chrono::DateTime<chrono::Utc>>,
    created_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct BatchCreateIdeasRequest {
    ideas: Vec<CreateIdeaRequest>,
}

#[derive(Debug, Deserialize)]
struct BatchDeleteIdeasRequest {
    idea_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
struct MergeIdeasRequest {
    source_idea_ids: Vec<Uuid>,
    target_idea: CreateIdeaRequest,
    delete_source_ideas: Option<bool>,
}

#[derive(Debug, Serialize)]
struct IdeaResponse {
    id: Uuid,
    session_id: Uuid,
    analysis_id: Option<Uuid>,
    title: String,
    description: Option<String>,
    category: Option<String>,
    priority: String,
    status: String,
    tags: Vec<String>,
    source_text: Option<String>,
    confidence_score: Option<f64>,
    metadata: Option<serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    // Related data
    session_title: Option<String>,
    analysis_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct IdeasStatsResponse {
    total_ideas: i64,
    categories: std::collections::HashMap<String, i64>,
    priorities: std::collections::HashMap<String, i64>,
    statuses: std::collections::HashMap<String, i64>,
    tags: std::collections::HashMap<String, i64>,
    avg_confidence_score: f64,
    ideas_per_day: Vec<DailyCount>,
    completion_rate: f64,
    top_categories: Vec<CategoryStats>,
}

#[derive(Debug, Serialize)]
struct DailyCount {
    date: chrono::NaiveDate,
    count: i64,
}

#[derive(Debug, Serialize)]
struct CategoryStats {
    category: String,
    count: i64,
    completion_rate: f64,
    avg_confidence: f64,
}

#[derive(Debug, Serialize)]
struct BatchCreateResponse {
    created: usize,
    failed: usize,
    ideas: Vec<IdeaResponse>,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BatchDeleteResponse {
    deleted: usize,
    failed: usize,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct MergeIdeasResponse {
    merged_idea: IdeaResponse,
    source_ideas_deleted: usize,
}

#[derive(Debug, Serialize)]
struct DuplicateIdeasResponse {
    duplicates: Vec<DuplicateGroup>,
}

#[derive(Debug, Serialize)]
struct DuplicateGroup {
    similarity_score: f64,
    ideas: Vec<IdeaResponse>,
}

#[derive(Debug, Serialize)]
struct CategoriesResponse {
    categories: Vec<CategoryInfo>,
}

#[derive(Debug, Serialize)]
struct CategoryInfo {
    name: String,
    count: i64,
    description: Option<String>,
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

/// List ideas with filtering and pagination
async fn list_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<IdeasListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<IdeaResponse>>>> {
    let tags_filter = query.tags.as_ref().map(|t| {
        t.split(',')
            .map(|tag| tag.trim().to_string())
            .collect::<Vec<_>>()
    });

    let ideas = state.repositories.idea()
        .find_with_filters(
            query.session_id,
            query.analysis_id,
            query.category.as_deref(),
            query.priority.as_deref(),
            query.status.as_deref(),
            tags_filter.as_ref().map(|v| v.as_slice()),
            query.min_confidence,
            query.created_after,
            query.created_before,
            query.search.q.as_deref(),
            Some(query.pagination.limit),
            Some(query.pagination.offset),
            query.sort.sort_by.as_deref(),
            query.sort.sort_order.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list ideas: {}", e)))?;

    let total = state.repositories.idea()
        .count_with_filters(
            query.session_id,
            query.analysis_id,
            query.category.as_deref(),
            query.priority.as_deref(),
            query.status.as_deref(),
            tags_filter.as_ref().map(|v| v.as_slice()),
            query.min_confidence,
            query.created_after,
            query.created_before,
            query.search.q.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count ideas: {}", e)))?;

    // Enrich with related data
    let mut responses = Vec::new();
    for idea in ideas {
        let session = state.repositories.session()
            .find_by_id(idea.session_id)
            .await
            .ok()
            .flatten();
        
        let analysis = if let Some(analysis_id) = idea.analysis_id {
            state.repositories.analysis()
                .find_by_id(analysis_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        responses.push(IdeaResponse {
            id: idea.id,
            session_id: idea.session_id,
            analysis_id: idea.analysis_id,
            title: idea.title,
            description: idea.description,
            category: idea.category,
            priority: idea.priority,
            status: idea.status,
            tags: idea.tags,
            source_text: idea.source_text,
            confidence_score: idea.confidence_score,
            metadata: idea.metadata,
            created_at: idea.created_at,
            updated_at: idea.updated_at,
            session_title: session.and_then(|s| s.title),
            analysis_type: analysis.map(|a| a.analysis_type),
        });
    }

    Ok(Json(ApiResponse {
        data: responses,
        total: Some(total),
        page: Some(query.pagination.page()),
        per_page: Some(query.pagination.limit),
    }))
}

/// Create a new idea
async fn create_idea<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<CreateIdeaRequest>,
) -> ApiResult<Json<ApiResponse<IdeaResponse>>> {
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

    let new_idea = NewIdea {
        session_id: request.session_id,
        analysis_id: request.analysis_id,
        title: request.title,
        description: request.description,
        category: request.category,
        priority: request.priority.unwrap_or_else(|| "medium".to_string()),
        status: request.status.unwrap_or_else(|| "new".to_string()),
        tags: request.tags.unwrap_or_default(),
        source_text: request.source_text,
        confidence_score: request.confidence_score,
        metadata: request.metadata,
    };

    let idea = state.repositories.idea()
        .create(new_idea)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create idea: {}", e)))?;

    let session = state.repositories.session()
        .find_by_id(idea.session_id)
        .await
        .ok()
        .flatten();
    
    let analysis = if let Some(analysis_id) = idea.analysis_id {
        state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let response = IdeaResponse {
        id: idea.id,
        session_id: idea.session_id,
        analysis_id: idea.analysis_id,
        title: idea.title,
        description: idea.description,
        category: idea.category,
        priority: idea.priority,
        status: idea.status,
        tags: idea.tags,
        source_text: idea.source_text,
        confidence_score: idea.confidence_score,
        metadata: idea.metadata,
        created_at: idea.created_at,
        updated_at: idea.updated_at,
        session_title: session.and_then(|s| s.title),
        analysis_type: analysis.map(|a| a.analysis_type),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get a specific idea by ID
async fn get_idea<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<IdeaResponse>>> {
    let idea = state.repositories.idea()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get idea: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Idea not found".to_string()))?;

    let session = state.repositories.session()
        .find_by_id(idea.session_id)
        .await
        .ok()
        .flatten();
    
    let analysis = if let Some(analysis_id) = idea.analysis_id {
        state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let response = IdeaResponse {
        id: idea.id,
        session_id: idea.session_id,
        analysis_id: idea.analysis_id,
        title: idea.title,
        description: idea.description,
        category: idea.category,
        priority: idea.priority,
        status: idea.status,
        tags: idea.tags,
        source_text: idea.source_text,
        confidence_score: idea.confidence_score,
        metadata: idea.metadata,
        created_at: idea.created_at,
        updated_at: idea.updated_at,
        session_title: session.and_then(|s| s.title),
        analysis_type: analysis.map(|a| a.analysis_type),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Update an idea
async fn update_idea<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateIdeaRequest>,
) -> ApiResult<Json<ApiResponse<IdeaResponse>>> {
    // Check if idea exists
    let _idea = state.repositories.idea()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get idea: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Idea not found".to_string()))?;

    let update_idea = UpdateIdea {
        title: request.title,
        description: request.description,
        category: request.category,
        priority: request.priority,
        status: request.status,
        tags: request.tags,
        source_text: request.source_text,
        confidence_score: request.confidence_score,
        metadata: request.metadata,
    };

    let updated_idea = state.repositories.idea()
        .update(id, update_idea)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to update idea: {}", e)))?;

    let session = state.repositories.session()
        .find_by_id(updated_idea.session_id)
        .await
        .ok()
        .flatten();
    
    let analysis = if let Some(analysis_id) = updated_idea.analysis_id {
        state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let response = IdeaResponse {
        id: updated_idea.id,
        session_id: updated_idea.session_id,
        analysis_id: updated_idea.analysis_id,
        title: updated_idea.title,
        description: updated_idea.description,
        category: updated_idea.category,
        priority: updated_idea.priority,
        status: updated_idea.status,
        tags: updated_idea.tags,
        source_text: updated_idea.source_text,
        confidence_score: updated_idea.confidence_score,
        metadata: updated_idea.metadata,
        created_at: updated_idea.created_at,
        updated_at: updated_idea.updated_at,
        session_title: session.and_then(|s| s.title),
        analysis_type: analysis.map(|a| a.analysis_type),
    };

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Delete an idea
async fn delete_idea<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Check if idea exists
    let _idea = state.repositories.idea()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get idea: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Idea not found".to_string()))?;

    state.repositories.idea()
        .delete(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete idea: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Export idea
async fn export_idea<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<axum::response::Response> {
    let idea = state.repositories.idea()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get idea: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Idea not found".to_string()))?;

    let format = params.get("format").map(|s| s.as_str()).unwrap_or("json");
    let include_metadata = params.get("include_metadata")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);

    let (content, content_type, filename) = match format {
        "json" => {
            let json_data = if include_metadata {
                serde_json::to_string_pretty(&idea)
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize idea: {}", e)))?
            } else {
                serde_json::to_string_pretty(&serde_json::json!({
                    "title": idea.title,
                    "description": idea.description,
                    "category": idea.category,
                    "priority": idea.priority,
                    "status": idea.status,
                    "tags": idea.tags
                }))
                .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize idea: {}", e)))?
            };
            (json_data, "application/json", format!("idea_{}.json", idea.id))
        }
        "txt" => {
            let text_content = format!(
                "Title: {}\nCategory: {}\nPriority: {}\nStatus: {}\nTags: {}\nCreated: {}\n\nDescription:\n{}\n\nSource Text:\n{}\n",
                idea.title,
                idea.category.unwrap_or_else(|| "Uncategorized".to_string()),
                idea.priority,
                idea.status,
                idea.tags.join(", "),
                idea.created_at,
                idea.description.unwrap_or_else(|| "No description".to_string()),
                idea.source_text.unwrap_or_else(|| "No source text".to_string())
            );
            (text_content, "text/plain", format!("idea_{}.txt", idea.id))
        }
        "md" => {
            let markdown_content = format!(
                "# {}\n\n**Category:** {}\n**Priority:** {}\n**Status:** {}\n**Tags:** {}\n**Created:** {}\n\n## Description\n\n{}\n\n## Source Text\n\n{}\n",
                idea.title,
                idea.category.unwrap_or_else(|| "Uncategorized".to_string()),
                idea.priority,
                idea.status,
                idea.tags.join(", "),
                idea.created_at,
                idea.description.unwrap_or_else(|| "No description".to_string()),
                idea.source_text.unwrap_or_else(|| "No source text".to_string())
            );
            (markdown_content, "text/markdown", format!("idea_{}.md", idea.id))
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported export format: {}. Supported formats: json, txt, md",
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

/// List ideas for a specific session
async fn list_session_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<IdeasListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<IdeaResponse>>>> {
    let mut modified_query = query;
    modified_query.session_id = Some(session_id);
    list_ideas(State(state), Query(modified_query)).await
}

/// List ideas for a specific analysis
async fn list_analysis_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(analysis_id): Path<Uuid>,
    Query(query): Query<IdeasListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<IdeaResponse>>>> {
    let mut modified_query = query;
    modified_query.analysis_id = Some(analysis_id);
    list_ideas(State(state), Query(modified_query)).await
}

/// Batch create ideas
async fn batch_create_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchCreateIdeasRequest>,
) -> ApiResult<Json<BatchCreateResponse>> {
    let mut created_ideas = Vec::new();
    let mut errors = Vec::new();
    let mut created_count = 0;
    let mut failed_count = 0;

    for idea_request in request.ideas {
        match create_idea_internal(&state, idea_request).await {
            Ok(idea) => {
                created_ideas.push(idea);
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
        ideas: created_ideas,
        errors,
    }))
}

/// Batch delete ideas
async fn batch_delete_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchDeleteIdeasRequest>,
) -> ApiResult<Json<BatchDeleteResponse>> {
    let mut deleted_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for idea_id in request.idea_ids {
        match state.repositories.idea().delete(idea_id).await {
            Ok(_) => deleted_count += 1,
            Err(e) => {
                errors.push(format!("Failed to delete idea {}: {}", idea_id, e));
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

/// Search ideas
async fn search_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<IdeasListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<IdeaResponse>>>> {
    // Enhanced search functionality
    list_ideas(State(state), Query(query)).await
}

/// Get ideas statistics
async fn ideas_stats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<IdeasStatsResponse>> {
    let stats = state.services.idea()
        .get_ideas_stats()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get ideas stats: {}", e)))?;

    Ok(Json(stats))
}

/// Get idea categories
async fn get_idea_categories<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<CategoriesResponse>> {
    let categories = state.repositories.idea()
        .get_categories()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get categories: {}", e)))?;

    let categories_info: Vec<CategoryInfo> = categories
        .into_iter()
        .map(|(name, count)| CategoryInfo {
            name,
            count,
            description: None, // Could be enhanced with category descriptions
        })
        .collect();

    Ok(Json(CategoriesResponse {
        categories: categories_info,
    }))
}

/// Get idea tags
async fn get_idea_tags<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<TagsResponse>> {
    let tags = state.repositories.idea()
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

/// Merge multiple ideas into one
async fn merge_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<MergeIdeasRequest>,
) -> ApiResult<Json<MergeIdeasResponse>> {
    // Validate source ideas exist
    let mut source_ideas = Vec::new();
    for idea_id in &request.source_idea_ids {
        let idea = state.repositories.idea()
            .find_by_id(*idea_id)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to get idea: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("Idea {} not found", idea_id)))?;
        source_ideas.push(idea);
    }

    // Create the merged idea
    let merged_idea = create_idea_internal(&state, request.target_idea).await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create merged idea: {}", e)))?;

    // Delete source ideas if requested
    let mut deleted_count = 0;
    if request.delete_source_ideas.unwrap_or(false) {
        for idea_id in &request.source_idea_ids {
            if let Ok(_) = state.repositories.idea().delete(*idea_id).await {
                deleted_count += 1;
            }
        }
    }

    Ok(Json(MergeIdeasResponse {
        merged_idea,
        source_ideas_deleted: deleted_count,
    }))
}

/// Find duplicate ideas
async fn find_duplicate_ideas<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<DuplicateIdeasResponse>> {
    let threshold = params.get("threshold")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.8);

    let duplicates = state.services.idea()
        .find_duplicates(threshold)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to find duplicates: {}", e)))?;

    Ok(Json(DuplicateIdeasResponse { duplicates }))
}

// Helper function for creating ideas
async fn create_idea_internal<R: RepositoryManager>(
    state: &AppState<R>,
    request: CreateIdeaRequest,
) -> Result<IdeaResponse, ApiError> {
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

    let new_idea = NewIdea {
        session_id: request.session_id,
        analysis_id: request.analysis_id,
        title: request.title,
        description: request.description,
        category: request.category,
        priority: request.priority.unwrap_or_else(|| "medium".to_string()),
        status: request.status.unwrap_or_else(|| "new".to_string()),
        tags: request.tags.unwrap_or_default(),
        source_text: request.source_text,
        confidence_score: request.confidence_score,
        metadata: request.metadata,
    };

    let idea = state.repositories.idea()
        .create(new_idea)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create idea: {}", e)))?;

    let session = state.repositories.session()
        .find_by_id(idea.session_id)
        .await
        .ok()
        .flatten();
    
    let analysis = if let Some(analysis_id) = idea.analysis_id {
        state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    Ok(IdeaResponse {
        id: idea.id,
        session_id: idea.session_id,
        analysis_id: idea.analysis_id,
        title: idea.title,
        description: idea.description,
        category: idea.category,
        priority: idea.priority,
        status: idea.status,
        tags: idea.tags,
        source_text: idea.source_text,
        confidence_score: idea.confidence_score,
        metadata: idea.metadata,
        created_at: idea.created_at,
        updated_at: idea.updated_at,
        session_title: session.and_then(|s| s.title),
        analysis_type: analysis.map(|a| a.analysis_type),
    })
}