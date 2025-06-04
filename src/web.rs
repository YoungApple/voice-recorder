use anyhow::Result;
use axum::extract::{Path, State, Query, Multipart};
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::{get, post, delete};
use axum::Router;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;
use tower_http::cors::CorsLayer;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use axum::body::{Body, Bytes};
use uuid::Uuid;
use chrono::Utc;

use crate::audio::VoiceRecorder;
use crate::config::LegacyConfig;
use crate::storage::{self, VoiceSession};

#[derive(Debug, Deserialize)]
struct SessionQuery {
    search: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    data: T,
    message: Option<String>,
    error: Option<String>,
}

pub async fn start_server(port: u16, recorder: Arc<AsyncMutex<VoiceRecorder>>) -> Result<()> {
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/sessions", get(list_sessions_handler))
        .route("/api/sessions/:id", get(get_session_handler))
        .route("/api/sessions/:id", delete(delete_session_handler))
        .route("/api/sessions/:id/export", get(export_session_handler))
        .route("/api/sessions/:id/audio", get(audio_handler))
        .route("/api/sessions/:id/transcript", get(get_transcript_handler))
        .route("/api/sessions/:id/analysis", get(get_analysis_handler))
        .route("/api/config", get(get_config_handler))
        .route("/api/record/start", post(start_record_handler))
        .route("/api/record/stop", post(stop_record_handler))
        .route("/api/record/status", get(record_status_handler))
        .route("/api/sessions/upload", post(upload_audio_handler))
        .with_state(recorder)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Web interface available at http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index_handler() -> impl IntoResponse {
    Html(include_str!("../web/index.html"))
}

async fn list_sessions_handler(
    Query(query): Query<SessionQuery>
) -> Result<Json<ApiResponse<Vec<VoiceSession>>>, StatusCode> {
    match storage::list_sessions().await {
        Ok(mut sessions) => {
            // Apply search filter if provided
            if let Some(search) = query.search {
                sessions.retain(|s| {
                    s.title.to_lowercase().contains(&search.to_lowercase()) ||
                    s.transcript.as_ref().map_or(false, |t| t.to_lowercase().contains(&search.to_lowercase()))
                });
            }

            // Apply sorting
            if let Some(sort_by) = query.sort_by {
                match sort_by.as_str() {
                    "title" => {
                        sessions.sort_by(|a, b| {
                            let order = query.sort_order.as_deref().unwrap_or("asc");
                            if order == "desc" {
                                b.title.cmp(&a.title)
                            } else {
                                a.title.cmp(&b.title)
                            }
                        });
                    }
                    "timestamp" => {
                        sessions.sort_by(|a, b| {
                            let order = query.sort_order.as_deref().unwrap_or("desc");
                            if order == "asc" {
                                a.timestamp.cmp(&b.timestamp)
                            } else {
                                b.timestamp.cmp(&a.timestamp)
                            }
                        });
                    }
                    _ => {}
                }
            }

            // Apply pagination
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(sessions.len());
            let paginated_sessions = sessions.into_iter()
                .skip(offset)
                .take(limit)
                .map(|mut s| {
                    s.audio_url = Some(format!("/api/sessions/{}/audio", s.id));
                    s
                })
                .collect();

            Ok(Json(ApiResponse {
                data: paginated_sessions,
                message: Some("Sessions retrieved successfully".to_string()),
                error: None,
            }))
        },
        Err(e) => {
            eprintln!("Failed to list sessions: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_session_handler(
    Path(id): Path<String>
) -> Result<Json<ApiResponse<VoiceSession>>, StatusCode> {
    match storage::get_session(&id).await {
        Ok(Some(mut session)) => {
            session.audio_url = Some(format!("/api/sessions/{}/audio", session.id));
            Ok(Json(ApiResponse {
                data: session,
                message: Some("Session retrieved successfully".to_string()),
                error: None,
            }))
        },
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            eprintln!("Failed to get session {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn delete_session_handler(
    Path(id): Path<String>
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match storage::delete_session(&id).await {
        Ok(_) => Ok(Json(ApiResponse {
            data: (),
            message: Some("Session deleted successfully".to_string()),
            error: None,
        })),
        Err(e) => {
            eprintln!("Failed to delete session {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn export_session_handler(
    Path(id): Path<String>,
    Query(format): Query<String>
) -> Result<Response, StatusCode> {
    match storage::get_session(&id).await {
        Ok(Some(session)) => {
            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&session)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok(Response::builder()
                        .header("Content-Type", "application/json")
                        .header("Content-Disposition", format!("attachment; filename=\"session_{}.json\"", id))
                        .body(Body::from(json))
                        .unwrap())
                },
                "txt" => {
                    let content = format!(
                        "Title: {}\nTimestamp: {}\nDuration: {}ms\n\nTranscript:\n{}\n\nAnalysis:\n{}",
                        session.title,
                        session.timestamp,
                        session.duration_ms,
                        session.transcript.unwrap_or_default(),
                        serde_json::to_string_pretty(&session.analysis).unwrap_or_default()
                    );
                    Ok(Response::builder()
                        .header("Content-Type", "text/plain")
                        .header("Content-Disposition", format!("attachment; filename=\"session_{}.txt\"", id))
                        .body(Body::from(content))
                        .unwrap())
                },
                _ => Err(StatusCode::BAD_REQUEST)
            }
        },
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            eprintln!("Failed to export session {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn audio_handler(Path(id): Path<String>) -> Result<Response, StatusCode> {
    let storage_dir = crate::config::get_storage_dir();
    let audio_file_path = storage_dir.join("audio").join(format!("{}.wav", id));

    if !audio_file_path.as_path().exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    match File::open(&audio_file_path).await {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            if let Err(e) = file.read_to_end(&mut buffer).await {
                eprintln!("Failed to read audio file {}: {:?}", id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Ok(Response::builder()
                .header("Content-Type", "audio/wav")
                .header("Content-Disposition", format!("inline; filename=\"session_{}.wav\"", id))
                .body(Body::from(Bytes::from(buffer)))
                .unwrap())
        },
        Err(e) => {
            eprintln!("Failed to open audio file {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_transcript_handler(
    Path(id): Path<String>
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match storage::get_session(&id).await {
        Ok(Some(session)) => {
            Ok(Json(ApiResponse {
                data: session.transcript.unwrap_or_default(),
                message: Some("Transcript retrieved successfully".to_string()),
                error: None,
            }))
        },
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            eprintln!("Failed to get transcript for session {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_analysis_handler(
    Path(id): Path<String>
) -> Result<Json<ApiResponse<Option<crate::storage::AnalysisResult>>>, StatusCode> {
    match storage::get_session(&id).await {
        Ok(Some(session)) => {
            Ok(Json(ApiResponse {
                data: session.analysis,
                message: Some("Analysis retrieved successfully".to_string()),
                error: None,
            }))
        },
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            eprintln!("Failed to get analysis for session {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_config_handler() -> Result<Json<ApiResponse<LegacyConfig>>, StatusCode> {
    match crate::config::load_config().await {
        Ok(config) => Ok(Json(ApiResponse {
            data: config,
            message: Some("Configuration retrieved successfully".to_string()),
            error: None,
        })),
        Err(e) => {
            eprintln!("Failed to load config: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}



async fn start_record_handler(
    State(recorder): State<Arc<AsyncMutex<VoiceRecorder>>>
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut guard = recorder.lock().await;
    match guard.start_recording().await {
        Ok(_) => Ok(Json(ApiResponse {
            data: (),
            message: Some("Recording started successfully".to_string()),
            error: None,
        })),
        Err(e) => {
            eprintln!("Failed to start recording from web: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn stop_record_handler(
    State(recorder): State<Arc<AsyncMutex<VoiceRecorder>>>
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut guard = recorder.lock().await;
    match guard.stop_recording().await {
        Ok(_) => Ok(Json(ApiResponse {
            data: (),
            message: Some("Recording stopped successfully".to_string()),
            error: None,
        })),
        Err(e) => {
            eprintln!("Failed to stop recording from web: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn record_status_handler(
    State(recorder): State<Arc<AsyncMutex<VoiceRecorder>>>
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    let guard = recorder.lock().await;
    let is_recording = guard.is_recording();
    Ok(Json(ApiResponse {
        data: is_recording,
        message: Some(if is_recording { "Recording in progress" } else { "Not recording" }.to_string()),
        error: None,
    }))
}

/**
 * Handle audio file upload and processing
 * Creates a new voice session, processes the audio file, and returns the session data
 */
async fn upload_audio_handler(
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<VoiceSession>>, StatusCode> {
    println!("[DEBUG] Starting audio upload process");
    
    let mut audio_data: Option<Bytes> = None;
    let mut filename: Option<String> = None;

    // Process multipart form data
    println!("[DEBUG] Processing multipart form data");
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        eprintln!("[ERROR] Failed to get next multipart field: {:?}", e);
        StatusCode::BAD_REQUEST
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        println!("[DEBUG] Processing field: {}", field_name);
        
        if field_name == "audio" {
            filename = field.file_name().map(|s| s.to_string());
            println!("[DEBUG] Found audio field, filename: {:?}", filename);
            
            match field.bytes().await {
                Ok(bytes) => {
                    println!("[DEBUG] Successfully read audio data, size: {} bytes", bytes.len());
                    audio_data = Some(bytes);
                },
                Err(e) => {
                    eprintln!("[ERROR] Failed to read audio field bytes: {:?}", e);
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
    }

    let audio_bytes = match audio_data {
        Some(bytes) => {
            println!("[DEBUG] Audio data extracted successfully, size: {} bytes", bytes.len());
            bytes
        },
        None => {
            eprintln!("[ERROR] No audio data found in multipart form");
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    
    // Generate unique session ID
    let session_id = Uuid::new_v4().to_string();
    println!("[DEBUG] Generated session ID: {}", session_id);
    
    // Create storage directories
    let storage_dir = crate::config::get_storage_dir();
    let audio_dir = storage_dir.join("audio");
    println!("[DEBUG] Storage directory: {:?}", storage_dir);
    println!("[DEBUG] Audio directory: {:?}", audio_dir);
    
    match tokio::fs::create_dir_all(&audio_dir).await {
        Ok(_) => println!("[DEBUG] Audio directory created/verified successfully"),
        Err(e) => {
            eprintln!("[ERROR] Failed to create audio directory: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    // Save audio file
    let audio_filename = format!("{}.wav", session_id);
    let audio_file_path = audio_dir.join(&audio_filename);
    println!("[DEBUG] Audio file path: {:?}", audio_file_path);
    
    let mut file = match tokio::fs::File::create(&audio_file_path).await {
        Ok(file) => {
            println!("[DEBUG] Audio file created successfully");
            file
        },
        Err(e) => {
            eprintln!("[ERROR] Failed to create audio file: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    match file.write_all(&audio_bytes).await {
        Ok(_) => println!("[DEBUG] Audio data written to file successfully"),
        Err(e) => {
            eprintln!("[ERROR] Failed to write audio data to file: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    // Create voice session
    println!("[DEBUG] Creating voice session object");
    let mut session = VoiceSession {
        id: session_id.clone(),
        timestamp: Utc::now(),
        audio_file_path: audio_file_path.clone(),
        transcript: None,
        analysis: None,
        title: "Processing...".to_string(),
        duration_ms: 0,
        audio_url: Some(format!("/api/sessions/{}/audio", session_id)),
    };
    println!("[DEBUG] Voice session created with ID: {}", session.id);
    
    // Process audio file
    println!("[DEBUG] Starting audio transcription for file: {:?}", audio_file_path);
    match crate::ai::transcribe_audio(&audio_file_path).await {
        Ok(transcript) => {
            println!("[DEBUG] Audio transcription successful, transcript length: {} characters", transcript.len());
            println!("[DEBUG] Transcript preview: {}", 
                if transcript.len() > 100 { 
                    format!("{}...", &transcript[..100]) 
                } else { 
                    transcript.clone() 
                }
            );
            session.transcript = Some(transcript.clone());
            
            // Analyze transcript
            println!("[DEBUG] Starting transcript analysis");
            match crate::ai::analyze_transcript(&transcript).await {
                Ok(analysis) => {
                    println!("[DEBUG] Transcript analysis successful");
                    println!("[DEBUG] Analysis contains: {} ideas, {} tasks, {} structured notes", 
                        analysis.ideas.len(), analysis.tasks.len(), analysis.structured_notes.len());
                    
                    // Generate title from analysis
                    let title = if let Some(first_idea) = analysis.ideas.first() {
                        println!("[DEBUG] Using first idea as title: {}", first_idea);
                        first_idea.clone()
                    } else if !analysis.tasks.is_empty() {
                        println!("[DEBUG] Using first task title: {}", analysis.tasks[0].title);
                        analysis.tasks[0].title.clone()
                    } else if !analysis.structured_notes.is_empty() {
                        println!("[DEBUG] Using first structured note title: {}", analysis.structured_notes[0].title);
                        analysis.structured_notes[0].title.clone()
                    } else {
                        println!("[DEBUG] No specific content found, using default title");
                        "Voice Note".to_string()
                    };
                    
                    session.title = title.clone();
                    session.analysis = Some(analysis.clone());
                    println!("[DEBUG] Session title set to: {}", title);
                    
                    // Save session with analysis
                    println!("[DEBUG] Saving session with analysis");
                    if let Err(e) = crate::storage::save_session(&mut session, Some(analysis)).await {
                        eprintln!("[ERROR] Failed to save session with analysis: {:?}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                    println!("[DEBUG] Session saved successfully with analysis");
                },
                Err(e) => {
                    eprintln!("[ERROR] Failed to analyze transcript: {:?}", e);
                    session.title = "Voice Note".to_string();
                    println!("[DEBUG] Set default title due to analysis failure");
                    
                    // Save session without analysis
                    println!("[DEBUG] Saving session without analysis due to analysis failure");
                    if let Err(e) = crate::storage::save_session(&mut session, None).await {
                        eprintln!("[ERROR] Failed to save session without analysis: {:?}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                    println!("[DEBUG] Session saved successfully without analysis");
                }
            }
        },
        Err(e) => {
            eprintln!("[ERROR] Failed to transcribe audio: {:?}", e);
            session.title = "Voice Note".to_string();
            println!("[DEBUG] Set default title due to transcription failure");
            
            // Save session without transcript
            println!("[DEBUG] Saving session without transcript due to transcription failure");
            if let Err(e) = crate::storage::save_session(&mut session, None).await {
                eprintln!("[ERROR] Failed to save session without transcript: {:?}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            println!("[DEBUG] Session saved successfully without transcript");
        }
    }
    
    println!("[DEBUG] Audio upload and processing completed successfully");
    println!("[DEBUG] Final session - ID: {}, Title: {}, Has transcript: {}, Has analysis: {}", 
        session.id, session.title, session.transcript.is_some(), session.analysis.is_some());
    
    Ok(Json(ApiResponse {
        data: session,
        message: Some("Audio uploaded and processed successfully".to_string()),
        error: None,
    }))
}