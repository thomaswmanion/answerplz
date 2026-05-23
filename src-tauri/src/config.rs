use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Openai,
    Anthropic,
    Google,
    Openrouter,
}

impl Provider {
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Openai => "gpt-4o-mini",
            Provider::Anthropic => "claude-3-5-haiku-latest",
            Provider::Google => "gemini-2.5-flash",
            Provider::Openrouter => "openai/gpt-4o-mini",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CaptureMonitor {
    Primary,
    All,
    Monitor { index: usize },
}

impl Default for CaptureMonitor {
    fn default() -> Self {
        Self::Primary
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub provider: Provider,
    pub api_key: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub capture_monitor: CaptureMonitor,
}

impl AppConfig {
    pub fn model(&self) -> String {
        self.model
            .clone()
            .unwrap_or_else(|| self.provider.default_model().to_string())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not find home directory")]
    NoHome,
    #[error("failed to read config: {0}")]
    Read(String),
    #[error("failed to write config: {0}")]
    Write(String),
    #[error("config not found")]
    NotFound,
}

pub fn config_dir() -> Result<PathBuf, ConfigError> {
    dirs::home_dir()
        .map(|h| h.join(".answerplz"))
        .ok_or(ConfigError::NoHome)
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    Ok(config_dir()?.join("config.json"))
}

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        return Err(ConfigError::NotFound);
    }
    let raw = fs::read_to_string(&path).map_err(|e| ConfigError::Read(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| ConfigError::Read(e.to_string()))
}

pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).map_err(|e| ConfigError::Write(e.to_string()))?;
    let path = dir.join("config.json");
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| ConfigError::Write(e.to_string()))?;
    fs::write(&path, json).map_err(|e| ConfigError::Write(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn has_config() -> bool {
    config_path()
        .ok()
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// Public view without API key (for UI display).
#[derive(Debug, Clone, Serialize)]
pub struct ConfigSummary {
    pub provider: Provider,
    pub model: String,
    pub configured: bool,
    pub capture_monitor: CaptureMonitor,
}

pub fn config_summary() -> ConfigSummary {
    match load_config() {
        Ok(c) => ConfigSummary {
            provider: c.provider.clone(),
            model: c.model(),
            configured: true,
            capture_monitor: c.capture_monitor.clone(),
        },
        Err(_) => ConfigSummary {
            provider: Provider::Openai,
            model: Provider::Openai.default_model().to_string(),
            configured: false,
            capture_monitor: CaptureMonitor::default(),
        },
    }
}
