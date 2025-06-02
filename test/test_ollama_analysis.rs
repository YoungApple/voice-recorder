// Test program to call Ollama analysis with session 61a0f530-6db8-496b-b251-78c90966f071 transcript
// This demonstrates how to construct and send requests to Ollama for analysis

use std::fs;
use tokio;
use anyhow::Result;

// Import the necessary modules from the voice-recorder crate
mod config;
mod storage;
mod ollama;

use crate::ollama::analyze_with_ollama;
use crate::storage::AnalysisResult;

#[tokio::main]
async fn main() -> Result<()> {
    // Read the transcript from the audio file
    let transcript_path = "local_storage/app_data/audio/61a0f530-6db8-496b-b251-78c90966f071.wav.txt";
    let transcript = fs::read_to_string(transcript_path)
        .expect("Failed to read transcript file");
    
    println!("[Test] Loaded transcript from session 61a0f530-6db8-496b-b251-78c90966f071");
    println!("[Test] Transcript length: {} characters", transcript.len());
    println!("[Test] Transcript content:");
    println!("{}", transcript);
    println!("\n" + &"=".repeat(80));
    
    // Ollama configuration
    let ollama_endpoint = "http://localhost:11434/api/chat";
    let model_name = "deepseek-r1:8b-0528-qwen3-fp16";
    
    println!("[Test] Using Ollama endpoint: {}", ollama_endpoint);
    println!("[Test] Using model: {}", model_name);
    println!("\n" + &"=".repeat(80));
    
    // Call the analyze_with_ollama function
    match analyze_with_ollama(&transcript, ollama_endpoint, model_name).await {
        Ok(analysis_result) => {
            println!("[Test] Analysis completed successfully!");
            println!("\n" + &"=".repeat(80));
            
            // Print the analysis result
            println!("[Test] Analysis Result:");
            println!("Title: {}", analysis_result.title);
            println!("Summary: {}", analysis_result.summary);
            
            println!("\nIdeas ({} items):", analysis_result.ideas.len());
            for (i, idea) in analysis_result.ideas.iter().enumerate() {
                println!("  {}. {}", i + 1, idea);
            }
            
            println!("\nTasks ({} items):", analysis_result.tasks.len());
            for (i, task) in analysis_result.tasks.iter().enumerate() {
                println!("  {}. {} (Priority: {:?})", i + 1, task.title, task.priority);
                if let Some(description) = &task.description {
                    println!("     Description: {}", description);
                }
            }
            
            println!("\nStructured Notes ({} items):", analysis_result.structured_notes.len());
            for (i, note) in analysis_result.structured_notes.iter().enumerate() {
                println!("  {}. {} (Type: {:?})", i + 1, note.title, note.note_type);
                println!("     Content: {}", note.content);
                println!("     Tags: {:?}", note.tags);
            }
            
            println!("\n" + &"=".repeat(80));
            
            // Simulate saving the session with analysis result
            let session_id = "61a0f530-6db8-496b-b251-78c90966f071";
            println!("[Test] Would save session {} with analysis result", session_id);
            
            // Create a VoiceSession object to demonstrate the complete flow
            let voice_session = storage::VoiceSession {
                id: session_id.to_string(),
                audio_file_path: format!("local_storage/app_data/audio/{}.wav", session_id),
                transcript: Some(transcript),
                analysis: Some(analysis_result),
                created_at: chrono::Utc::now(),
                duration_seconds: None, // Would be calculated from audio file
            };
            
            println!("[Test] Created VoiceSession object:");
            println!("  ID: {}", voice_session.id);
            println!("  Audio file: {}", voice_session.audio_file_path);
            println!("  Has transcript: {}", voice_session.transcript.is_some());
            println!("  Has analysis: {}", voice_session.analysis.is_some());
            println!("  Created at: {}", voice_session.created_at);
            
            // Note: We don't actually save to avoid overwriting existing data
            println!("[Test] Session would be saved to: local_storage/app_data/sessions/{}.json", session_id);
        }
        Err(e) => {
            println!("[Test] Analysis failed with error: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}