// src/services/traits.rs
//! Service layer trait definitions
//!
//! This module defines the service layer traits that encapsulate business logic
//! and coordinate between the API layer and repository layer.

use async_trait::async_trait;
use uuid::Uuid;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::repository::traits::*;

/// Audio processing service for handling audio file operations
#[async_trait]
pub trait AudioService: Send + Sync {
    /// Process and store an uploaded audio file
    async fn process_audio_file(
        &self,
        session_id: Uuid,
        file_data: &[u8],
        filename: &str,
        format: &str,
    ) -> Result<AudioFile>;
    
    /// Get audio file information by session ID
    async fn get_audio_by_session(&self, session_id: &Uuid) -> Result<Option<AudioFile>>;
    
    /// Delete audio file and its record
    async fn delete_audio_file(&self, audio_id: &Uuid) -> Result<()>;
    
    /// Validate audio file format and metadata
    async fn validate_audio_file(&self, file_data: &[u8], format: &str) -> Result<AudioMetadata>;
    
    /// Get audio file path for playback
    async fn get_audio_file_path(&self, audio_id: &Uuid) -> Result<Option<String>>;
}

/// Audio metadata extracted from file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub duration_ms: i64,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub file_size: i64,
    pub format: String,
}

/// Transcription service for converting audio to text
#[async_trait]
pub trait TranscriptionService: Send + Sync {
    /// Transcribe audio file to text
    async fn transcribe_audio(
        &self,
        session_id: &Uuid,
        audio_file_path: &str,
        language: Option<&str>,
    ) -> Result<Transcript>;
    
    /// Get transcript by session ID
    async fn get_transcript_by_session(&self, session_id: &Uuid) -> Result<Option<Transcript>>;
    
    /// Re-transcribe with different provider or settings
    async fn retranscribe(
        &self,
        session_id: &Uuid,
        provider: &str,
        language: Option<&str>,
    ) -> Result<Transcript>;
    
    /// Get available transcription providers
    fn get_available_providers(&self) -> Vec<String>;
    
    /// Get supported languages for a provider
    fn get_supported_languages(&self, provider: &str) -> Vec<String>;
}

/// Analysis service for AI-powered content analysis
#[async_trait]
pub trait AnalysisService: Send + Sync {
    /// Analyze transcript content and extract structured information
    async fn analyze_transcript(
        &self,
        session_id: &Uuid,
        transcript_content: &str,
        language: Option<&str>,
    ) -> Result<AnalysisResult>;
    
    /// Get analysis result by session ID
    async fn get_analysis_by_session(&self, session_id: &Uuid) -> Result<Option<AnalysisResult>>;
    
    /// Re-analyze with different provider or model
    async fn reanalyze(
        &self,
        session_id: &Uuid,
        provider: &str,
        model_version: Option<&str>,
    ) -> Result<AnalysisResult>;
    
    /// Extract ideas from analysis result
    async fn extract_ideas(&self, analysis_id: &Uuid) -> Result<Vec<Idea>>;
    
    /// Extract tasks from analysis result
    async fn extract_tasks(&self, analysis_id: &Uuid) -> Result<Vec<Task>>;
    
    /// Extract structured notes from analysis result
    async fn extract_structured_notes(&self, analysis_id: &Uuid) -> Result<Vec<StructuredNote>>;
    
    /// Get available analysis providers
    fn get_available_providers(&self) -> Vec<String>;
}

/// Session management service
#[async_trait]
pub trait SessionService: Send + Sync {
    /// Create a new voice recording session
    async fn create_session(&self, title: &str, duration_ms: i64) -> Result<Session>;
    
    /// Get session by ID
    async fn get_session(&self, id: &Uuid) -> Result<Option<Session>>;
    
    /// List sessions with filtering and pagination
    async fn list_sessions(&self, filter: &SessionFilter) -> Result<SessionListResponse>;
    
    /// Update session information
    async fn update_session(&self, id: &Uuid, updates: &SessionUpdate) -> Result<Session>;
    
    /// Archive a session
    async fn archive_session(&self, id: &Uuid) -> Result<()>;
    
    /// Delete a session and all related data
    async fn delete_session(&self, id: &Uuid) -> Result<()>;
    
    /// Get complete session data including audio, transcript, and analysis
    async fn get_complete_session(&self, id: &Uuid) -> Result<Option<CompleteSession>>;
    
    /// Search sessions by content
    async fn search_sessions(&self, query: &str, limit: Option<i64>) -> Result<Vec<Session>>;
}

/// Complete session data with all related information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteSession {
    pub session: Session,
    pub audio_file: Option<AudioFile>,
    pub transcript: Option<Transcript>,
    pub analysis: Option<AnalysisResult>,
    pub ideas: Vec<Idea>,
    pub tasks: Vec<Task>,
    pub structured_notes: Vec<StructuredNote>,
}

/// Session list response with pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListResponse {
    pub sessions: Vec<Session>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
    pub has_next: bool,
    pub has_previous: bool,
}

/// Idea management service
#[async_trait]
pub trait IdeaService: Send + Sync {
    /// Create a new idea
    async fn create_idea(&self, idea: &NewIdea) -> Result<Idea>;
    
    /// Get idea by ID
    async fn get_idea(&self, id: &Uuid) -> Result<Option<Idea>>;
    
    /// List ideas by analysis ID
    async fn list_ideas_by_analysis(&self, analysis_id: &Uuid) -> Result<Vec<Idea>>;
    
    /// Update idea
    async fn update_idea(
        &self,
        id: &Uuid,
        content: &str,
        category: Option<&str>,
        priority: i32,
    ) -> Result<Idea>;
    
    /// Delete idea
    async fn delete_idea(&self, id: &Uuid) -> Result<()>;
    
    /// Search ideas by content or category
    async fn search_ideas(&self, query: &str, category: Option<&str>) -> Result<Vec<Idea>>;
    
    /// Get ideas by category
    async fn get_ideas_by_category(&self, category: &str) -> Result<Vec<Idea>>;
}

/// Task management service
#[async_trait]
pub trait TaskService: Send + Sync {
    /// Create a new task
    async fn create_task(&self, task: &NewTask) -> Result<Task>;
    
    /// Get task by ID
    async fn get_task(&self, id: &Uuid) -> Result<Option<Task>>;
    
    /// List tasks by analysis ID
    async fn list_tasks_by_analysis(&self, analysis_id: &Uuid) -> Result<Vec<Task>>;
    
    /// Update task
    async fn update_task(&self, id: &Uuid, updates: &TaskUpdate) -> Result<Task>;
    
    /// Delete task
    async fn delete_task(&self, id: &Uuid) -> Result<()>;
    
    /// Mark task as completed
    async fn complete_task(&self, id: &Uuid) -> Result<Task>;
    
    /// Get tasks by status
    async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;
    
    /// Get tasks by priority
    async fn get_tasks_by_priority(&self, priority: Priority) -> Result<Vec<Task>>;
    
    /// Search tasks by title or description
    async fn search_tasks(&self, query: &str) -> Result<Vec<Task>>;
}

/// Structured note management service
#[async_trait]
pub trait StructuredNoteService: Send + Sync {
    /// Create a new structured note
    async fn create_note(&self, note: &NewStructuredNote) -> Result<StructuredNote>;
    
    /// Get structured note by ID
    async fn get_note(&self, id: &Uuid) -> Result<Option<StructuredNote>>;
    
    /// List structured notes by analysis ID
    async fn list_notes_by_analysis(&self, analysis_id: &Uuid) -> Result<Vec<StructuredNote>>;
    
    /// Update structured note
    async fn update_note(&self, id: &Uuid, updates: &StructuredNoteUpdate) -> Result<StructuredNote>;
    
    /// Delete structured note
    async fn delete_note(&self, id: &Uuid) -> Result<()>;
    
    /// Get structured notes by type
    async fn get_notes_by_type(&self, note_type: NoteType) -> Result<Vec<StructuredNote>>;
    
    /// Search structured notes by tags
    async fn search_notes_by_tags(&self, tags: &[String]) -> Result<Vec<StructuredNote>>;
    
    /// Search structured notes by content
    async fn search_notes_by_content(&self, query: &str) -> Result<Vec<StructuredNote>>;
}

/// Ollama service for local AI model integration
#[async_trait]
pub trait OllamaService: Send + Sync {
    /// Check if Ollama service is available
    async fn is_available(&self) -> bool;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<OllamaModel>>;
    
    /// Pull/download a model
    async fn pull_model(&self, model_name: &str) -> Result<()>;
    
    /// Generate text completion
    async fn generate(
        &self,
        model: &str,
        prompt: &str,
        options: Option<OllamaOptions>,
    ) -> Result<String>;
    
    /// Generate structured response (JSON)
    async fn generate_structured<T>(
        &self,
        model: &str,
        prompt: &str,
        options: Option<OllamaOptions>,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;
    
    /// Get model information
    async fn get_model_info(&self, model_name: &str) -> Result<Option<OllamaModel>>;
}

/// Ollama model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: i64,
    pub digest: String,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub details: Option<OllamaModelDetails>,
}

/// Ollama model details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelDetails {
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub parameter_size: String,
    pub quantization_level: String,
}

/// Ollama generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaOptions {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
    pub repeat_penalty: Option<f32>,
    pub seed: Option<i32>,
    pub num_predict: Option<i32>,
    pub stop: Option<Vec<String>>,
}

/// File storage service for managing audio files
#[async_trait]
pub trait FileStorageService: Send + Sync {
    /// Store audio file and return file path
    async fn store_audio_file(
        &self,
        session_id: &Uuid,
        file_data: &[u8],
        filename: &str,
        format: &str,
    ) -> Result<String>;
    
    /// Get audio file data
    async fn get_audio_file(&self, file_path: &str) -> Result<Vec<u8>>;
    
    /// Delete audio file
    async fn delete_audio_file(&self, file_path: &str) -> Result<()>;
    
    /// Check if file exists
    async fn file_exists(&self, file_path: &str) -> bool;
    
    /// Get file size
    async fn get_file_size(&self, file_path: &str) -> Result<i64>;
    
    /// Calculate file checksum
    async fn calculate_checksum(&self, file_path: &str) -> Result<String>;
    
    /// Get storage statistics
    async fn get_storage_stats(&self) -> Result<StorageStats>;
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_files: i64,
    pub total_size_bytes: i64,
    pub available_space_bytes: i64,
    pub used_space_bytes: i64,
}

/// Configuration service for managing application settings
#[async_trait]
pub trait ConfigService: Send + Sync {
    /// Get configuration value by key
    async fn get_config(&self, key: &str) -> Result<Option<String>>;
    
    /// Set configuration value
    async fn set_config(&self, key: &str, value: &str) -> Result<()>;
    
    /// Get all configuration values
    async fn get_all_config(&self) -> Result<std::collections::HashMap<String, String>>;
    
    /// Delete configuration value
    async fn delete_config(&self, key: &str) -> Result<()>;
    
    /// Get typed configuration value
    async fn get_typed_config<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>;
    
    /// Set typed configuration value
    async fn set_typed_config<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize;
}