// src/api/routes/mod.rs
//! API routes module
//!
//! This module organizes all HTTP routes for the voice recorder API.

pub mod v1;
pub mod health;

use axum::Router;
use crate::api::AppState;
use crate::repository::RepositoryManager;

/// Create all API routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .nest("/v1", v1::create_routes())
        .nest("/health", health::create_routes())
}