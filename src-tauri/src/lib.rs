mod ai;
mod config;
mod screenshot;

use config::{config_summary, has_config, save_config, AppConfig, ConfigSummary};
use screenshot::capture_primary_screen;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Mutex;

struct AppState {
    answering: Mutex<bool>,
}

#[derive(serde::Serialize)]
struct AnswerResponse {
    answer: String,
}

#[tauri::command]
fn get_config_summary() -> ConfigSummary {
    config_summary()
}

#[tauri::command]
fn is_configured() -> bool {
    has_config()
}

#[tauri::command]
async fn validate_and_save_config(config: AppConfig) -> ai::ValidationResult {
    let result = ai::validate_config(&config).await;
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
        let _ = overlay.hide();
    }
    // Let the compositor drop the overlay before capture.
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    let result = async {
        let config = config::load_config().map_err(|e| e.to_string())?;
        let captured = capture_primary_screen().map_err(|e| e.to_string())?;
        let answer = ai::answer_from_screenshot(&config, &captured.jpeg_base64)
            .await
            .map_err(|e| e.to_string())?;
        Ok(AnswerResponse { answer })
    }
    .await;

    *state.answering.lock().await = false;

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.show();
    }

    result
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
        let _ = overlay.close();
    }
    Ok(())
}

#[tauri::command]
async fn finish_setup(app: AppHandle) -> Result<(), String> {
    if let Some(setup) = app.get_webview_window("setup") {
        let _ = setup.close();
    }
    build_overlay_window(&app).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn build_setup_window(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("setup").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "setup", WebviewUrl::default())
        .title("answerplz — setup")
        .inner_size(440.0, 520.0)
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
        .inner_size(280.0, 120.0)
        .min_inner_size(200.0, 80.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .build()?;
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
                let _ = main.close();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config_summary,
            is_configured,
            validate_and_save_config,
            capture_and_answer,
            open_setup_window,
            finish_setup,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
