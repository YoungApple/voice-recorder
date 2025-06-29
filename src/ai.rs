// ai.rs
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use log::{info /* , warn, error */};
use std::fmt;

use crate::ollama::analyze_with_ollama_v2;
use crate::storage::{AnalysisResult, Task, Priority, StructuredNote, NoteType};

use crate::config::AiProvider;
use std::path::Path;
use std::process::{Command, Stdio};
use tokio::fs;
use chrono::Utc;

pub async fn transcribe_audio(audio_path: &Path) -> Result<String, anyhow::Error> {
    let config = crate::config::load_config().await?;

    match config.ai_provider {
        AiProvider::OpenAI => {
            if let Some(_api_key) = &config.api_keys.openai_api_key {
                transcribe_with_openai(audio_path, _api_key).await
            } else {
                // error!("OpenAI API key is not configured. Please set it in the config file.");
                Err(anyhow::anyhow!(
                    "OpenAI API key is not configured. Please set it in the config file."
                ))
            }
        }

        AiProvider::WhisperCpp => {
            if let (Some(model_path), Some(executable_path)) = (
                &config.speech_model.whisper_model_path,
                &config.speech_model.whisper_executable_path,
            ) {
                transcribe_with_whisper_cpp(audio_path, model_path, executable_path).await
            } else {
                // error!("Whisper.cpp model path or executable path not set in config.");
                Err(anyhow::anyhow!(
                    "Whisper.cpp model path or executable path not set in config."
                ))
            }
        }
        AiProvider::Ollama => {
            // error!("Transcription directly via Ollama provider is not implemented. Ollama is typically used for text analysis. Configure WhisperCpp for STT and Ollama as text_model for analysis.");
            Err(anyhow::anyhow!("Transcription directly via Ollama provider is not implemented. Ollama is typically used for text analysis. Configure WhisperCpp for STT and Ollama as text_model for analysis."))
        }
    }
}

// Function to call Whisper.cpp executable for transcription
async fn transcribe_with_whisper_cpp(
    audio_path: &Path,
    model_path: &str,
    executable_path: &str,
) -> Result<String, anyhow::Error> {
    info!(
        "[Whisper.cpp] Attempting transcription:\n  Executable: {}\n  Model: {}\n  Audio file: {}",
        executable_path,
        model_path,
        audio_path.display()
    );

    let absolute_audio_path = if audio_path.is_absolute() {
        audio_path.to_path_buf()
    } else {
        let current_dir = std::env::current_dir()?;
        info!(
            "[Whisper.cpp] Audio path is relative. Current directory: {}",
            current_dir.display()
        );
        current_dir.join(audio_path)
    };
    info!(
        "[Whisper.cpp] Absolute audio path: {}",
        absolute_audio_path.display()
    );

    let command_str = format!(
        "{} -m {} -f {} -l auto -otxt",
        executable_path,
        model_path,
        absolute_audio_path.to_str().unwrap_or("INVALID_PATH")
    );
    info!("[Whisper.cpp] Executing command: {}", command_str);

    let output = Command::new(executable_path)
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(
            absolute_audio_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid audio file path"))?,
        )
        .arg("-l")
        .arg("auto") // Specify Chinese language
        .arg("-otxt") // Output as plain text
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output_result) => {
            let stdout_str = String::from_utf8_lossy(&output_result.stdout);
            let stderr_str = String::from_utf8_lossy(&output_result.stderr);
            info!(
                "[Whisper.cpp] Process exited with status: {}",
                output_result.status
            );
            info!("[Whisper.cpp] STDOUT:\n{}", stdout_str);
            info!("[Whisper.cpp] STDERR:\n{}", stderr_str);

            if output_result.status.success() {
                let output_txt_path = absolute_audio_path.with_extension("wav.txt");
                info!(
                    "[Whisper.cpp] Attempting to read transcript from: {}",
                    output_txt_path.display()
                );

                match fs::read_to_string(&output_txt_path).await {
                    Ok(content) => {
                        info!(
                            "[Whisper.cpp] Successfully read transcript file. Content length: {}",
                            content.len()
                        );
                        // Optionally, remove the .txt file after reading
                        // fs::remove_file(output_txt_path).await.ok();
                        Ok(content.trim().to_string())
                    }
                    Err(e) => {
                        // error!("[Whisper.cpp] ERROR: Failed to read transcript file {}: {}", output_txt_path.display(), e);
                        Err(anyhow::anyhow!("Failed to read transcript file generated by Whisper.cpp: {}. Ensure Whisper.cpp has write permissions to the audio file's directory and the file exists.", e))
                    }
                }
            } else {
                // error!(
                //     "Whisper.cpp execution failed with status {}. STDERR: {}. STDOUT: {}",
                //     output_result.status,
                //     stderr_str,
                //     stdout_str
                // );
                Err(anyhow::anyhow!(
                    "Whisper.cpp execution failed with status {}. STDERR: {}. STDOUT: {}",
                    output_result.status,
                    stderr_str,
                    stdout_str
                ))
            }
        }
        Err(e) => {
            // error!("[Whisper.cpp] ERROR: Failed to execute Whisper.cpp command: {}", e);
            Err(anyhow::anyhow!("Failed to execute Whisper.cpp: {}. Check executable path, model path, and permissions.", e))
        }
    }
}

// 创建离线模式下的默认分析结果
fn create_offline_analysis_result(transcript: &str) -> AnalysisResult {
    // 从转录文本中提取前几个词作为标题
    let words: Vec<&str> = transcript.split_whitespace().take(5).collect();
    let title = if !words.is_empty() {
        format!("{}...", words.join(" "))
    } else {
        "离线模式转录".to_string()
    };
    
    // 使用转录文本的前100个字符作为摘要
    let preview = if transcript.chars().count() > 100 {
        let mut result = String::new();
        for (i, c) in transcript.chars().enumerate() {
            if i < 100 {
                result.push(c);
            } else {
                break;
            }
        }
        format!("{result}...")
    } else {
        transcript.to_string()
    };
    let summary = format!("[离线模式] {}", preview);
    
    // 创建基本的分析结果
    AnalysisResult {
        title,
        summary,
        ideas: vec!["[离线模式] 无法连接到AI服务，无法提取想法".to_string()],
        tasks: vec![Task {
            title: "检查网络连接".to_string(),
            description: Some("当前处于离线模式，无法连接到AI服务进行分析".to_string()),
            priority: Priority::Medium,
            due_date: None,
        }],
        structured_notes: vec![StructuredNote {
            title: "离线模式通知".to_string(),
            content: "当前处于离线模式，无法连接到AI服务进行分析。请检查网络连接或Ollama服务是否正常运行。".to_string(),
            tags: vec!["离线".to_string(), "需要重新分析".to_string()],
            note_type: NoteType::Reference,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }],
    }
}

pub async fn analyze_transcript(transcript: &str) -> Result<AnalysisResult, anyhow::Error> {
    let config = crate::config::load_config().await?;

    // 检查是否有OFFLINE环境变量或命令行参数
    let offline_mode = std::env::var("OFFLINE").is_ok();
    if offline_mode {
        info!("Running in offline mode. Returning default analysis result.");
        return Ok(create_offline_analysis_result(transcript));
    }

    // Determine which text model provider to use for analysis.
    // If the main ai_provider is WhisperCpp, we look at the text_model.provider configuration.
    // Otherwise, the main ai_provider (OpenAI, Ollama, Local) dictates the analysis method.
    let provider_for_analysis = AiProvider::Ollama;
    let use_openai_key: Option<String> = None;

    let ollama_settings_for_analysis = config.text_model.ollama_settings.clone();
    let local_model_endpoint_for_analysis = config.text_model.local_model_path.clone();

    info!(
        "Analyzing transcript with provider: {}",
        provider_for_analysis
    );

    match provider_for_analysis {
        AiProvider::OpenAI => {
            if let Some(api_key) = use_openai_key {
                analyze_with_openai(transcript, &api_key).await
            } else {
                // error!("OpenAI API key not configured for analysis.");
                Err(anyhow::anyhow!(
                    "OpenAI API key not configured for analysis."
                ))
            }
        }
        AiProvider::Ollama => {
            if let Some(ollama_settings) = ollama_settings_for_analysis {
                if ollama_settings.enabled {
                    // 使用 v2 版本的 Ollama 分析函数
                    analyze_with_ollama_v2(transcript, &ollama_settings.endpoint).await
                } else {
                    // warn!("Ollama is disabled in config. Skipping analysis.");
                    Ok(AnalysisResult::default_with_summary(
                        "Ollama analysis skipped (disabled).".to_string(),
                    ))
                }
            } else {
                // error!("Ollama settings not configured for analysis.");
                Err(anyhow::anyhow!(
                    "Ollama settings not configured for analysis."
                ))
            }
        }
        _ => {
            // warn!("No analysis provider configured or recognized. Skipping analysis.");
            Ok(AnalysisResult::default_with_summary(
                "No analysis performed.".to_string(),
            ))
        }
    }
}

async fn transcribe_with_openai(
    audio_path: &Path,
    _api_key: &str,
) -> Result<String, anyhow::Error> {
    info!("[OpenAI] Transcribing audio file: {}", audio_path.display());
    // Placeholder for actual OpenAI transcription logic
    Ok(format!("OpenAI transcription of {}", audio_path.display()))
}

async fn analyze_with_openai(
    transcript: &str,
    api_key: &str,
) -> Result<AnalysisResult, anyhow::Error> {
    info!("[OpenAI Analysis] Analyzing transcript: '{}'", transcript);
    let client = Client::with_config(OpenAIConfig::new().with_api_key(api_key));

    let system_message = ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessageArgs::default()
        .content("You are a helpful assistant that analyzes meeting transcripts. Extract key ideas, tasks, and structured notes. Provide a concise summary.")
        .build()?);

    let user_message = ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessageArgs::default()
            .content(format!(
                "Analyze the following transcript:\n\n{}",
                transcript
            ))
            .build()?,
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o") // Or another suitable model like "gpt-3.5-turbo"
        .messages(vec![system_message, user_message])
        .build()?;

    let response = client.chat().create(request).await?;

    let analysis_text = response.choices[0]
        .message
        .content
        .clone()
        .unwrap_or_default();
    info!("[OpenAI Analysis] Raw analysis response: {}", analysis_text);

    // Simple parsing for demonstration. A more robust solution would use structured JSON output from the AI.
    let title = extract_value(&analysis_text, "Title:");
    let ideas = extract_list(&analysis_text, "Ideas:");
    let tasks = extract_tasks(&analysis_text, "Tasks:");
    let structured_notes = extract_structured_notes(&analysis_text, "Notes:");
    let summary = extract_value(&analysis_text, "Summary:");

    Ok(AnalysisResult {
        title,
        ideas,
        tasks,
        structured_notes,
        summary,
    })
}

// Helper functions for parsing (simplified)
fn extract_value(text: &str, prefix: &str) -> String {
    text.lines()
        .find(|line| line.starts_with(prefix))
        .map(|line| line.trim_start_matches(prefix).trim().to_string())
        .unwrap_or_else(|| "N/A".to_string())
}

fn extract_list(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .skip_while(|line| !line.starts_with(prefix))
        .skip(1) // Skip the prefix line itself
        .take_while(|line| !line.is_empty() && !line.contains(":")) // Stop at next section or empty line
        .filter_map(|line| {
            line.trim_start_matches("-")
                .trim_start_matches("*")
                .trim()
                .to_string()
                .into()
        })
        .collect()
}

fn extract_tasks(text: &str, prefix: &str) -> Vec<crate::storage::Task> {
    text.lines()
        .skip_while(|line| !line.starts_with(prefix))
        .skip(1)
        .take_while(|line| !line.is_empty() && !line.contains(":"))
        .filter_map(|line| {
            let cleaned_line = line
                .trim_start_matches("-")
                .trim_start_matches("*")
                .trim()
                .to_string();
            if cleaned_line.is_empty() {
                return None;
            }
            Some(crate::storage::Task {
                title: cleaned_line,
                description: None,
                priority: crate::storage::Priority::Medium,
                due_date: None,
            })
        })
        .collect()
}

fn extract_structured_notes(text: &str, prefix: &str) -> Vec<crate::storage::StructuredNote> {
    text.lines()
        .skip_while(|line| !line.starts_with(prefix))
        .skip(1)
        .take_while(|line| !line.is_empty() && !line.contains(":"))
        .filter_map(|line| {
            let cleaned_line = line
                .trim_start_matches("-")
                .trim_start_matches("*")
                .trim()
                .to_string();
            if cleaned_line.is_empty() {
                return None;
            }
            Some(crate::storage::StructuredNote {
                title: cleaned_line,
                content: "".to_string(),
                tags: Vec::new(),
                note_type: crate::storage::NoteType::Reference,
                updated_at: chrono::Utc::now(),
                created_at: chrono::Utc::now(),
            })
        })
        .collect()
}
impl fmt::Display for AiProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AiProvider::OpenAI => write!(f, "OpenAI"),
            AiProvider::Ollama => write!(f, "Ollama"),
            AiProvider::WhisperCpp => write!(f, "WhisperCPP"),
        }
    }
}
