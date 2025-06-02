// src/repository/mod.rs
//! Repository module for data access layer
//!
//! This module provides the data access layer abstraction for the voice recorder application.
//! It includes trait definitions and implementations for all data entities.

pub mod traits;
pub mod postgres;

// Re-export commonly used types and traits
pub use traits::*;
pub use postgres::PostgresRepositoryManager;

/// Repository manager trait that provides access to all repositories
pub trait RepositoryManager: Send + Sync {
    type SessionRepo: SessionRepository;
    type AudioRepo: AudioRepository;
    type TranscriptRepo: TranscriptRepository;
    type AnalysisRepo: AnalysisRepository;
    type IdeaRepo: IdeaRepository;
    type TaskRepo: TaskRepository;
    type StructuredNoteRepo: StructuredNoteRepository;

    /// Get session repository
    fn sessions(&self) -> &Self::SessionRepo;
    
    /// Get audio repository
    fn audio_files(&self) -> &Self::AudioRepo;
    
    /// Get transcript repository
    fn transcripts(&self) -> &Self::TranscriptRepo;
    
    /// Get analysis repository
    fn analysis_results(&self) -> &Self::AnalysisRepo;
    
    /// Get idea repository
    fn ideas(&self) -> &Self::IdeaRepo;
    
    /// Get task repository
    fn tasks(&self) -> &Self::TaskRepo;
    
    /// Get structured note repository
    fn structured_notes(&self) -> &Self::StructuredNoteRepo;
}