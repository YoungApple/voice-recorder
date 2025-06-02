use anyhow::{Result, Context};
use chrono::Utc;
use log::{info, warn, error};
use std::path::PathBuf;
use tokio::fs;

use crate::storage::{VoiceSession, AnalysisResult};
use crate::ai::{transcribe_audio, analyze_transcript};

#[derive(Debug, Default)]
struct BackfillStats {
    total_sessions: usize,
    processed: usize,
    skipped: usize,
    transcript_generated: usize,
    analysis_generated: usize,
    errors: usize,
}

pub async fn backfill_sessions() -> Result<()> {
    let sessions = crate::storage::list_sessions().await
        .context("Failed to list sessions")?;
    
    let mut stats = BackfillStats {
        total_sessions: sessions.len(),
        ..Default::default()
    };
    
    info!("Starting backfill process for {} sessions", stats.total_sessions);

    for (index, mut session) in sessions.into_iter().enumerate() {
        let session_id = session.id.clone();
        info!("Processing session {}/{}: {}", index + 1, stats.total_sessions, session_id);
        
        if !session.audio_file_path.exists() {
            warn!("Audio file not found for session {}, skipping", session_id);
            stats.skipped += 1;
            continue;
        }

        let needs_transcript = session.transcript.is_none();
        let needs_analysis = session.analysis.is_none() || 
            (session.analysis.as_ref().map_or(true, |a| {
                let is_default_summary = a.summary == "No analysis performed." || 
                                       a.summary == "Ollama analysis skipped (disabled)." ||
                                       a.summary.is_empty();
                
                let is_default_title = a.title == "Untitled Note";
                
                let is_empty_analysis = a.ideas.is_empty() && 
                                      a.tasks.is_empty() && 
                                      a.structured_notes.is_empty();
                
                is_default_summary && (is_default_title || a.title.is_empty()) && is_empty_analysis
            }));

        let mut should_save = false;
        let mut session_error = false;

        // 处理 transcript
        if needs_transcript {
            info!("[{}] Generating transcript...", session_id);
            match transcribe_audio(&session.audio_file_path).await {
                Ok(transcript) => {
                    session.transcript = Some(transcript.clone());
                    info!("[{}] Successfully generated transcript ({} chars)", 
                          session_id, transcript.len());
                    stats.transcript_generated += 1;
                    should_save = true;
                }
                Err(e) => {
                    error!("[{}] Failed to generate transcript: {}", session_id, e);
                    stats.errors += 1;
                    session_error = true;
                }
            }
        }

        // 处理 analysis
        if !session_error && needs_analysis {
            info!("[{}] Generating analysis...", session_id);
            if let Some(transcript) = &session.transcript {
                match analyze_transcript(transcript).await {
                    Ok(analysis) => {
                        session.analysis = Some(analysis.clone());
                        if !analysis.title.is_empty() {
                            session.title = analysis.title.clone();
                        }
                        info!("[{}] Successfully generated analysis (title: {}, {} ideas, {} tasks)", 
                              session_id, 
                              analysis.title,
                              analysis.ideas.len(),
                              analysis.tasks.len());
                        stats.analysis_generated += 1;
                        should_save = true;
                    }
                    Err(e) => {
                        error!("[{}] Failed to generate analysis: {}", session_id, e);
                        stats.errors += 1;
                    }
                }
            } else {
                warn!("[{}] Cannot generate analysis without transcript", session_id);
                stats.skipped += 1;
            }
        } else if !needs_analysis {
            info!("[{}] Session is already complete, skipping", session_id);
            stats.skipped += 1;
        }

        // 保存更新后的 session
        if should_save {
            info!("[{}] Saving session...", session_id);
            let analysis_to_save = session.analysis.take();
            if let Err(e) = crate::storage::save_session(&mut session, analysis_to_save).await {
                error!("[{}] Failed to save session: {}", session_id, e);
                stats.errors += 1;
            } else {
                info!("[{}] Successfully saved session", session_id);
            }
        }

        stats.processed += 1;
        
        // 每处理10个session打印一次进度
        if stats.processed % 10 == 0 {
            info!("Progress: {}/{} sessions processed", stats.processed, stats.total_sessions);
        }
    }

    // 打印最终统计信息
    info!("Backfill completed with stats:");
    info!("  Total sessions: {}", stats.total_sessions);
    info!("  Processed: {}", stats.processed);
    info!("  Skipped: {}", stats.skipped);
    info!("  Transcripts generated: {}", stats.transcript_generated);
    info!("  Analysis generated: {}", stats.analysis_generated);
    info!("  Errors encountered: {}", stats.errors);

    if stats.errors > 0 {
        warn!("Backfill completed with {} errors", stats.errors);
    } else {
        info!("Backfill completed successfully");
    }

    Ok(())
} 