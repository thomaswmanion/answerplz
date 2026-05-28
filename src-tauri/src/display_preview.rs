use crate::screenshot::{list_monitors, MonitorInfo};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const PREVIEW_LABEL_PREFIX: &str = "monitor-preview-";

#[cfg(windows)]
const PREVIEW_CLOSE_SETTLE_MS: u64 = 50;

/// Full-screen monitor highlight overlays are unreliable on Windows (opaque WebView2,
/// click-through failures). The in-app monitor picker still works without them.
pub fn display_preview_supported() -> bool {
    !cfg!(windows)
}

fn settle_preview_close() {
    #[cfg(windows)]
    std::thread::sleep(std::time::Duration::from_millis(PREVIEW_CLOSE_SETTLE_MS));
}

fn close_preview_window(window: &WebviewWindow) {
    crate::set_ignore_cursor_events_safe!(window, false);
    let _ = window.set_always_on_top(false);
    let _ = window.hide();
    settle_preview_close();
    let _ = window.close();
}

pub fn hide_display_previews(app: &AppHandle) {
    let labels: Vec<String> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(PREVIEW_LABEL_PREFIX))
        .map(|(label, _)| label)
        .collect();

    for label in labels {
        if let Some(window) = app.get_webview_window(&label) {
            close_preview_window(&window);
        }
    }
}

pub fn show_display_preview(app: &AppHandle, selection: &str) -> Result<(), String> {
    if !display_preview_supported() {
        return Ok(());
    }

    hide_display_previews(app);

    let monitors = list_monitors().map_err(|e| e.to_string())?;
    if monitors.is_empty() {
        return Err("No displays found.".into());
    }

    let targets: Vec<&MonitorInfo> = match selection {
        "primary" => {
            let monitor = monitors
                .iter()
                .find(|m| m.is_primary)
                .or(monitors.first())
                .ok_or_else(|| "No displays found.".to_string())?;
            vec![monitor]
        }
        "all" => monitors.iter().collect(),
        index => {
            let index: usize = index
                .parse()
                .map_err(|_| format!("Invalid display index: {index}"))?;
            let monitor = monitors
                .get(index)
                .ok_or_else(|| format!("Display index {index} is out of range"))?;
            vec![monitor]
        }
    };

    for monitor in targets {
        create_preview_window(app, monitor)?;
    }

  // Keep settings above full-screen highlight overlays on macOS/Linux.
    if let Some(setup) = app.get_webview_window("setup") {
        let _ = setup.set_always_on_top(true);
        let _ = setup.set_focus();
    }

    Ok(())
}

fn create_preview_window(app: &AppHandle, monitor: &MonitorInfo) -> Result<(), String> {
    let label = format!("{PREVIEW_LABEL_PREFIX}{}", monitor.index);

    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::default())
        .title("")
        .position(monitor.x as f64, monitor.y as f64)
        .inner_size(monitor.width as f64, monitor.height as f64)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    let _ = window.show();
    // Enabling click-through before show panics on Linux/GTK.
    #[cfg(target_os = "linux")]
    std::thread::sleep(std::time::Duration::from_millis(50));
    crate::set_ignore_cursor_events_safe!(window, true);
    Ok(())
}
