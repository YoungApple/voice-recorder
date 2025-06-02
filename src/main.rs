// src/main.rs
//! 典型非Send字段示例
//! ```
//! raw_ptr: *mut c_void   // 原始指针
//! os_handle: Handle       // 操作系统资源句柄
//! ```

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::{info, warn/*, error*/};

mod ai;
mod audio;
mod config;
mod keyboard;
mod storage;
mod web;
mod ollama;
mod backfill;

#[derive(Parser)]
#[command(name = "voice-recorder")]
#[command(about = "A voice recording and AI analysis tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the voice recorder application
    Start,
    /// Transcribe an audio file
    Transcribe { 
        #[arg(short, long)]
        file: String,
    },
    /// Analyze a transcript
    Analyze { 
        #[arg(short, long)]
        file: String,
    },
    /// Play an audio file
    Play { 
        #[arg(short, long)]
        file: String,
    },
    /// List all recorded sessions
    List,
    /// Show details of a specific session
    Show { 
        #[arg(short, long)]
        id: String,
    },
    /// Delete a specific session
    Delete { 
        #[arg(short, long)]
        id: String,
    },
    /// Export a specific session
    Export { 
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        format: String,
    },
    /// Configure the application
    Config,
    /// Test Ollama analysis with a specific session
    TestOllama { 
        #[arg(short, long)]
        id: String,
    },
    /// Start the web interface
    Web { 
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// Backfill missing transcripts and analysis for all sessions
    Backfill,
}

// #[derive(Subcommand)]
// enum ConfigCommands {
//     /// Set OpenAI API key
//     SetOpenai { api_key: String },
//     /// Set local model endpoint
//     SetLocal { endpoint: String },
//     /// Show current configuration
//     Show,
// }

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting voice-recorder application...");

    let cli = Cli::parse();

    match &cli.command {
        Commands::Start => {
            info!("Starting application...");
            let recorder = Arc::new(tokio::sync::Mutex::new(audio::VoiceRecorder::new().await?));
            let mut keyboard_handler = keyboard::KeyboardHandler::new(recorder.clone());
            keyboard_handler.start_listening()?.await;
        }
        Commands::Transcribe { file } => {
            info!("Transcribing file: {}", file);
            let audio_path = std::path::PathBuf::from(file);
            let transcript = ai::transcribe_audio(&audio_path).await?;
            info!("Transcript: {}", transcript);
        }
        Commands::Analyze { file } => {
            info!("Analyzing file: {}", file);
            let transcript = tokio::fs::read_to_string(file).await?;
            let analysis = ai::analyze_transcript(&transcript).await?;
            info!("Analysis: {:#?}", analysis);
        }
        Commands::Play { file } => {
            info!("Playing file: {}", file);
            audio::VoiceRecorder::new().await?.play_audio_file(file).await?;
        }
        Commands::List => {
            info!("Listing sessions...");
            let sessions = storage::list_sessions().await?;
            for session in sessions {
                info!("Session ID: {}, Title: {}, Created: {}", session.id, session.title, session.timestamp);
            }
        }
        Commands::Show { id } => {
            info!("Showing session: {}", id);
            if let Some(session) = storage::get_session(&id).await? {
                info!("Session: {:#?}", session);
            } else {
                warn!("Session with ID {} not found.", id);
            }
        }
        Commands::Delete { id } => {
            info!("Deleting session: {}", id);
            storage::delete_session(&id).await?;
            info!("Session {} deleted.", id);
        }
        Commands::Export { id, format } => {
            info!("Exporting session {} in format {}", id, format);
            // Implement export logic here
            warn!("Export functionality not yet implemented.");
        }
        Commands::Config => {
            info!("Opening config file...");
            // Implement config opening logic here
            warn!("Config functionality not yet implemented.");
        }
        Commands::TestOllama { id } => {
            info!("Testing Ollama analysis for session: {}", id);
            if let Some(session) = storage::get_session(&id).await? {
                if let Some(transcript) = session.transcript {
                    info!("Transcript found for session {}. Analyzing with Ollama...", id);
                    let analysis = ai::analyze_transcript(&transcript).await?;
                    info!("Ollama Analysis Result: {:#?}", analysis);
                } else {
                    warn!("No transcript found for session {}. Cannot perform Ollama analysis.", id);
                }
            } else {
                warn!("Session with ID {} not found. Cannot perform Ollama analysis.", id);
            }
        }
        Commands::Web { port } => {
            info!("Starting web interface on port {}", port);
            let recorder = Arc::new(tokio::sync::Mutex::new(audio::VoiceRecorder::new().await?));
            web::start_server(*port, recorder).await?;
        }
        Commands::Backfill => {
            info!("Starting backfill process...");
            backfill::backfill_sessions().await?;
        }
    }

    Ok(())
}