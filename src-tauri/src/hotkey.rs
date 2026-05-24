use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::config::{self, AppConfig};

pub const DEFAULT_HOTKEY: &str = "Ctrl+Shift+A";

pub fn hotkey_string(config: &AppConfig) -> String {
    config
        .hotkey
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(DEFAULT_HOTKEY)
        .to_string()
}

pub fn parse_hotkey(raw: &str) -> Result<Shortcut, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Hotkey cannot be empty.".into());
    }
    Shortcut::from_str(trimmed).map_err(|e| format!("Invalid hotkey: {e}"))
}

pub fn validate_hotkey_string(raw: &str) -> Result<String, String> {
    let shortcut = parse_hotkey(raw)?;
    Ok(shortcut.to_string())
}

pub fn register_hotkey(app: &AppHandle) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let config = config::load_config().map_err(|e| e.to_string())?;
    let hotkey_str = hotkey_string(&config);
    let shortcut = parse_hotkey(&hotkey_str)?;

    gs.on_shortcut(shortcut, {
        let app = app.clone();
        move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<crate::AppState>();
                let _ = crate::run_capture_and_answer(app, &state).await;
            });
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}
