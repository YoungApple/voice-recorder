// src/repository/postgres.rs
//! PostgreSQL implementation of repository traits
//!
//! This module provides concrete implementations of all repository traits using PostgreSQL
//! as the underlying database through sqlx.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};

use super::traits::*;

/// PostgreSQL session repository implementation
pub struct PostgresSessionRepository {
    pool: PgPool,
}

impl PostgresSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for PostgresSessionRepository {
    async fn create(&self, session: &NewSession) -> Result<Session> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = sqlx::query!(
            r#"
            INSERT INTO sessions (id, title, created_at, updated_at, duration_ms, status, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, title, created_at, updated_at, duration_ms, status as "status: SessionStatus", metadata
            "#,
            id,
            session.title,
            now,
            now,
            session.duration_ms,
            SessionStatus::Active as SessionStatus,
            session.metadata
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to create session")?;

        Ok(Session {
            id: row.id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
            duration_ms: row.duration_ms,
            status: row.status,
            metadata: row.metadata,
        })
    }

    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Session>> {
        let row = sqlx::query!(
            r#"
            SELECT id, title, created_at, updated_at, duration_ms, status as "status: SessionStatus", metadata
            FROM sessions
            WHERE id = $1 AND status != 'deleted'
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find session by id")?;

        Ok(row.map(|r| Session {
            id: r.id,
            title: r.title,
            created_at: r.created_at,
            updated_at: r.updated_at,
            duration_ms: r.duration_ms,
            status: r.status,
            metadata: r.metadata,
        }))
    }

    async fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>> {
        let mut query = "SELECT id, title, created_at, updated_at, duration_ms, status, metadata FROM sessions WHERE status != 'deleted'".to_string();
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 1;

        if let Some(search) = &filter.search {
            conditions.push(format!("title ILIKE ${}", param_count));
            params.push(Box::new(format!("%{}%", search)));
            param_count += 1;
        }

        if let Some(status) = &filter.status {
            conditions.push(format!("status = ${}", param_count));
            params.push(Box::new(status.clone()));
            param_count += 1;
        }

        if let Some(created_after) = &filter.created_after {
            conditions.push(format!("created_at >= ${}", param_count));
            params.push(Box::new(*created_after));
            param_count += 1;
        }

        if let Some(created_before) = &filter.created_before {
            conditions.push(format!("created_at <= ${}", param_count));
            params.push(Box::new(*created_before));
            param_count += 1;
        }

        if !conditions.is_empty() {
            query.push_str(" AND ");
            query.push_str(&conditions.join(" AND "));
        }

        // Add sorting
        let sort_column = match filter.sort_by {
            Some(SessionSortBy::CreatedAt) => "created_at",
            Some(SessionSortBy::UpdatedAt) => "updated_at",
            Some(SessionSortBy::Title) => "title",
            Some(SessionSortBy::Duration) => "duration_ms",
            None => "created_at",
        };

        let sort_order = match filter.sort_order {
            Some(SortOrder::Asc) => "ASC",
            Some(SortOrder::Desc) | None => "DESC",
        };

        query.push_str(&format!(" ORDER BY {} {}", sort_column, sort_order));

        // Add pagination
        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list sessions")?;

        let sessions = rows
            .into_iter()
            .map(|row| Session {
                id: row.get("id"),
                title: row.get("title"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                duration_ms: row.get("duration_ms"),
                status: row.get("status"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(sessions)
    }

    async fn update(&self, id: &Uuid, updates: &SessionUpdate) -> Result<Session> {
        let now = Utc::now();
        
        let row = sqlx::query!(
            r#"
            UPDATE sessions 
            SET title = COALESCE($2, title),
                status = COALESCE($3, status),
                metadata = COALESCE($4, metadata),
                updated_at = $5
            WHERE id = $1
            RETURNING id, title, created_at, updated_at, duration_ms, status as "status: SessionStatus", metadata
            "#,
            id,
            updates.title,
            updates.status.as_ref().map(|s| s.clone() as SessionStatus),
            updates.metadata,
            now
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to update session")?;

        Ok(Session {
            id: row.id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
            duration_ms: row.duration_ms,
            status: row.status,
            metadata: row.metadata,
        })
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let now = Utc::now();
        
        sqlx::query!(
            "UPDATE sessions SET status = 'deleted', updated_at = $2 WHERE id = $1",
            id,
            now
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete session")?;

        Ok(())
    }

    async fn count(&self, filter: &SessionFilter) -> Result<i64> {
        let mut query = "SELECT COUNT(*) FROM sessions WHERE status != 'deleted'".to_string();
        let mut conditions = Vec::new();

        if let Some(search) = &filter.search {
            conditions.push(format!("title ILIKE '%{}%'", search));
        }

        if let Some(status) = &filter.status {
            conditions.push(format!("status = '{}'", match status {
                SessionStatus::Active => "active",
                SessionStatus::Archived => "archived",
                SessionStatus::Deleted => "deleted",
            }));
        }

        if !conditions.is_empty() {
            query.push_str(" AND ");
            query.push_str(&conditions.join(" AND "));
        }

        let row = sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count sessions")?;

        Ok(row.get::<i64, _>(0))
    }

    async fn find_by_status(&self, status: SessionStatus) -> Result<Vec<Session>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, title, created_at, updated_at, duration_ms, status as "status: SessionStatus", metadata
            FROM sessions
            WHERE status = $1
            ORDER BY created_at DESC
            "#,
            status as SessionStatus
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to find sessions by status")?;

        let sessions = rows
            .into_iter()
            .map(|row| Session {
                id: row.id,
                title: row.title,
                created_at: row.created_at,
                updated_at: row.updated_at,
                duration_ms: row.duration_ms,
                status: row.status,
                metadata: row.metadata,
            })
            .collect();

        Ok(sessions)
    }
}

/// PostgreSQL audio repository implementation
pub struct PostgresAudioRepository {
    pool: PgPool,
}

impl PostgresAudioRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AudioRepository for PostgresAudioRepository {
    async fn create(&self, audio: &NewAudioFile) -> Result<AudioFile> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = sqlx::query!(
            r#"
            INSERT INTO audio_files (id, session_id, file_path, file_size, format, sample_rate, channels, created_at, checksum)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, session_id, file_path, file_size, format, sample_rate, channels, created_at, checksum
            "#,
            id,
            audio.session_id,
            audio.file_path,
            audio.file_size,
            audio.format,
            audio.sample_rate,
            audio.channels,
            now,
            audio.checksum
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to create audio file")?;

        Ok(AudioFile {
            id: row.id,
            session_id: row.session_id,
            file_path: row.file_path,
            file_size: row.file_size,
            format: row.format,
            sample_rate: row.sample_rate,
            channels: row.channels,
            created_at: row.created_at,
            checksum: row.checksum,
        })
    }

    async fn find_by_id(&self, id: &Uuid) -> Result<Option<AudioFile>> {
        let row = sqlx::query!(
            r#"
            SELECT id, session_id, file_path, file_size, format, sample_rate, channels, created_at, checksum
            FROM audio_files
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find audio file by id")?;

        Ok(row.map(|r| AudioFile {
            id: r.id,
            session_id: r.session_id,
            file_path: r.file_path,
            file_size: r.file_size,
            format: r.format,
            sample_rate: r.sample_rate,
            channels: r.channels,
            created_at: r.created_at,
            checksum: r.checksum,
        }))
    }

    async fn find_by_session_id(&self, session_id: &Uuid) -> Result<Option<AudioFile>> {
        let row = sqlx::query!(
            r#"
            SELECT id, session_id, file_path, file_size, format, sample_rate, channels, created_at, checksum
            FROM audio_files
            WHERE session_id = $1
            "#,
            session_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find audio file by session id")?;

        Ok(row.map(|r| AudioFile {
            id: r.id,
            session_id: r.session_id,
            file_path: r.file_path,
            file_size: r.file_size,
            format: r.format,
            sample_rate: r.sample_rate,
            channels: r.channels,
            created_at: r.created_at,
            checksum: r.checksum,
        }))
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM audio_files WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .context("Failed to delete audio file")?;

        Ok(())
    }

    async fn update_checksum(&self, id: &Uuid, checksum: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE audio_files SET checksum = $2 WHERE id = $1",
            id,
            checksum
        )
        .execute(&self.pool)
        .await
        .context("Failed to update audio file checksum")?;

        Ok(())
    }
}

/// PostgreSQL repository manager implementation
pub struct PostgresRepositoryManager {
    sessions: PostgresSessionRepository,
    audio_files: PostgresAudioRepository,
    transcripts: PostgresTranscriptRepository,
    analysis_results: PostgresAnalysisRepository,
    ideas: PostgresIdeaRepository,
    tasks: PostgresTaskRepository,
    structured_notes: PostgresStructuredNoteRepository,
}

impl PostgresRepositoryManager {
    pub fn new(pool: PgPool) -> Self {
        Self {
            sessions: PostgresSessionRepository::new(pool.clone()),
            audio_files: PostgresAudioRepository::new(pool.clone()),
            transcripts: PostgresTranscriptRepository::new(pool.clone()),
            analysis_results: PostgresAnalysisRepository::new(pool.clone()),
            ideas: PostgresIdeaRepository::new(pool.clone()),
            tasks: PostgresTaskRepository::new(pool.clone()),
            structured_notes: PostgresStructuredNoteRepository::new(pool),
        }
    }
}

impl super::RepositoryManager for PostgresRepositoryManager {
    type SessionRepo = PostgresSessionRepository;
    type AudioRepo = PostgresAudioRepository;
    type TranscriptRepo = PostgresTranscriptRepository;
    type AnalysisRepo = PostgresAnalysisRepository;
    type IdeaRepo = PostgresIdeaRepository;
    type TaskRepo = PostgresTaskRepository;
    type StructuredNoteRepo = PostgresStructuredNoteRepository;

    fn sessions(&self) -> &Self::SessionRepo {
        &self.sessions
    }

    fn audio_files(&self) -> &Self::AudioRepo {
        &self.audio_files
    }

    fn transcripts(&self) -> &Self::TranscriptRepo {
        &self.transcripts
    }

    fn analysis_results(&self) -> &Self::AnalysisRepo {
        &self.analysis_results
    }

    fn ideas(&self) -> &Self::IdeaRepo {
        &self.ideas
    }

    fn tasks(&self) -> &Self::TaskRepo {
        &self.tasks
    }

    fn structured_notes(&self) -> &Self::StructuredNoteRepo {
        &self.structured_notes
    }
}

// Placeholder implementations for other repositories
// These would be implemented similarly to the above

pub struct PostgresTranscriptRepository {
    pool: PgPool,
}

impl PostgresTranscriptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TranscriptRepository for PostgresTranscriptRepository {
    async fn create(&self, transcript: &NewTranscript) -> Result<Transcript> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = sqlx::query!(
            r#"
            INSERT INTO transcripts (id, session_id, content, language, confidence_score, provider, created_at, processing_time_ms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, session_id, content, language, confidence_score, provider, created_at, processing_time_ms
            "#,
            id,
            transcript.session_id,
            transcript.content,
            transcript.language,
            transcript.confidence_score,
            transcript.provider,
            now,
            transcript.processing_time_ms
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to create transcript")?;

        Ok(Transcript {
            id: row.id,
            session_id: row.session_id,
            content: row.content,
            language: row.language,
            confidence_score: row.confidence_score,
            provider: row.provider,
            created_at: row.created_at,
            processing_time_ms: row.processing_time_ms,
        })
    }

    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Transcript>> {
        let row = sqlx::query!(
            r#"
            SELECT id, session_id, content, language, confidence_score, provider, created_at, processing_time_ms
            FROM transcripts
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find transcript by id")?;

        Ok(row.map(|r| Transcript {
            id: r.id,
            session_id: r.session_id,
            content: r.content,
            language: r.language,
            confidence_score: r.confidence_score,
            provider: r.provider,
            created_at: r.created_at,
            processing_time_ms: r.processing_time_ms,
        }))
    }

    async fn find_by_session_id(&self, session_id: &Uuid) -> Result<Option<Transcript>> {
        let row = sqlx::query!(
            r#"
            SELECT id, session_id, content, language, confidence_score, provider, created_at, processing_time_ms
            FROM transcripts
            WHERE session_id = $1
            "#,
            session_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find transcript by session id")?;

        Ok(row.map(|r| Transcript {
            id: r.id,
            session_id: r.session_id,
            content: r.content,
            language: r.language,
            confidence_score: r.confidence_score,
            provider: r.provider,
            created_at: r.created_at,
            processing_time_ms: r.processing_time_ms,
        }))
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM transcripts WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .context("Failed to delete transcript")?;

        Ok(())
    }

    async fn find_by_provider(&self, provider: &str) -> Result<Vec<Transcript>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, session_id, content, language, confidence_score, provider, created_at, processing_time_ms
            FROM transcripts
            WHERE provider = $1
            ORDER BY created_at DESC
            "#,
            provider
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to find transcripts by provider")?;

        let transcripts = rows
            .into_iter()
            .map(|row| Transcript {
                id: row.id,
                session_id: row.session_id,
                content: row.content,
                language: row.language,
                confidence_score: row.confidence_score,
                provider: row.provider,
                created_at: row.created_at,
                processing_time_ms: row.processing_time_ms,
            })
            .collect();

        Ok(transcripts)
    }
}

// Placeholder structs for other repositories - these would be fully implemented
pub struct PostgresAnalysisRepository { pool: PgPool }
pub struct PostgresIdeaRepository { pool: PgPool }
pub struct PostgresTaskRepository { pool: PgPool }
pub struct PostgresStructuredNoteRepository { pool: PgPool }

impl PostgresAnalysisRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

impl PostgresIdeaRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

impl PostgresTaskRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

impl PostgresStructuredNoteRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

// Placeholder trait implementations - these would be fully implemented
#[async_trait]
impl AnalysisRepository for PostgresAnalysisRepository {
    async fn create(&self, _analysis: &NewAnalysisResult) -> Result<AnalysisResult> {
        todo!("Implement analysis repository create")
    }
    
    async fn find_by_id(&self, _id: &Uuid) -> Result<Option<AnalysisResult>> {
        todo!("Implement analysis repository find_by_id")
    }
    
    async fn find_by_session_id(&self, _session_id: &Uuid) -> Result<Option<AnalysisResult>> {
        todo!("Implement analysis repository find_by_session_id")
    }
    
    async fn update(&self, _id: &Uuid, _updates: &AnalysisUpdate) -> Result<AnalysisResult> {
        todo!("Implement analysis repository update")
    }
    
    async fn delete(&self, _id: &Uuid) -> Result<()> {
        todo!("Implement analysis repository delete")
    }
    
    async fn find_by_provider(&self, _provider: &str) -> Result<Vec<AnalysisResult>> {
        todo!("Implement analysis repository find_by_provider")
    }
}

#[async_trait]
impl IdeaRepository for PostgresIdeaRepository {
    async fn create(&self, _idea: &NewIdea) -> Result<Idea> {
        todo!("Implement idea repository create")
    }
    
    async fn find_by_id(&self, _id: &Uuid) -> Result<Option<Idea>> {
        todo!("Implement idea repository find_by_id")
    }
    
    async fn find_by_analysis_id(&self, _analysis_id: &Uuid) -> Result<Vec<Idea>> {
        todo!("Implement idea repository find_by_analysis_id")
    }
    
    async fn update(&self, _id: &Uuid, _content: &str, _category: Option<&str>, _priority: i32) -> Result<Idea> {
        todo!("Implement idea repository update")
    }
    
    async fn delete(&self, _id: &Uuid) -> Result<()> {
        todo!("Implement idea repository delete")
    }
    
    async fn find_by_category(&self, _category: &str) -> Result<Vec<Idea>> {
        todo!("Implement idea repository find_by_category")
    }
}

#[async_trait]
impl TaskRepository for PostgresTaskRepository {
    async fn create(&self, _task: &NewTask) -> Result<Task> {
        todo!("Implement task repository create")
    }
    
    async fn find_by_id(&self, _id: &Uuid) -> Result<Option<Task>> {
        todo!("Implement task repository find_by_id")
    }
    
    async fn find_by_analysis_id(&self, _analysis_id: &Uuid) -> Result<Vec<Task>> {
        todo!("Implement task repository find_by_analysis_id")
    }
    
    async fn update(&self, _id: &Uuid, _updates: &TaskUpdate) -> Result<Task> {
        todo!("Implement task repository update")
    }
    
    async fn delete(&self, _id: &Uuid) -> Result<()> {
        todo!("Implement task repository delete")
    }
    
    async fn find_by_status(&self, _status: TaskStatus) -> Result<Vec<Task>> {
        todo!("Implement task repository find_by_status")
    }
    
    async fn find_by_priority(&self, _priority: Priority) -> Result<Vec<Task>> {
        todo!("Implement task repository find_by_priority")
    }
    
    async fn mark_completed(&self, _id: &Uuid) -> Result<Task> {
        todo!("Implement task repository mark_completed")
    }
}

#[async_trait]
impl StructuredNoteRepository for PostgresStructuredNoteRepository {
    async fn create(&self, _note: &NewStructuredNote) -> Result<StructuredNote> {
        todo!("Implement structured note repository create")
    }
    
    async fn find_by_id(&self, _id: &Uuid) -> Result<Option<StructuredNote>> {
        todo!("Implement structured note repository find_by_id")
    }
    
    async fn find_by_analysis_id(&self, _analysis_id: &Uuid) -> Result<Vec<StructuredNote>> {
        todo!("Implement structured note repository find_by_analysis_id")
    }
    
    async fn update(&self, _id: &Uuid, _updates: &StructuredNoteUpdate) -> Result<StructuredNote> {
        todo!("Implement structured note repository update")
    }
    
    async fn delete(&self, _id: &Uuid) -> Result<()> {
        todo!("Implement structured note repository delete")
    }
    
    async fn find_by_note_type(&self, _note_type: NoteType) -> Result<Vec<StructuredNote>> {
        todo!("Implement structured note repository find_by_note_type")
    }
    
    async fn find_by_tags(&self, _tags: &[String]) -> Result<Vec<StructuredNote>> {
        todo!("Implement structured note repository find_by_tags")
    }
}