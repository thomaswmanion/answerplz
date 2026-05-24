use crate::screenshot::{list_monitors, MonitorInfo};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const PREVIEW_LABEL_PREFIX: &str = "monitor-preview-";

pub fn hide_display_previews(app: &AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with(PREVIEW_LABEL_PREFIX) {
            let _ = window.close();
        }
    }
}

pub fn show_display_preview(app: &AppHandle, selection: &str) -> Result<(), String> {
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
        .build()
        .map_err(|e| e.to_string())?;

    let _ = window.set_ignore_cursor_events(true);
    Ok(())
}
