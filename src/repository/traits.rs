// src/repository/traits.rs
//! Repository trait definitions for data access layer abstraction
//!
//! This module defines the core repository traits that abstract database operations.
//! Each entity has its own repository trait with CRUD operations and specific queries.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::Result;

// Re-export common types
pub use crate::storage::{Priority, NoteType};

/// Filter and pagination options for session queries
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SessionFilter {
    pub search: Option<String>,
    pub status: Option<SessionStatus>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<SessionSortBy>,
    pub sort_order: Option<SortOrder>,
}

/// Session status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Archived,
    Deleted,
}

/// Session sorting options
#[derive(Debug, Clone, Deserialize)]
pub enum SessionSortBy {
    CreatedAt,
    UpdatedAt,
    Title,
    Duration,
}

/// Sort order enumeration
#[derive(Debug, Clone, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// New session data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSession {
    pub title: String,
    pub duration_ms: i64,
    pub metadata: Option<serde_json::Value>,
}

/// Session update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub title: Option<String>,
    pub status: Option<SessionStatus>,
    pub metadata: Option<serde_json::Value>,
}

/// Complete session data model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub status: SessionStatus,
    pub metadata: Option<serde_json::Value>,
}

/// Audio file data model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AudioFile {
    pub id: Uuid,
    pub session_id: Uuid,
    pub file_path: String,
    pub file_size: i64,
    pub format: String,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub checksum: Option<String>,
}

/// New audio file data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAudioFile {
    pub session_id: Uuid,
    pub file_path: String,
    pub file_size: i64,
    pub format: String,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub checksum: Option<String>,
}

/// Transcript data model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Transcript {
    pub id: Uuid,
    pub session_id: Uuid,
    pub content: String,
    pub language: Option<String>,
    pub confidence_score: Option<rust_decimal::Decimal>,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub processing_time_ms: Option<i32>,
}

/// New transcript data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTranscript {
    pub session_id: Uuid,
    pub content: String,
    pub language: Option<String>,
    pub confidence_score: Option<rust_decimal::Decimal>,
    pub provider: String,
    pub processing_time_ms: Option<i32>,
}

/// Analysis result data model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnalysisResult {
    pub id: Uuid,
    pub session_id: Uuid,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub provider: String,
    pub model_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processing_time_ms: Option<i32>,
}

/// New analysis result data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAnalysisResult {
    pub session_id: Uuid,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub provider: String,
    pub model_version: Option<String>,
    pub processing_time_ms: Option<i32>,
}

/// Analysis result update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisUpdate {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub model_version: Option<String>,
}

/// Idea data model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Idea {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub content: String,
    pub category: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}

/// New idea data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewIdea {
    pub analysis_id: Uuid,
    pub content: String,
    pub category: Option<String>,
    pub priority: i32,
}

/// Task data model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub status: TaskStatus,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

/// New task data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub analysis_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub due_date: Option<DateTime<Utc>>,
}

/// Task update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub status: Option<TaskStatus>,
    pub due_date: Option<DateTime<Utc>>,
}

/// Structured note data model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StructuredNote {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub title: String,
    pub content: String,
    pub note_type: NoteType,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New structured note data for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewStructuredNote {
    pub analysis_id: Uuid,
    pub title: String,
    pub content: String,
    pub note_type: NoteType,
    pub tags: Vec<String>,
}

/// Structured note update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredNoteUpdate {
    pub title: Option<String>,
    pub content: Option<String>,
    pub note_type: Option<NoteType>,
    pub tags: Option<Vec<String>>,
}

// Repository trait definitions

/// Session repository trait for managing voice recording sessions
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Create a new session
    async fn create(&self, session: &NewSession) -> Result<Session>;
    
    /// Find a session by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Session>>;
    
    /// List sessions with filtering and pagination
    async fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>>;
    
    /// Update a session
    async fn update(&self, id: &Uuid, updates: &SessionUpdate) -> Result<Session>;
    
    /// Delete a session (soft delete by setting status to Deleted)
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Count total sessions matching filter
    async fn count(&self, filter: &SessionFilter) -> Result<i64>;
    
    /// Find sessions by status
    async fn find_by_status(&self, status: SessionStatus) -> Result<Vec<Session>>;
}

/// Audio file repository trait for managing audio files
#[async_trait]
pub trait AudioRepository: Send + Sync {
    /// Create a new audio file record
    async fn create(&self, audio: &NewAudioFile) -> Result<AudioFile>;
    
    /// Find audio file by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<AudioFile>>;
    
    /// Find audio file by session ID
    async fn find_by_session_id(&self, session_id: &Uuid) -> Result<Option<AudioFile>>;
    
    /// Delete audio file record
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Update audio file checksum
    async fn update_checksum(&self, id: &Uuid, checksum: &str) -> Result<()>;
}

/// Transcript repository trait for managing transcription data
#[async_trait]
pub trait TranscriptRepository: Send + Sync {
    /// Create a new transcript
    async fn create(&self, transcript: &NewTranscript) -> Result<Transcript>;
    
    /// Find transcript by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Transcript>>;
    
    /// Find transcript by session ID
    async fn find_by_session_id(&self, session_id: &Uuid) -> Result<Option<Transcript>>;
    
    /// Delete transcript
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Find transcripts by provider
    async fn find_by_provider(&self, provider: &str) -> Result<Vec<Transcript>>;
}

/// Analysis repository trait for managing AI analysis results
#[async_trait]
pub trait AnalysisRepository: Send + Sync {
    /// Create a new analysis result
    async fn create(&self, analysis: &NewAnalysisResult) -> Result<AnalysisResult>;
    
    /// Find analysis result by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<AnalysisResult>>;
    
    /// Find analysis result by session ID
    async fn find_by_session_id(&self, session_id: &Uuid) -> Result<Option<AnalysisResult>>;
    
    /// Update analysis result
    async fn update(&self, id: &Uuid, updates: &AnalysisUpdate) -> Result<AnalysisResult>;
    
    /// Delete analysis result
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Find analysis results by provider
    async fn find_by_provider(&self, provider: &str) -> Result<Vec<AnalysisResult>>;
}

/// Idea repository trait for managing extracted ideas
#[async_trait]
pub trait IdeaRepository: Send + Sync {
    /// Create a new idea
    async fn create(&self, idea: &NewIdea) -> Result<Idea>;
    
    /// Find idea by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Idea>>;
    
    /// Find ideas by analysis ID
    async fn find_by_analysis_id(&self, analysis_id: &Uuid) -> Result<Vec<Idea>>;
    
    /// Update idea
    async fn update(&self, id: &Uuid, content: &str, category: Option<&str>, priority: i32) -> Result<Idea>;
    
    /// Delete idea
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Find ideas by category
    async fn find_by_category(&self, category: &str) -> Result<Vec<Idea>>;
}

/// Task repository trait for managing extracted tasks
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// Create a new task
    async fn create(&self, task: &NewTask) -> Result<Task>;
    
    /// Find task by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Task>>;
    
    /// Find tasks by analysis ID
    async fn find_by_analysis_id(&self, analysis_id: &Uuid) -> Result<Vec<Task>>;
    
    /// Update task
    async fn update(&self, id: &Uuid, updates: &TaskUpdate) -> Result<Task>;
    
    /// Delete task
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Find tasks by status
    async fn find_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;
    
    /// Find tasks by priority
    async fn find_by_priority(&self, priority: Priority) -> Result<Vec<Task>>;
    
    /// Mark task as completed
    async fn mark_completed(&self, id: &Uuid) -> Result<Task>;
}

/// Structured note repository trait for managing structured notes
#[async_trait]
pub trait StructuredNoteRepository: Send + Sync {
    /// Create a new structured note
    async fn create(&self, note: &NewStructuredNote) -> Result<StructuredNote>;
    
    /// Find structured note by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<StructuredNote>>;
    
    /// Find structured notes by analysis ID
    async fn find_by_analysis_id(&self, analysis_id: &Uuid) -> Result<Vec<StructuredNote>>;
    
    /// Update structured note
    async fn update(&self, id: &Uuid, updates: &StructuredNoteUpdate) -> Result<StructuredNote>;
    
    /// Delete structured note
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Find structured notes by type
    async fn find_by_note_type(&self, note_type: NoteType) -> Result<Vec<StructuredNote>>;
    
    /// Find structured notes by tags
    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<StructuredNote>>;
}