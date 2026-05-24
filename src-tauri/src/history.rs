use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_ENTRIES: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnswerSource {
    Screenshot,
    Question,
    Clipboard,
}

impl AnswerSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Screenshot => "screenshot",
            Self::Question => "question",
            Self::Clipboard => "clipboard",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub at: String,
    pub source: String,
    pub preview: String,
    pub answer: String,
}

fn history_path() -> Result<PathBuf, String> {
    crate::config::config_dir()
        .map(|d| d.join("history.json"))
        .map_err(|e| e.to_string())
}

pub fn list_history() -> Result<Vec<HistoryEntry>, String> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

pub fn clear_history() -> Result<(), String> {
    let path = history_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn append_entry(source: AnswerSource, preview: &str, answer: &str) {
    let preview = truncate(preview, 120);
    let answer_stored = truncate(answer, 4000);
    let entry = HistoryEntry {
        at: chrono_lite_now(),
        source: source.as_str().to_string(),
        preview,
        answer: answer_stored,
    };
    let _ = append_entry_inner(entry);
}

fn append_entry_inner(entry: HistoryEntry) -> Result<(), String> {
    let path = history_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut entries = list_history().unwrap_or_default();
    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);
    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    out.push('…');
    out
}

/// Minimal UTC timestamp without pulling in chrono.
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
