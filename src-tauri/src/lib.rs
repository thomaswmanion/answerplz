mod ai;
mod config;
mod display_preview;
mod screenshot;

use config::{
    config_summary, has_config, load_config, save_config, ConfigSummary, SaveConfigRequest,
};
use display_preview::{hide_display_previews, show_display_preview as apply_display_preview};
use screenshot::{capture_for_target, list_monitors, MonitorInfo};
use tauri::{
    AppHandle, Manager, RunEvent, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tokio::sync::Mutex;

const DWM_SETTLE_MS: u64 = 50;

struct AppState {
    answering: Mutex<bool>,
}

#[derive(serde::Serialize)]
struct AnswerResponse {
    answer: String,
}

/// Strip compositor hooks so Windows DWM releases the transparent overlay region.
macro_rules! release_compositor_hooks {
    ($window:expr) => {{
        let _ = $window.set_ignore_cursor_events(false);
        let _ = $window.set_always_on_top(false);
    }};
}

fn compositor_settle() {
    #[cfg(windows)]
    std::thread::sleep(std::time::Duration::from_millis(DWM_SETTLE_MS));
}

fn release_all_windows(app: &AppHandle) {
    hide_display_previews(app);
    for label in ["overlay", "setup", "main"] {
        if let Some(window) = app.get_webview_window(label) {
            release_compositor_hooks!(window);
            let _ = window.hide();
        }
    }
    compositor_settle();
}

fn suspend_overlay_for_capture(window: &WebviewWindow) {
    release_compositor_hooks!(window);
    let _ = window.hide();
    compositor_settle();
}

#[tauri::command]
fn get_config_summary() -> ConfigSummary {
    config_summary()
}

#[tauri::command]
fn list_displays() -> Result<Vec<MonitorInfo>, String> {
    list_monitors().map_err(|e| e.to_string())
}

#[tauri::command]
fn is_configured() -> bool {
    has_config()
}

#[tauri::command]
async fn save_app_config(request: SaveConfigRequest) -> ai::ValidationResult {
    let new_key = request
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty());

    let mut config = match load_config() {
        Ok(existing) => existing,
        Err(config::ConfigError::NotFound) => {
            let Some(key) = new_key else {
                return ai::ValidationResult {
                    ok: false,
                    message: "Enter an API key.".into(),
                };
            };
            config::AppConfig {
                provider: request.provider.clone(),
                api_key: key.to_string(),
                model: request.model.clone(),
                base_url: request.base_url.clone(),
                capture_monitor: request.capture_monitor.clone(),
            }
        }
        Err(e) => {
            return ai::ValidationResult {
                ok: false,
                message: e.to_string(),
            };
        }
    };

    config.provider = request.provider;
    config.model = request.model;
    config.base_url = request.base_url;
    config.capture_monitor = request.capture_monitor;
    if let Some(key) = new_key {
        config.api_key = key.to_string();
    }

    let result = if new_key.is_some() || !has_config() {
        ai::validate_config(&config).await
    } else {
        ai::ValidationResult {
            ok: true,
            message: "Settings saved.".into(),
        }
    };

    if result.ok {
        if let Err(e) = save_config(&config) {
            return ai::ValidationResult {
                ok: false,
                message: e.to_string(),
            };
        }
    }
    result
}

fn read_clipboard_text() -> Result<String, String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|e| format!("Could not read clipboard: {e}"))
}

async fn try_begin_answering(state: &State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.answering.lock().await;
    if *guard {
        return Err("Already processing a request.".into());
    }
    *guard = true;
    Ok(())
}

#[tauri::command]
async fn answer_question(
    state: State<'_, AppState>,
    question: String,
) -> Result<AnswerResponse, String> {
    try_begin_answering(&state).await?;
    let result = async {
        let config = config::load_config().map_err(|e| e.to_string())?;
        let answer = ai::answer_question(&config, &question)
            .await
            .map_err(|e| e.to_string())?;
        Ok(AnswerResponse { answer })
    }
    .await;
    *state.answering.lock().await = false;
    result
}

#[tauri::command]
async fn answer_from_clipboard(state: State<'_, AppState>) -> Result<AnswerResponse, String> {
    try_begin_answering(&state).await?;
    let result = async {
        let config = config::load_config().map_err(|e| e.to_string())?;
        let clipboard_text = read_clipboard_text()?;
        let answer = ai::answer_from_clipboard_text(&config, &clipboard_text)
            .await
            .map_err(|e| e.to_string())?;
        Ok(AnswerResponse { answer })
    }
    .await;
    *state.answering.lock().await = false;
    result
}

#[tauri::command]
async fn capture_and_answer(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AnswerResponse, String> {
    {
        let mut guard = state.answering.lock().await;
        if *guard {
            return Err("Already processing a screenshot.".into());
        }
        *guard = true;
    }

    if let Some(overlay) = app.get_webview_window("overlay") {
        suspend_overlay_for_capture(&overlay);
    }
    // Let the compositor drop the overlay before capture.
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    let result = async {
        let config = config::load_config().map_err(|e| e.to_string())?;
        let captured = capture_for_target(&config.capture_monitor).map_err(|e| e.to_string())?;
        let answer = ai::answer_from_screenshot(&config, &captured.jpeg_base64)
            .await
            .map_err(|e| e.to_string())?;
        Ok(AnswerResponse { answer })
    }
    .await;

    *state.answering.lock().await = false;
    show_overlay(&app);

    result
}

fn show_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.show();
        let _ = overlay.set_always_on_top(true);
    }
}

#[tauri::command]
fn show_display_preview(app: AppHandle, selection: String) -> Result<(), String> {
    apply_display_preview(&app, &selection)
}

#[tauri::command]
fn hide_display_preview(app: AppHandle) {
    hide_display_previews(&app);
}

#[tauri::command]
async fn open_setup_window(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window("setup").is_some() {
        if let Some(w) = app.get_webview_window("setup") {
            let _ = w.set_focus();
        }
        return Ok(());
    }
    build_setup_window(&app).map_err(|e| e.to_string())?;
    if let Some(overlay) = app.get_webview_window("overlay") {
        release_compositor_hooks!(overlay);
        compositor_settle();
        let _ = overlay.close();
    }
    Ok(())
}

#[tauri::command]
async fn finish_setup(app: AppHandle) -> Result<(), String> {
    close_setup_window(app).await
}

#[tauri::command]
async fn close_setup_window(app: AppHandle) -> Result<(), String> {
    hide_display_previews(&app);
    // Create and show overlay before closing setup — closing the only window exits the app.
    build_overlay_window(&app).map_err(|e| e.to_string())?;
    show_overlay(&app);
    if let Some(setup) = app.get_webview_window("setup") {
        let _ = setup.close();
    }
    Ok(())
}

#[tauri::command]
async fn quit_app(app: AppHandle) {
    release_all_windows(&app);
    app.exit(0);
}

fn build_setup_window(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("setup").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "setup", WebviewUrl::default())
        .title("answerplz — setup")
        .inner_size(480.0, 720.0)
        .resizable(false)
        .center()
        .build()?;
    Ok(())
}

fn build_overlay_window(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("overlay").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "overlay", WebviewUrl::default())
        .title("answerplz")
        .inner_size(420.0, 160.0)
        .min_inner_size(280.0, 80.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .build()?;
    show_overlay(app);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            answering: Mutex::new(false),
        })
        .setup(|app| {
            if has_config() {
                build_overlay_window(app.handle())?;
            } else {
                build_setup_window(app.handle())?;
            }
            if let Some(main) = app.get_webview_window("main") {
                release_compositor_hooks!(main);
                let _ = main.close();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => {
                    release_compositor_hooks!(window);
                    compositor_settle();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config_summary,
            list_displays,
            show_display_preview,
            hide_display_preview,
            is_configured,
            save_app_config,
            answer_question,
            answer_from_clipboard,
            capture_and_answer,
            open_setup_window,
            finish_setup,
            close_setup_window,
            quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                release_all_windows(app_handle);
            }
        });
}
