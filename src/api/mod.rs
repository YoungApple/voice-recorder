// src/api/mod.rs
//! API layer module
//!
//! This module contains the HTTP API layer that handles incoming requests,
//! validates input, calls appropriate services, and formats responses.

pub mod handlers;
pub mod routes;
pub mod middleware;
pub mod responses;
pub mod extractors;

use axum::Router;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_http::timeout::TimeoutLayer;
use std::time::Duration;

use crate::services::ServiceManager;
use crate::repository::RepositoryManager;
use crate::config::Config;

/// API application state
#[derive(Clone)]
pub struct AppState<R: RepositoryManager> {
    pub services: Arc<ServiceManager<R>>,
    pub config: Arc<Config>,
}

impl<R: RepositoryManager> AppState<R> {
    pub fn new(services: Arc<ServiceManager<R>>, config: Arc<Config>) -> Self {
        Self { services, config }
    }
}

/// Create the main API router with all routes and middleware
pub fn create_router<R: RepositoryManager + 'static>(state: AppState<R>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(state.config.server.cors_origins.iter().map(|origin| {
            origin.parse().expect("Invalid CORS origin")
        }).collect::<Vec<_>>())
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::PATCH,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(TimeoutLayer::new(Duration::from_secs(
            state.config.server.request_timeout_secs,
        )))
        .layer(middleware::request_id::RequestIdLayer::new())
        .layer(middleware::logging::LoggingLayer::new());

    Router::new()
        .nest("/api/v1", routes::v1::create_routes())
        .nest("/health", routes::health::create_routes())
        .layer(middleware)
        .with_state(state)
}

/// API error types
pub mod error {
    use axum::{
        http::StatusCode,
        response::{IntoResponse, Response},
        Json,
    };
    use serde_json::json;
    use std::fmt;

    /// API error type
    #[derive(Debug)]
    pub enum ApiError {
        BadRequest(String),
        Unauthorized(String),
        Forbidden(String),
        NotFound(String),
        Conflict(String),
        UnprocessableEntity(String),
        InternalServerError(String),
        ServiceUnavailable(String),
    }

    impl fmt::Display for ApiError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ApiError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
                ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
                ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
                ApiError::NotFound(msg) => write!(f, "Not Found: {}", msg),
                ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
                ApiError::UnprocessableEntity(msg) => write!(f, "Unprocessable Entity: {}", msg),
                ApiError::InternalServerError(msg) => write!(f, "Internal Server Error: {}", msg),
                ApiError::ServiceUnavailable(msg) => write!(f, "Service Unavailable: {}", msg),
            }
        }
    }

    impl std::error::Error for ApiError {}

    impl IntoResponse for ApiError {
        fn into_response(self) -> Response {
            let (status, error_message) = match self {
                ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
                ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
                ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
                ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
                ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
                ApiError::UnprocessableEntity(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
                ApiError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
                ApiError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            };

            let body = Json(json!({
                "error": {
                    "message": error_message,
                    "code": status.as_u16()
                }
            }));

            (status, body).into_response()
        }
    }

    impl From<anyhow::Error> for ApiError {
        fn from(err: anyhow::Error) -> Self {
            tracing::error!("Internal error: {:?}", err);
            ApiError::InternalServerError("An internal error occurred".to_string())
        }
    }

    impl From<sqlx::Error> for ApiError {
        fn from(err: sqlx::Error) -> Self {
            tracing::error!("Database error: {:?}", err);
            match err {
                sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
                sqlx::Error::Database(db_err) => {
                    if db_err.is_unique_violation() {
                        ApiError::Conflict("Resource already exists".to_string())
                    } else {
                        ApiError::InternalServerError("Database error occurred".to_string())
                    }
                }
                _ => ApiError::InternalServerError("Database error occurred".to_string()),
            }
        }
    }

    impl From<serde_json::Error> for ApiError {
        fn from(err: serde_json::Error) -> Self {
            tracing::error!("JSON serialization error: {:?}", err);
            ApiError::BadRequest("Invalid JSON format".to_string())
        }
    }

    impl From<uuid::Error> for ApiError {
        fn from(err: uuid::Error) -> Self {
            tracing::error!("UUID parsing error: {:?}", err);
            ApiError::BadRequest("Invalid UUID format".to_string())
        }
    }
}

/// API result type
pub type ApiResult<T> = Result<T, error::ApiError>;

/// Common API response wrapper
#[derive(serde::Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Pagination parameters
#[derive(serde::Deserialize, Debug)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }

    pub fn limit(&self) -> i64 {
        self.page_size
    }

    pub fn validate(&self) -> ApiResult<()> {
        if self.page < 1 {
            return Err(error::ApiError::BadRequest(
                "Page must be greater than 0".to_string(),
            ));
        }
        if self.page_size < 1 || self.page_size > 100 {
            return Err(error::ApiError::BadRequest(
                "Page size must be between 1 and 100".to_string(),
            ));
        }
        Ok(())
    }
}

/// Search parameters
#[derive(serde::Deserialize, Debug)]
pub struct SearchParams {
    pub q: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Sort parameters
#[derive(serde::Deserialize, Debug)]
pub struct SortParams {
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl SortParams {
    pub fn validate_sort_by(&self, allowed_fields: &[&str]) -> ApiResult<()> {
        if let Some(sort_by) = &self.sort_by {
            if !allowed_fields.contains(&sort_by.as_str()) {
                return Err(error::ApiError::BadRequest(format!(
                    "Invalid sort field. Allowed fields: {}",
                    allowed_fields.join(", ")
                )));
            }
        }
        Ok(())
    }

    pub fn validate_sort_order(&self) -> ApiResult<()> {
        if let Some(sort_order) = &self.sort_order {
            match sort_order.to_lowercase().as_str() {
                "asc" | "desc" => Ok(()),
                _ => Err(error::ApiError::BadRequest(
                    "Sort order must be 'asc' or 'desc'".to_string(),
                )),
            }
        } else {
            Ok(())
        }
    }
}