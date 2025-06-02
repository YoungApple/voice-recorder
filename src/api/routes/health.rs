// src/api/routes/health.rs
//! Health check routes
//!
//! This module provides health check endpoints for monitoring the application status.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::api::{AppState, ApiResult};
use crate::repository::RepositoryManager;

/// Create health check routes
pub fn create_routes<R: RepositoryManager + 'static>() -> Router<AppState<R>> {
    Router::new()
        .route("/", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/live", get(liveness_check))
        .route("/detailed", get(detailed_health_check))
}

/// Basic health check endpoint
/// Returns 200 OK if the service is running
async fn health_check() -> ApiResult<Json<Value>> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "service": "voice-recorder",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

/// Readiness check endpoint
/// Returns 200 OK if the service is ready to accept requests
async fn readiness_check<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<Value>> {
    // Check if essential services are available
    let mut checks = HashMap::new();
    let mut overall_status = "ready";
    
    // Check Ollama service availability
    let ollama_available = state.services.ollama().is_available().await;
    checks.insert("ollama", if ollama_available { "available" } else { "unavailable" });
    
    if !ollama_available {
        overall_status = "not_ready";
    }
    
    // Check storage directory
    let storage_accessible = tokio::fs::metadata(&state.config.storage.audio_directory)
        .await
        .is_ok();
    checks.insert("storage", if storage_accessible { "accessible" } else { "inaccessible" });
    
    if !storage_accessible {
        overall_status = "not_ready";
    }
    
    let status_code = if overall_status == "ready" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    
    let response = Json(json!({
        "status": overall_status,
        "timestamp": chrono::Utc::now(),
        "checks": checks
    }));
    
    match status_code {
        StatusCode::OK => Ok(response),
        _ => Err(crate::api::error::ApiError::ServiceUnavailable(
            "Service is not ready".to_string()
        ))
    }
}

/// Liveness check endpoint
/// Returns 200 OK if the service is alive (basic functionality)
async fn liveness_check() -> ApiResult<Json<Value>> {
    // Perform basic checks to ensure the service is alive
    let memory_usage = get_memory_usage();
    
    Ok(Json(json!({
        "status": "alive",
        "timestamp": chrono::Utc::now(),
        "uptime_seconds": get_uptime_seconds(),
        "memory_usage_mb": memory_usage
    })))
}

/// Detailed health check endpoint
/// Returns comprehensive health information about all components
async fn detailed_health_check<R: RepositoryManager>(
    State(state): State<AppState<R>>,
) -> ApiResult<Json<Value>> {
    let mut checks = HashMap::new();
    let mut overall_status = "healthy";
    
    // Check Ollama service
    let ollama_available = state.services.ollama().is_available().await;
    let ollama_models = if ollama_available {
        state.services.ollama().list_models().await.unwrap_or_default()
    } else {
        vec![]
    };
    
    checks.insert("ollama", json!({
        "status": if ollama_available { "available" } else { "unavailable" },
        "base_url": state.config.ollama.base_url,
        "models_count": ollama_models.len(),
        "default_model": state.config.ollama.default_model
    }));
    
    if !ollama_available {
        overall_status = "degraded";
    }
    
    // Check storage
    let storage_info = get_storage_info(&state.config.storage.audio_directory).await;
    checks.insert("storage", storage_info);
    
    // Check OpenAI configuration
    checks.insert("openai", json!({
        "configured": state.config.is_openai_configured(),
        "model": state.config.openai.analysis_model
    }));
    
    // System information
    checks.insert("system", json!({
        "memory_usage_mb": get_memory_usage(),
        "uptime_seconds": get_uptime_seconds(),
        "version": env!("CARGO_PKG_VERSION"),
        "rust_version": env!("RUSTC_VERSION"),
        "build_timestamp": env!("BUILD_TIMESTAMP")
    }));
    
    // Configuration summary
    checks.insert("configuration", json!({
        "server_address": state.config.server_address(),
        "default_provider": state.config.analysis.default_provider,
        "auto_analyze": state.config.analysis.auto_analyze,
        "max_file_size_mb": state.config.storage.max_file_size / (1024 * 1024),
        "allowed_formats": state.config.storage.allowed_formats
    }));
    
    Ok(Json(json!({
        "status": overall_status,
        "timestamp": chrono::Utc::now(),
        "service": "voice-recorder",
        "version": env!("CARGO_PKG_VERSION"),
        "checks": checks
    })))
}

/// Get storage information
async fn get_storage_info(storage_path: &std::path::Path) -> Value {
    match tokio::fs::metadata(storage_path).await {
        Ok(_) => {
            // Try to get directory size and file count
            let (file_count, total_size) = get_directory_stats(storage_path).await;
            
            json!({
                "status": "accessible",
                "path": storage_path.to_string_lossy(),
                "file_count": file_count,
                "total_size_mb": total_size / (1024 * 1024)
            })
        }
        Err(e) => {
            json!({
                "status": "inaccessible",
                "path": storage_path.to_string_lossy(),
                "error": e.to_string()
            })
        }
    }
}

/// Get directory statistics (file count and total size)
async fn get_directory_stats(path: &std::path::Path) -> (u64, u64) {
    let mut file_count = 0;
    let mut total_size = 0;
    
    if let Ok(mut entries) = tokio::fs::read_dir(path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    file_count += 1;
                    total_size += metadata.len();
                }
            }
        }
    }
    
    (file_count, total_size)
}

/// Get memory usage in MB (simplified)
fn get_memory_usage() -> u64 {
    // This is a simplified implementation
    // In a real application, you might want to use a proper system monitoring library
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024; // Convert KB to MB
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: return 0 if we can't determine memory usage
    0
}

/// Get uptime in seconds
fn get_uptime_seconds() -> u64 {
    // This is a simplified implementation
    // In a real application, you might want to track the actual start time
    static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    
    let start = START_TIME.get_or_init(|| std::time::Instant::now());
    start.elapsed().as_secs()
}