// src/config/mod.rs
//! Configuration management module
//!
//! This module handles application configuration loading from various sources
//! including environment variables, configuration files, and default values.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Result, Context};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// OpenAI API configuration
    pub openai: OpenAIConfig,
    /// Ollama configuration
    pub ollama: OllamaConfig,
    /// Storage configuration
    pub storage: StorageConfig,
    /// Analysis configuration
    pub analysis: AnalysisConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
    /// Enable SQL query logging
    pub log_queries: bool,
}

/// OpenAI API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// OpenAI API key
    pub api_key: String,
    /// OpenAI organization ID (optional)
    pub organization_id: Option<String>,
    /// Default model for transcription
    pub transcription_model: String,
    /// Default model for analysis
    pub analysis_model: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
}

/// Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama server base URL
    pub base_url: String,
    /// Default model for analysis
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Enable automatic model pulling
    pub auto_pull_models: bool,
    /// Models to ensure are available
    pub required_models: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory for storing audio files
    pub audio_directory: PathBuf,
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Allowed audio formats
    pub allowed_formats: Vec<String>,
    /// Enable file compression
    pub enable_compression: bool,
    /// Cleanup old files after days
    pub cleanup_after_days: Option<u32>,
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Default model for analysis
    pub default_model: String,
    /// Default provider (openai or ollama)
    pub default_provider: String,
    /// Enable automatic analysis
    pub auto_analyze: bool,
    /// Analysis timeout in seconds
    pub timeout_secs: u64,
    /// Maximum content length for analysis
    pub max_content_length: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log format (json or pretty)
    pub format: String,
    /// Log file path (optional)
    pub file_path: Option<PathBuf>,
    /// Enable request logging
    pub log_requests: bool,
    /// Enable SQL query logging
    pub log_sql: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            openai: OpenAIConfig::default(),
            ollama: OllamaConfig::default(),
            storage: StorageConfig::default(),
            analysis: AnalysisConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_origins: vec!["http://localhost:3000".to_string()],
            request_timeout_secs: 30,
            max_body_size: 50 * 1024 * 1024, // 50MB
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://voice_recorder:password@localhost/voice_recorder".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
            log_queries: false,
        }
    }
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            organization_id: None,
            transcription_model: "whisper-1".to_string(),
            analysis_model: "gpt-3.5-turbo".to_string(),
            timeout_secs: 120,
            max_retries: 3,
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            default_model: "llama2".to_string(),
            timeout_secs: 300,
            auto_pull_models: false,
            required_models: vec!["llama2".to_string()],
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            audio_directory: PathBuf::from("./storage/audio"),
            max_file_size: 100 * 1024 * 1024, // 100MB
            allowed_formats: vec![
                "wav".to_string(),
                "mp3".to_string(),
                "m4a".to_string(),
                "flac".to_string(),
            ],
            enable_compression: false,
            cleanup_after_days: Some(90),
        }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            default_model: "llama2".to_string(),
            default_provider: "ollama".to_string(),
            auto_analyze: true,
            timeout_secs: 300,
            max_content_length: 50000,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file_path: None,
            log_requests: true,
            log_sql: false,
        }
    }
}

impl Config {
    /// Load configuration from file and environment variables
    pub fn load() -> Result<Self> {
        let mut config = Config::default();
        
        // Try to load from config file
        if let Ok(file_config) = Self::load_from_file("config.toml") {
            config = file_config;
        }
        
        // Override with environment variables
        config.load_from_env()?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Load configuration from a TOML file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path))?;
        
        toml::from_str(&content)
            .context("Failed to parse config file")
    }
    
    /// Load configuration from environment variables
    pub fn load_from_env(&mut self) -> Result<()> {
        // Server configuration
        if let Ok(host) = std::env::var("SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            self.server.port = port.parse().context("Invalid SERVER_PORT")?;
        }
        
        // Database configuration
        if let Ok(url) = std::env::var("DATABASE_URL") {
            self.database.url = url;
        }
        
        // OpenAI configuration
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.openai.api_key = api_key;
        }
        if let Ok(org_id) = std::env::var("OPENAI_ORGANIZATION_ID") {
            self.openai.organization_id = Some(org_id);
        }
        
        // Ollama configuration
        if let Ok(base_url) = std::env::var("OLLAMA_BASE_URL") {
            self.ollama.base_url = base_url;
        }
        if let Ok(model) = std::env::var("OLLAMA_DEFAULT_MODEL") {
            self.ollama.default_model = model;
        }
        
        // Storage configuration
        if let Ok(audio_dir) = std::env::var("STORAGE_AUDIO_DIRECTORY") {
            self.storage.audio_directory = PathBuf::from(audio_dir);
        }
        
        // Logging configuration
        if let Ok(level) = std::env::var("LOG_LEVEL") {
            self.logging.level = level;
        }
        
        Ok(())
    }
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }
        
        // Validate database URL
        if self.database.url.is_empty() {
            return Err(anyhow::anyhow!("Database URL cannot be empty"));
        }
        
        // Validate OpenAI API key if using OpenAI
        if self.analysis.default_provider == "openai" && self.openai.api_key.is_empty() {
            return Err(anyhow::anyhow!("OpenAI API key is required when using OpenAI provider"));
        }
        
        // Validate Ollama URL
        if self.analysis.default_provider == "ollama" && self.ollama.base_url.is_empty() {
            return Err(anyhow::anyhow!("Ollama base URL cannot be empty"));
        }
        
        // Validate storage directory
        if !self.storage.audio_directory.exists() {
            std::fs::create_dir_all(&self.storage.audio_directory)
                .context("Failed to create audio storage directory")?;
        }
        
        // Validate log level
        match self.logging.level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(anyhow::anyhow!("Invalid log level: {}", self.logging.level)),
        }
        
        Ok(())
    }
    
    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(path, content)
            .context(format!("Failed to write config file: {}", path))?;
        
        Ok(())
    }
    
    /// Get database connection string
    pub fn database_url(&self) -> &str {
        &self.database.url
    }
    
    /// Check if OpenAI is configured
    pub fn is_openai_configured(&self) -> bool {
        !self.openai.api_key.is_empty()
    }
    

}

/// Get the storage directory path
pub fn get_storage_dir() -> PathBuf {
    // Try to load from config.json first
    if let Ok(content) = std::fs::read_to_string("config.json") {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(storage_dir) = config.get("storage_dir").and_then(|v| v.as_str()) {
                return PathBuf::from(storage_dir);
            }
        }
    }
    
    // Fallback to default location
    PathBuf::from("./local_storage/app_data")
}



/// AI Provider enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AiProvider {
    OpenAI,
    Ollama,
    WhisperCpp,
}

/// Legacy config structure for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfig {
    pub ai_provider: AiProvider,
    pub api_keys: ApiKeysConfig,
    pub speech_model: SpeechModelConfig,
    pub text_model: TextModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeysConfig {
    pub openai_api_key: Option<String>,
    pub google_cloud_api_key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechModelConfig {
    pub whisper_model_path: Option<String>,
    pub whisper_executable_path: Option<String>,
    pub mozilla_tts_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextModelConfig {
    pub ollama_settings: Option<OllamaSettings>,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaSettings {
    pub enabled: bool,
    pub endpoint: String,
    pub model_name: String,
}

pub async fn load_config() -> Result<LegacyConfig> {
    let config_content = std::fs::read_to_string("config.json")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;

    Ok(LegacyConfig {
        ai_provider: AiProvider::WhisperCpp,
        api_keys: ApiKeysConfig {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            google_cloud_api_key_path: None,
        },
        speech_model: SpeechModelConfig {
            whisper_model_path: config["speech_model"]["whisper_model_path"]
                .as_str().map(String::from),
            whisper_executable_path: config["speech_model"]["whisper_executable_path"]
                .as_str().map(String::from),
            mozilla_tts_model_path: None,
        },
        text_model: TextModelConfig {
            ollama_settings: Some(OllamaSettings {
                enabled: true,
                endpoint: config["text_model"]["ollama_settings"]["endpoint"]
                    .as_str().unwrap().to_string(),
                model_name: config["text_model"]["ollama_settings"]["model_name"]
                    .as_str().unwrap().to_string(),
            }),
            local_model_path: None,
        },
    })
}
