mod ai;
mod config;
mod display_preview;
mod history;
mod hotkey;
mod platform;
mod screenshot;

use config::{
    config_summary, has_config, load_config, save_config, ConfigSummary, SaveConfigRequest,
};
use display_preview::{hide_display_previews, show_display_preview as apply_display_preview};
use history::{append_entry, clear_history, list_history, AnswerSource, HistoryEntry};
use hotkey::{register_hotkey, validate_hotkey_string};
use screenshot::{capture_for_target, list_monitors, MonitorInfo};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tokio::sync::Mutex;

const DWM_SETTLE_MS: u64 = 50;

pub struct AppState {
    answering: Mutex<bool>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OverlayAnswerEvent {
    pub answer: String,
    pub is_error: bool,
}

#[derive(serde::Serialize)]
struct AnswerResponse {
    answer: String,
}

/// Strip compositor hooks so Windows DWM releases the transparent overlay region.
macro_rules! release_compositor_hooks {
    ($window:expr) => {{
        crate::set_ignore_cursor_events_safe!($window, false);
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

fn show_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.show();
        let _ = overlay.set_always_on_top(true);
        let _ = overlay.set_focus();
    }
}

/// Bring back the overlay after settings closes (show existing window or create one).
fn restore_overlay_after_setup(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("overlay").is_some() {
        show_overlay(app);
        Ok(())
    } else {
        build_overlay_window(app)
    }
}

fn hide_overlay_window(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        release_compositor_hooks!(overlay);
        let _ = overlay.hide();
    }
}

fn overlay_is_visible(app: &AppHandle) -> bool {
    app.get_webview_window("overlay")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

fn toggle_overlay_visibility(app: &AppHandle) {
    if overlay_is_visible(app) {
        hide_overlay_window(app);
    } else {
        show_overlay(app);
    }
}

fn emit_overlay_loading(app: &AppHandle) {
    let _ = app.emit_to("overlay", "overlay-loading", ());
}

fn emit_overlay_answer(app: &AppHandle, answer: String, is_error: bool) {
    let _ = app.emit_to(
        "overlay",
        "overlay-answer",
        OverlayAnswerEvent { answer, is_error },
    );
}

async fn try_begin_answering(state: &State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.answering.lock().await;
    if *guard {
        return Err("Already processing a request.".into());
    }
    *guard = true;
    Ok(())
}

fn read_clipboard_text() -> Result<String, String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|e| format!("Could not read clipboard: {e}"))
}

pub async fn run_capture_and_answer(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    {
        let mut guard = state.answering.lock().await;
        if *guard {
            return Err("Already processing a screenshot.".into());
        }
        *guard = true;
    }

    emit_overlay_loading(&app);

    if let Some(overlay) = app.get_webview_window("overlay") {
        suspend_overlay_for_capture(&overlay);
    }
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    let captured = async {
        let config = config::load_config().map_err(|e| e.to_string())?;
        capture_for_target(&config.capture_monitor).map_err(|e| e.to_string())
    }
    .await;

    show_overlay(&app);
    emit_overlay_loading(&app);

    let result: Result<String, String> = match captured {
        Ok(captured) => {
            let config = config::load_config().map_err(|e| e.to_string())?;
            let answer = ai::answer_from_screenshot(&config, &captured.jpeg_base64)
                .await
                .map_err(|e| e.to_string())?;
            append_entry(AnswerSource::Screenshot, "Screenshot", &answer);
            Ok(answer)
        }
        Err(err) => Err(err),
    };

    *state.answering.lock().await = false;

    match &result {
        Ok(answer) => emit_overlay_answer(&app, answer.clone(), false),
        Err(err) => emit_overlay_answer(&app, err.clone(), true),
    }

    result
}

fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if !platform::system_tray_available() {
        eprintln!(
            "system tray unavailable (no DBus session); use the overlay bar for settings and quit"
        );
        return Ok(());
    }

    let show_i = MenuItem::with_id(app, "tray-show", "Show overlay", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "tray-hide", "Hide overlay", true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "tray-settings", "Settings…", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "tray-quit", "Quit answerplz", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &hide_i, &settings_i, &quit_i])?;

    let icon = app
        .default_window_icon()
        .ok_or("missing default window icon")?
        .clone();

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("answerplz")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray-show" => show_overlay(app),
            "tray-hide" => hide_overlay_window(app),
            "tray-settings" => {
                let _ = build_setup_window(app);
                if let Some(setup) = app.get_webview_window("setup") {
                    let _ = setup.set_focus();
                }
            }
            "tray-quit" => {
                release_all_windows(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_overlay_visibility(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
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
fn validate_hotkey(hotkey: String) -> Result<String, String> {
    validate_hotkey_string(&hotkey)
}

#[tauri::command]
fn list_answer_history() -> Result<Vec<HistoryEntry>, String> {
    list_history()
}

#[tauri::command]
fn clear_answer_history() -> Result<(), String> {
    clear_history()
}

fn normalized_answer_prompt(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() || trimmed == ai::DEFAULT_ANSWER_PROMPT {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[tauri::command]
async fn save_app_config(
    app: AppHandle,
    request: SaveConfigRequest,
) -> ai::ValidationResult {
    let new_key = request
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty());

    let hotkey_raw = request.hotkey.as_deref().map(str::trim);
    if let Some(raw) = hotkey_raw.filter(|s| !s.is_empty()) {
        if let Err(msg) = validate_hotkey_string(raw) {
            return ai::ValidationResult {
                ok: false,
                message: msg,
            };
        }
    }

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
                hotkey: request.hotkey.clone(),
                answer_prompt: normalized_answer_prompt(request.answer_prompt.as_deref()),
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
    if let Some(raw) = hotkey_raw.filter(|s| !s.is_empty()) {
        config.hotkey = Some(raw.to_string());
    }
    config.answer_prompt = normalized_answer_prompt(request.answer_prompt.as_deref());

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
        if let Err(e) = register_hotkey(&app) {
            return ai::ValidationResult {
                ok: false,
                message: format!("Settings saved but hotkey failed: {e}"),
            };
        }
    }
    result
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
        let preview = if question.chars().count() > 80 {
            format!("{}…", question.chars().take(80).collect::<String>())
        } else {
            question.clone()
        };
        append_entry(AnswerSource::Question, &preview, &answer);
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
        let preview = if clipboard_text.chars().count() > 80 {
            format!(
                "{}…",
                clipboard_text.chars().take(80).collect::<String>()
            )
        } else {
            clipboard_text.clone()
        };
        let answer = ai::answer_from_clipboard_text(&config, &clipboard_text)
            .await
            .map_err(|e| e.to_string())?;
        append_entry(AnswerSource::Clipboard, &preview, &answer);
        Ok(AnswerResponse { answer })
    }
    .await;
    *state.answering.lock().await = false;
    result
}

#[tauri::command]
async fn capture_and_answer(app: AppHandle) -> Result<AnswerResponse, String> {
    let answer = run_capture_and_answer(app).await?;
    Ok(AnswerResponse { answer })
}

#[tauri::command]
fn hide_overlay(app: AppHandle) {
    hide_overlay_window(&app);
}

#[tauri::command]
fn show_overlay_command(app: AppHandle) {
    show_overlay(&app);
}

#[tauri::command]
fn toggle_overlay(app: AppHandle) {
    toggle_overlay_visibility(&app);
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
fn display_preview_supported() -> bool {
    display_preview::display_preview_supported()
}

#[tauri::command]
async fn open_setup_window(app: AppHandle) -> Result<(), String> {
    hide_display_previews(&app);
    if app.get_webview_window("setup").is_some() {
        if let Some(w) = app.get_webview_window("setup") {
            let _ = w.set_always_on_top(true);
            let _ = w.set_focus();
        }
        hide_overlay_window(&app);
        return Ok(());
    }
    build_setup_window(&app).map_err(|e| e.to_string())?;
    if let Some(setup) = app.get_webview_window("setup") {
        let _ = setup.set_always_on_top(true);
        let _ = setup.set_focus();
    }
    hide_overlay_window(&app);
    Ok(())
}

#[tauri::command]
async fn finish_setup(app: AppHandle) -> Result<(), String> {
    close_setup_window(app).await
}

#[tauri::command]
async fn close_setup_window(app: AppHandle) -> Result<(), String> {
    hide_display_previews(&app);
    restore_overlay_after_setup(&app).map_err(|e| e.to_string())?;
    if let Some(setup) = app.get_webview_window("setup") {
        let _ = setup.set_always_on_top(false);
        let _ = setup.close();
    }
    if let Err(e) = register_hotkey(&app) {
        eprintln!("hotkey registration failed: {e}");
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
        .inner_size(480.0, 820.0)
        .resizable(true)
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
        .inner_size(300.0, 56.0)
        .min_inner_size(220.0, 52.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            answering: Mutex::new(false),
        })
        .setup(|app| {
            if let Err(e) = build_tray(app.handle()) {
                eprintln!("tray icon failed: {e}");
            }
            if has_config() {
                build_overlay_window(app.handle())?;
                if let Err(e) = register_hotkey(app.handle()) {
                    eprintln!("hotkey registration failed: {e}");
                }
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
            let label = window.label();
            match event {
                WindowEvent::CloseRequested { .. } if label == "setup" => {
                    hide_display_previews(window.app_handle());
                    let app = window.app_handle();
                    let _ = window.set_always_on_top(false);
                    if let Err(e) = restore_overlay_after_setup(&app) {
                        eprintln!("failed to restore overlay after setup close: {e}");
                    }
                }
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => {
                    if label.starts_with("monitor-preview-") {
                        release_compositor_hooks!(window);
                        compositor_settle();
                    } else {
                        release_compositor_hooks!(window);
                        compositor_settle();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config_summary,
            list_displays,
            show_display_preview,
            hide_display_preview,
            display_preview_supported,
            is_configured,
            validate_hotkey,
            save_app_config,
            answer_question,
            answer_from_clipboard,
            capture_and_answer,
            list_answer_history,
            clear_answer_history,
            hide_overlay,
            show_overlay_command,
            toggle_overlay,
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
