// src/services/mod.rs
//! Service layer module
//!
//! This module contains the business logic layer that sits between the API and repository layers.
//! Services encapsulate business rules, coordinate between repositories, and provide
//! high-level operations for the application.

pub mod traits;
pub mod implementations;
pub mod ollama;
pub mod audio;
pub mod transcription;
pub mod analysis;
pub mod session;
pub mod file_storage;

// Re-export commonly used types and traits
pub use traits::*;
pub use implementations::*;

use std::sync::Arc;
use crate::repository::RepositoryManager;

/// Service manager that provides access to all services
pub struct ServiceManager<R: RepositoryManager> {
    repository_manager: Arc<R>,
    audio_service: Arc<dyn AudioService>,
    transcription_service: Arc<dyn TranscriptionService>,
    analysis_service: Arc<dyn AnalysisService>,
    session_service: Arc<dyn SessionService>,
    idea_service: Arc<dyn IdeaService>,
    task_service: Arc<dyn TaskService>,
    structured_note_service: Arc<dyn StructuredNoteService>,
    ollama_service: Arc<dyn OllamaService>,
    file_storage_service: Arc<dyn FileStorageService>,
    config_service: Arc<dyn ConfigService>,
}

impl<R: RepositoryManager + 'static> ServiceManager<R> {
    /// Create a new service manager with the given repository manager
    pub fn new(repository_manager: Arc<R>, config: &crate::config::Config) -> Self {
        let file_storage_service = Arc::new(
            file_storage::LocalFileStorageService::new(&config.storage.audio_directory)
        );
        
        let ollama_service = Arc::new(
            ollama::OllamaServiceImpl::new(&config.ollama.base_url)
        );
        
        let audio_service = Arc::new(
            audio::AudioServiceImpl::new(
                repository_manager.clone(),
                file_storage_service.clone(),
            )
        );
        
        let transcription_service = Arc::new(
            transcription::TranscriptionServiceImpl::new(
                repository_manager.clone(),
                &config.openai.api_key,
            )
        );
        
        let analysis_service = Arc::new(
            analysis::AnalysisServiceImpl::new(
                repository_manager.clone(),
                ollama_service.clone(),
                &config.analysis.default_model,
            )
        );
        
        let session_service = Arc::new(
            session::SessionServiceImpl::new(repository_manager.clone())
        );
        
        let idea_service = Arc::new(
            implementations::IdeaServiceImpl::new(repository_manager.clone())
        );
        
        let task_service = Arc::new(
            implementations::TaskServiceImpl::new(repository_manager.clone())
        );
        
        let structured_note_service = Arc::new(
            implementations::StructuredNoteServiceImpl::new(repository_manager.clone())
        );
        
        let config_service = Arc::new(
            implementations::ConfigServiceImpl::new()
        );
        
        Self {
            repository_manager,
            audio_service,
            transcription_service,
            analysis_service,
            session_service,
            idea_service,
            task_service,
            structured_note_service,
            ollama_service,
            file_storage_service,
            config_service,
        }
    }
    
    /// Get audio service
    pub fn audio(&self) -> &dyn AudioService {
        self.audio_service.as_ref()
    }
    
    /// Get transcription service
    pub fn transcription(&self) -> &dyn TranscriptionService {
        self.transcription_service.as_ref()
    }
    
    /// Get analysis service
    pub fn analysis(&self) -> &dyn AnalysisService {
        self.analysis_service.as_ref()
    }
    
    /// Get session service
    pub fn sessions(&self) -> &dyn SessionService {
        self.session_service.as_ref()
    }
    
    /// Get idea service
    pub fn ideas(&self) -> &dyn IdeaService {
        self.idea_service.as_ref()
    }
    
    /// Get task service
    pub fn tasks(&self) -> &dyn TaskService {
        self.task_service.as_ref()
    }
    
    /// Get structured note service
    pub fn structured_notes(&self) -> &dyn StructuredNoteService {
        self.structured_note_service.as_ref()
    }
    
    /// Get Ollama service
    pub fn ollama(&self) -> &dyn OllamaService {
        self.ollama_service.as_ref()
    }
    
    /// Get file storage service
    pub fn file_storage(&self) -> &dyn FileStorageService {
        self.file_storage_service.as_ref()
    }
    
    /// Get configuration service
    pub fn config(&self) -> &dyn ConfigService {
        self.config_service.as_ref()
    }
    
    /// Get repository manager
    pub fn repositories(&self) -> &R {
        self.repository_manager.as_ref()
    }
}

/// Service factory for creating service instances
pub struct ServiceFactory;

impl ServiceFactory {
    /// Create a new service manager with PostgreSQL repositories
    pub fn create_postgres_service_manager(
        database_url: &str,
        config: &crate::config::Config,
    ) -> anyhow::Result<ServiceManager<crate::repository::PostgresRepositoryManager>> {
        use sqlx::PgPool;
        
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
        
        let repository_manager = Arc::new(
            crate::repository::PostgresRepositoryManager::new(pool)
        );
        
        Ok(ServiceManager::new(repository_manager, config))
    }
}