// src/storage.rs
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSession {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub audio_file_path: PathBuf,
    pub transcript: Option<String>,
    pub analysis: Option<AnalysisResult>,
    pub title: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AnalysisResult {
    pub title: String,
    pub ideas: Vec<String>,
    pub tasks: Vec<Task>,
    pub structured_notes: Vec<StructuredNote>,
    pub summary: String,
}

impl AnalysisResult {
    pub fn default_with_summary(summary: String) -> Self {
        AnalysisResult {
            title: "Untitled Note".to_string(),
            ideas: Vec::new(),
            tasks: Vec::new(),
            structured_notes: Vec::new(),
            summary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredNote {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub note_type: NoteType,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) created_at: DateTime<Utc>,
    // Removed created_at and updated_at as they are not part of the struct definition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteType {
    Meeting,
    Brainstorm,
    Decision,
    Action,
    Reference,
}



pub async fn save_session(session: &mut VoiceSession, analysis_result: Option<AnalysisResult>) -> Result<()> {
    if let Some(analysis) = analysis_result {
        session.title = analysis.title.clone();
        session.analysis = Some(analysis);
    }
    let storage_dir = crate::config::get_storage_dir();
    let session_file = storage_dir.join("sessions").join(format!("{}.json", session.id));
    
    let content = serde_json::to_string_pretty(session)?;
    fs::write(session_file, content).await?;
    
    Ok(())
}

pub async fn get_session(id: &str) -> Result<Option<VoiceSession>> {
    let storage_dir = crate::config::get_storage_dir();
    let session_file = storage_dir.join("sessions").join(format!("{}.json", id));
    
    if !session_file.exists() {
        return Ok(None);
    }
    
    let content: String = fs::read_to_string(session_file).await?;
    let session: VoiceSession = serde_json::from_str(&content)?;
    
    Ok(Some(session))
}

pub async fn list_sessions() -> Result<Vec<VoiceSession>> {
    let storage_dir = crate::config::get_storage_dir();
    let sessions_dir = storage_dir.join("sessions");
    
    let mut sessions = Vec::new();
    let mut entries = fs::read_dir(sessions_dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            let content: String = fs::read_to_string(entry.path()).await?;
            if let Ok(session) = serde_json::from_str::<VoiceSession>(&content) {
                sessions.push(session);
            }
        }
    }
    
    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(sessions)
}

pub async fn delete_session(id: &str) -> Result<()> {
    let storage_dir = crate::config::get_storage_dir();
    let session_file = storage_dir.join("sessions").join(format!("{}.json", id));
    let audio_file = storage_dir.join("audio").join(format!("{}.wav", id));

    if session_file.exists() {
        fs::remove_file(session_file).await?;
    }
    if audio_file.exists() {
        fs::remove_file(audio_file).await?;
    }
    Ok(())
}

pub fn create_new_session() -> VoiceSession {
    let id = Uuid::new_v4().to_string();
    let storage_dir = crate::config::get_storage_dir();
    let audio_file_path = storage_dir.join("audio").join(format!("{}.wav", id));
    
    VoiceSession {
        id,
        timestamp: Utc::now(),
        audio_file_path,
        transcript: None,
        analysis: None,
        title: String::new(), // Initialize with an empty string
        duration_ms: 0,
        audio_url: None,
    }
}