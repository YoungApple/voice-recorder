// src/api/routes/v1/tasks.rs
//! Tasks management API routes
//!
//! This module provides endpoints for managing extracted tasks from analysis results.

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
    traits::{TaskRepository, NewTask, UpdateTask},
    RepositoryManager,
};
use crate::services::traits::TaskService;

/// Create tasks routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/:id", get(get_task).patch(update_task).delete(delete_task))
        .route("/:id/export", get(export_task))
        .route("/:id/complete", post(complete_task))
        .route("/:id/reopen", post(reopen_task))
        .route("/session/:session_id", get(list_session_tasks))
        .route("/analysis/:analysis_id", get(list_analysis_tasks))
        .route("/batch", post(batch_create_tasks).delete(batch_delete_tasks))
        .route("/batch/update", patch(batch_update_tasks))
        .route("/search", get(search_tasks))
        .route("/stats", get(tasks_stats))
        .route("/priorities", get(get_task_priorities))
        .route("/statuses", get(get_task_statuses))
        .route("/overdue", get(get_overdue_tasks))
        .route("/upcoming", get(get_upcoming_tasks))
        .route("/calendar", get(get_tasks_calendar))
}

#[derive(Debug, Deserialize)]
struct CreateTaskRequest {
    session_id: Uuid,
    analysis_id: Option<Uuid>,
    title: String,
    description: Option<String>,
    priority: Option<String>, // "low", "medium", "high", "critical"
    status: Option<String>, // "todo", "in_progress", "completed", "cancelled"
    due_date: Option<chrono::DateTime<chrono::Utc>>,
    estimated_duration: Option<i32>, // in minutes
    tags: Option<Vec<String>>,
    source_text: Option<String>,
    confidence_score: Option<f64>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct UpdateTaskRequest {
    title: Option<String>,
    description: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    due_date: Option<chrono::DateTime<chrono::Utc>>,
    estimated_duration: Option<i32>,
    tags: Option<Vec<String>>,
    source_text: Option<String>,
    confidence_score: Option<f64>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TasksListQuery {
    #[serde(flatten)]
    pagination: PaginationParams,
    #[serde(flatten)]
    search: SearchParams,
    #[serde(flatten)]
    sort: SortParams,
    session_id: Option<Uuid>,
    analysis_id: Option<Uuid>,
    priority: Option<String>,
    status: Option<String>,
    tags: Option<String>, // Comma-separated tags
    min_confidence: Option<f64>,
    due_after: Option<chrono::DateTime<chrono::Utc>>,
    due_before: Option<chrono::DateTime<chrono::Utc>>,
    created_after: Option<chrono::DateTime<chrono::Utc>>,
    created_before: Option<chrono::DateTime<chrono::Utc>>,
    overdue: Option<bool>,
    completed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BatchCreateTasksRequest {
    tasks: Vec<CreateTaskRequest>,
}

#[derive(Debug, Deserialize)]
struct BatchDeleteTasksRequest {
    task_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
struct BatchUpdateTasksRequest {
    updates: Vec<BatchTaskUpdate>,
}

#[derive(Debug, Deserialize)]
struct BatchTaskUpdate {
    task_id: Uuid,
    update: UpdateTaskRequest,
}

#[derive(Debug, Deserialize)]
struct CalendarQuery {
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    session_id: Option<Uuid>,
    priority: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct TaskResponse {
    id: Uuid,
    session_id: Uuid,
    analysis_id: Option<Uuid>,
    title: String,
    description: Option<String>,
    priority: String,
    status: String,
    due_date: Option<chrono::DateTime<chrono::Utc>>,
    estimated_duration: Option<i32>,
    tags: Vec<String>,
    source_text: Option<String>,
    confidence_score: Option<f64>,
    metadata: Option<serde_json::Value>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    // Computed fields
    is_overdue: bool,
    days_until_due: Option<i64>,
    // Related data
    session_title: Option<String>,
    analysis_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct TasksStatsResponse {
    total_tasks: i64,
    priorities: std::collections::HashMap<String, i64>,
    statuses: std::collections::HashMap<String, i64>,
    tags: std::collections::HashMap<String, i64>,
    avg_confidence_score: f64,
    completion_rate: f64,
    overdue_count: i64,
    upcoming_count: i64, // Due in next 7 days
    avg_completion_time: f64, // in days
    tasks_per_day: Vec<DailyCount>,
    priority_completion_rates: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
struct DailyCount {
    date: chrono::NaiveDate,
    count: i64,
}

#[derive(Debug, Serialize)]
struct BatchCreateResponse {
    created: usize,
    failed: usize,
    tasks: Vec<TaskResponse>,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BatchDeleteResponse {
    deleted: usize,
    failed: usize,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BatchUpdateResponse {
    updated: usize,
    failed: usize,
    tasks: Vec<TaskResponse>,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PrioritiesResponse {
    priorities: Vec<PriorityInfo>,
}

#[derive(Debug, Serialize)]
struct PriorityInfo {
    name: String,
    display_name: String,
    color: String,
    order: i32,
    count: i64,
}

#[derive(Debug, Serialize)]
struct StatusesResponse {
    statuses: Vec<StatusInfo>,
}

#[derive(Debug, Serialize)]
struct StatusInfo {
    name: String,
    display_name: String,
    color: String,
    is_completed: bool,
    count: i64,
}

#[derive(Debug, Serialize)]
struct CalendarResponse {
    tasks: Vec<CalendarTask>,
    summary: CalendarSummary,
}

#[derive(Debug, Serialize)]
struct CalendarTask {
    id: Uuid,
    title: String,
    priority: String,
    status: String,
    due_date: chrono::DateTime<chrono::Utc>,
    estimated_duration: Option<i32>,
    session_title: Option<String>,
}

#[derive(Debug, Serialize)]
struct CalendarSummary {
    total_tasks: i64,
    overdue_tasks: i64,
    completed_tasks: i64,
    high_priority_tasks: i64,
}

/// List tasks with filtering and pagination
async fn list_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<TasksListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    let tags_filter = query.tags.as_ref().map(|t| {
        t.split(',')
            .map(|tag| tag.trim().to_string())
            .collect::<Vec<_>>()
    });

    let tasks = state.repositories.task()
        .find_with_filters(
            query.session_id,
            query.analysis_id,
            query.priority.as_deref(),
            query.status.as_deref(),
            tags_filter.as_ref().map(|v| v.as_slice()),
            query.min_confidence,
            query.due_after,
            query.due_before,
            query.created_after,
            query.created_before,
            query.overdue,
            query.completed,
            query.search.q.as_deref(),
            Some(query.pagination.limit),
            Some(query.pagination.offset),
            query.sort.sort_by.as_deref(),
            query.sort.sort_order.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to list tasks: {}", e)))?;

    let total = state.repositories.task()
        .count_with_filters(
            query.session_id,
            query.analysis_id,
            query.priority.as_deref(),
            query.status.as_deref(),
            tags_filter.as_ref().map(|v| v.as_slice()),
            query.min_confidence,
            query.due_after,
            query.due_before,
            query.created_after,
            query.created_before,
            query.overdue,
            query.completed,
            query.search.q.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to count tasks: {}", e)))?;

    // Enrich with related data and computed fields
    let mut responses = Vec::new();
    let now = chrono::Utc::now();
    
    for task in tasks {
        let session = state.repositories.session()
            .find_by_id(task.session_id)
            .await
            .ok()
            .flatten();
        
        let analysis = if let Some(analysis_id) = task.analysis_id {
            state.repositories.analysis()
                .find_by_id(analysis_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let (is_overdue, days_until_due) = if let Some(due_date) = task.due_date {
            let days_diff = (due_date - now).num_days();
            (days_diff < 0 && task.status != "completed", Some(days_diff))
        } else {
            (false, None)
        };

        responses.push(TaskResponse {
            id: task.id,
            session_id: task.session_id,
            analysis_id: task.analysis_id,
            title: task.title,
            description: task.description,
            priority: task.priority,
            status: task.status,
            due_date: task.due_date,
            estimated_duration: task.estimated_duration,
            tags: task.tags,
            source_text: task.source_text,
            confidence_score: task.confidence_score,
            metadata: task.metadata,
            completed_at: task.completed_at,
            created_at: task.created_at,
            updated_at: task.updated_at,
            is_overdue,
            days_until_due,
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

/// Create a new task
async fn create_task<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<CreateTaskRequest>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
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

    let new_task = NewTask {
        session_id: request.session_id,
        analysis_id: request.analysis_id,
        title: request.title,
        description: request.description,
        priority: request.priority.unwrap_or_else(|| "medium".to_string()),
        status: request.status.unwrap_or_else(|| "todo".to_string()),
        due_date: request.due_date,
        estimated_duration: request.estimated_duration,
        tags: request.tags.unwrap_or_default(),
        source_text: request.source_text,
        confidence_score: request.confidence_score,
        metadata: request.metadata,
    };

    let task = state.repositories.task()
        .create(new_task)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create task: {}", e)))?;

    let response = create_task_response(&state, task).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Get a specific task by ID
async fn get_task<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
    let task = state.repositories.task()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    let response = create_task_response(&state, task).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Update a task
async fn update_task<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateTaskRequest>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
    // Check if task exists
    let _task = state.repositories.task()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    let update_task = UpdateTask {
        title: request.title,
        description: request.description,
        priority: request.priority,
        status: request.status,
        due_date: request.due_date,
        estimated_duration: request.estimated_duration,
        tags: request.tags,
        source_text: request.source_text,
        confidence_score: request.confidence_score,
        metadata: request.metadata,
    };

    let updated_task = state.repositories.task()
        .update(id, update_task)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to update task: {}", e)))?;

    let response = create_task_response(&state, updated_task).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Delete a task
async fn delete_task<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Check if task exists
    let _task = state.repositories.task()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    state.repositories.task()
        .delete(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete task: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Export task
async fn export_task<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<axum::response::Response> {
    let task = state.repositories.task()
        .find_by_id(id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    let format = params.get("format").map(|s| s.as_str()).unwrap_or("json");
    let include_metadata = params.get("include_metadata")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);

    let (content, content_type, filename) = match format {
        "json" => {
            let json_data = if include_metadata {
                serde_json::to_string_pretty(&task)
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize task: {}", e)))?
            } else {
                serde_json::to_string_pretty(&serde_json::json!({
                    "title": task.title,
                    "description": task.description,
                    "priority": task.priority,
                    "status": task.status,
                    "due_date": task.due_date,
                    "tags": task.tags
                }))
                .map_err(|e| ApiError::InternalServerError(format!("Failed to serialize task: {}", e)))?
            };
            (json_data, "application/json", format!("task_{}.json", task.id))
        }
        "txt" => {
            let text_content = format!(
                "Title: {}\nPriority: {}\nStatus: {}\nDue Date: {}\nEstimated Duration: {} minutes\nTags: {}\nCreated: {}\n\nDescription:\n{}\n\nSource Text:\n{}\n",
                task.title,
                task.priority,
                task.status,
                task.due_date.map(|d| d.to_string()).unwrap_or_else(|| "Not set".to_string()),
                task.estimated_duration.map(|d| d.to_string()).unwrap_or_else(|| "Not set".to_string()),
                task.tags.join(", "),
                task.created_at,
                task.description.unwrap_or_else(|| "No description".to_string()),
                task.source_text.unwrap_or_else(|| "No source text".to_string())
            );
            (text_content, "text/plain", format!("task_{}.txt", task.id))
        }
        "md" => {
            let markdown_content = format!(
                "# {}\n\n**Priority:** {}\n**Status:** {}\n**Due Date:** {}\n**Estimated Duration:** {} minutes\n**Tags:** {}\n**Created:** {}\n\n## Description\n\n{}\n\n## Source Text\n\n{}\n",
                task.title,
                task.priority,
                task.status,
                task.due_date.map(|d| d.to_string()).unwrap_or_else(|| "Not set".to_string()),
                task.estimated_duration.map(|d| d.to_string()).unwrap_or_else(|| "Not set".to_string()),
                task.tags.join(", "),
                task.created_at,
                task.description.unwrap_or_else(|| "No description".to_string()),
                task.source_text.unwrap_or_else(|| "No source text".to_string())
            );
            (markdown_content, "text/markdown", format!("task_{}.md", task.id))
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

/// Complete a task
async fn complete_task<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
    let update_task = UpdateTask {
        status: Some("completed".to_string()),
        ..Default::default()
    };

    let updated_task = state.repositories.task()
        .update(id, update_task)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to complete task: {}", e)))?;

    let response = create_task_response(&state, updated_task).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// Reopen a completed task
async fn reopen_task<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
    let update_task = UpdateTask {
        status: Some("todo".to_string()),
        ..Default::default()
    };

    let updated_task = state.repositories.task()
        .update(id, update_task)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to reopen task: {}", e)))?;

    let response = create_task_response(&state, updated_task).await?;

    Ok(Json(ApiResponse {
        data: response,
        total: None,
        page: None,
        per_page: None,
    }))
}

/// List tasks for a specific session
async fn list_session_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<TasksListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    let mut modified_query = query;
    modified_query.session_id = Some(session_id);
    list_tasks(State(state), Query(modified_query)).await
}

/// List tasks for a specific analysis
async fn list_analysis_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Path(analysis_id): Path<Uuid>,
    Query(query): Query<TasksListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    let mut modified_query = query;
    modified_query.analysis_id = Some(analysis_id);
    list_tasks(State(state), Query(modified_query)).await
}

/// Batch create tasks
async fn batch_create_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchCreateTasksRequest>,
) -> ApiResult<Json<BatchCreateResponse>> {
    let mut created_tasks = Vec::new();
    let mut errors = Vec::new();
    let mut created_count = 0;
    let mut failed_count = 0;

    for task_request in request.tasks {
        match create_task_internal(&state, task_request).await {
            Ok(task) => {
                created_tasks.push(task);
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
        tasks: created_tasks,
        errors,
    }))
}

/// Batch delete tasks
async fn batch_delete_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchDeleteTasksRequest>,
) -> ApiResult<Json<BatchDeleteResponse>> {
    let mut deleted_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for task_id in request.task_ids {
        match state.repositories.task().delete(task_id).await {
            Ok(_) => deleted_count += 1,
            Err(e) => {
                errors.push(format!("Failed to delete task {}: {}", task_id, e));
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

/// Batch update tasks
async fn batch_update_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Json(request): Json<BatchUpdateTasksRequest>,
) -> ApiResult<Json<BatchUpdateResponse>> {
    let mut updated_tasks = Vec::new();
    let mut errors = Vec::new();
    let mut updated_count = 0;
    let mut failed_count = 0;

    for update_request in request.updates {
        match state.repositories.task()
            .update(update_request.task_id, UpdateTask {
                title: update_request.update.title,
                description: update_request.update.description,
                priority: update_request.update.priority,
                status: update_request.update.status,
                due_date: update_request.update.due_date,
                estimated_duration: update_request.update.estimated_duration,
                tags: update_request.update.tags,
                source_text: update_request.update.source_text,
                confidence_score: update_request.update.confidence_score,
                metadata: update_request.update.metadata,
            })
            .await
        {
            Ok(task) => {
                match create_task_response(&state, task).await {
                    Ok(response) => {
                        updated_tasks.push(response);
                        updated_count += 1;
                    }
                    Err(e) => {
                        errors.push(format!("Failed to create response for task {}: {}", update_request.task_id, e));
                        failed_count += 1;
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Failed to update task {}: {}", update_request.task_id, e));
                failed_count += 1;
            }
        }
    }

    Ok(Json(BatchUpdateResponse {
        updated: updated_count,
        failed: failed_count,
        tasks: updated_tasks,
        errors,
    }))
}

/// Search tasks
async fn search_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<TasksListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    // Enhanced search functionality
    list_tasks(State(state), Query(query)).await
}

/// Get tasks statistics
async fn tasks_stats<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<TasksStatsResponse>> {
    let stats = state.services.task()
        .get_tasks_stats()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get tasks stats: {}", e)))?;

    Ok(Json(stats))
}

/// Get task priorities
async fn get_task_priorities<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<PrioritiesResponse>> {
    let priorities_count = state.repositories.task()
        .get_priorities()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get priorities: {}", e)))?;

    let priorities = vec![
        PriorityInfo {
            name: "low".to_string(),
            display_name: "Low".to_string(),
            color: "#10B981".to_string(), // green
            order: 1,
            count: priorities_count.get("low").copied().unwrap_or(0),
        },
        PriorityInfo {
            name: "medium".to_string(),
            display_name: "Medium".to_string(),
            color: "#F59E0B".to_string(), // yellow
            order: 2,
            count: priorities_count.get("medium").copied().unwrap_or(0),
        },
        PriorityInfo {
            name: "high".to_string(),
            display_name: "High".to_string(),
            color: "#EF4444".to_string(), // red
            order: 3,
            count: priorities_count.get("high").copied().unwrap_or(0),
        },
        PriorityInfo {
            name: "critical".to_string(),
            display_name: "Critical".to_string(),
            color: "#7C2D12".to_string(), // dark red
            order: 4,
            count: priorities_count.get("critical").copied().unwrap_or(0),
        },
    ];

    Ok(Json(PrioritiesResponse { priorities }))
}

/// Get task statuses
async fn get_task_statuses<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<StatusesResponse>> {
    let statuses_count = state.repositories.task()
        .get_statuses()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get statuses: {}", e)))?;

    let statuses = vec![
        StatusInfo {
            name: "todo".to_string(),
            display_name: "To Do".to_string(),
            color: "#6B7280".to_string(), // gray
            is_completed: false,
            count: statuses_count.get("todo").copied().unwrap_or(0),
        },
        StatusInfo {
            name: "in_progress".to_string(),
            display_name: "In Progress".to_string(),
            color: "#3B82F6".to_string(), // blue
            is_completed: false,
            count: statuses_count.get("in_progress").copied().unwrap_or(0),
        },
        StatusInfo {
            name: "completed".to_string(),
            display_name: "Completed".to_string(),
            color: "#10B981".to_string(), // green
            is_completed: true,
            count: statuses_count.get("completed").copied().unwrap_or(0),
        },
        StatusInfo {
            name: "cancelled".to_string(),
            display_name: "Cancelled".to_string(),
            color: "#EF4444".to_string(), // red
            is_completed: true,
            count: statuses_count.get("cancelled").copied().unwrap_or(0),
        },
    ];

    Ok(Json(StatusesResponse { statuses }))
}

/// Get overdue tasks
async fn get_overdue_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<TasksListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    let mut modified_query = query;
    modified_query.overdue = Some(true);
    modified_query.completed = Some(false);
    list_tasks(State(state), Query(modified_query)).await
}

/// Get upcoming tasks (due in next 7 days)
async fn get_upcoming_tasks<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<TasksListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    let now = chrono::Utc::now();
    let next_week = now + chrono::Duration::days(7);
    
    let mut modified_query = query;
    modified_query.due_after = Some(now);
    modified_query.due_before = Some(next_week);
    modified_query.completed = Some(false);
    list_tasks(State(state), Query(modified_query)).await
}

/// Get tasks calendar view
async fn get_tasks_calendar<R: RepositoryManager>(
    State(state): State<AppState<R>>,
    Query(query): Query<CalendarQuery>,
) -> ApiResult<Json<CalendarResponse>> {
    let start_datetime = query.start_date.and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let end_datetime = query.end_date.and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc();

    let tasks = state.repositories.task()
        .find_with_filters(
            query.session_id,
            None, // analysis_id
            query.priority.as_deref(),
            query.status.as_deref(),
            None, // tags
            None, // min_confidence
            Some(start_datetime),
            Some(end_datetime),
            None, // created_after
            None, // created_before
            None, // overdue
            None, // completed
            None, // search
            None, // limit
            None, // offset
            Some("due_date"), // sort_by
            Some("asc"), // sort_order
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get calendar tasks: {}", e)))?;

    let mut calendar_tasks = Vec::new();
    let mut total_tasks = 0;
    let mut overdue_tasks = 0;
    let mut completed_tasks = 0;
    let mut high_priority_tasks = 0;
    let now = chrono::Utc::now();

    for task in tasks {
        if let Some(due_date) = task.due_date {
            let session = state.repositories.session()
                .find_by_id(task.session_id)
                .await
                .ok()
                .flatten();

            calendar_tasks.push(CalendarTask {
                id: task.id,
                title: task.title.clone(),
                priority: task.priority.clone(),
                status: task.status.clone(),
                due_date,
                estimated_duration: task.estimated_duration,
                session_title: session.and_then(|s| s.title),
            });

            total_tasks += 1;
            
            if due_date < now && task.status != "completed" {
                overdue_tasks += 1;
            }
            
            if task.status == "completed" {
                completed_tasks += 1;
            }
            
            if task.priority == "high" || task.priority == "critical" {
                high_priority_tasks += 1;
            }
        }
    }

    let summary = CalendarSummary {
        total_tasks,
        overdue_tasks,
        completed_tasks,
        high_priority_tasks,
    };

    Ok(Json(CalendarResponse {
        tasks: calendar_tasks,
        summary,
    }))
}

// Helper functions

async fn create_task_response<R: RepositoryManager>(
    state: &AppState<R>,
    task: crate::repository::traits::Task,
) -> Result<TaskResponse, ApiError> {
    let session = state.repositories.session()
        .find_by_id(task.session_id)
        .await
        .ok()
        .flatten();
    
    let analysis = if let Some(analysis_id) = task.analysis_id {
        state.repositories.analysis()
            .find_by_id(analysis_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let now = chrono::Utc::now();
    let (is_overdue, days_until_due) = if let Some(due_date) = task.due_date {
        let days_diff = (due_date - now).num_days();
        (days_diff < 0 && task.status != "completed", Some(days_diff))
    } else {
        (false, None)
    };

    Ok(TaskResponse {
        id: task.id,
        session_id: task.session_id,
        analysis_id: task.analysis_id,
        title: task.title,
        description: task.description,
        priority: task.priority,
        status: task.status,
        due_date: task.due_date,
        estimated_duration: task.estimated_duration,
        tags: task.tags,
        source_text: task.source_text,
        confidence_score: task.confidence_score,
        metadata: task.metadata,
        completed_at: task.completed_at,
        created_at: task.created_at,
        updated_at: task.updated_at,
        is_overdue,
        days_until_due,
        session_title: session.and_then(|s| s.title),
        analysis_type: analysis.map(|a| a.analysis_type),
    })
}

async fn create_task_internal<R: RepositoryManager>(
    state: &AppState<R>,
    request: CreateTaskRequest,
) -> Result<TaskResponse, ApiError> {
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

    let new_task = NewTask {
        session_id: request.session_id,
        analysis_id: request.analysis_id,
        title: request.title,
        description: request.description,
        priority: request.priority.unwrap_or_else(|| "medium".to_string()),
        status: request.status.unwrap_or_else(|| "todo".to_string()),
        due_date: request.due_date,
        estimated_duration: request.estimated_duration,
        tags: request.tags.unwrap_or_default(),
        source_text: request.source_text,
        confidence_score: request.confidence_score,
        metadata: request.metadata,
    };

    let task = state.repositories.task()
        .create(new_task)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create task: {}", e)))?;

    create_task_response(state, task).await
}