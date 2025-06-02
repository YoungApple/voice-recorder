// src/api/routes/v1/mod.rs
//! Version 1 API routes
//!
//! This module contains all v1 API endpoints for the voice recorder service.

pub mod sessions;
pub mod audio;
pub mod transcripts;
pub mod analysis;
pub mod ideas;
pub mod tasks;
pub mod notes;
pub mod ollama;

use axum::Router;
use crate::api::AppState;
use crate::repository::RepositoryManager;

/// Create all v1 API routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .nest("/sessions", sessions::create_routes())
        .nest("/audio", audio::create_routes())
        .nest("/transcripts", transcripts::create_routes())
        .nest("/analysis", analysis::create_routes())
        .nest("/ideas", ideas::create_routes())
        .nest("/tasks", tasks::create_routes())
        .nest("/notes", notes::create_routes())
        .nest("/ollama", ollama::create_routes())
}