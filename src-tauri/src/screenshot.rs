use crate::config::CaptureMonitor;
use base64::{engine::general_purpose::STANDARD, Engine};
use display_info::DisplayInfo;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScreenshotError {
    #[error("no displays found")]
    NoDisplays,
    #[error("monitor index {0} is out of range (found {1} display(s))")]
    InvalidMonitorIndex(usize, usize),
    #[error("capture failed: {0}")]
    Capture(String),
    #[error("encode failed: {0}")]
    Encode(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorInfo {
    pub index: usize,
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
    pub label: String,
}

pub struct CapturedScreen {
    pub jpeg_base64: String,
    pub width: u32,
    pub height: u32,
}

pub fn list_monitors() -> Result<Vec<MonitorInfo>, ScreenshotError> {
    let displays = DisplayInfo::all().map_err(|e| ScreenshotError::Capture(e.to_string()))?;
    if displays.is_empty() {
        return Err(ScreenshotError::NoDisplays);
    }

    Ok(displays
        .into_iter()
        .enumerate()
        .map(|(index, display)| {
            let is_primary = display.is_primary;
            let label = format!(
                "Display {} — {}×{}{}",
                index + 1,
                display.width,
                display.height,
                if is_primary { " (primary)" } else { "" }
            );
            MonitorInfo {
                index,
                id: display.id,
                width: display.width,
                height: display.height,
                x: display.x,
                y: display.y,
                is_primary,
                label,
            }
        })
        .collect())
}

pub fn capture_for_target(target: &CaptureMonitor) -> Result<CapturedScreen, ScreenshotError> {
    match target {
        CaptureMonitor::Primary => capture_primary(),
        CaptureMonitor::All => capture_all(),
        CaptureMonitor::Monitor { index } => capture_monitor(*index),
    }
}

fn capture_primary() -> Result<CapturedScreen, ScreenshotError> {
    let monitors = list_monitors()?;
    let index = monitors
        .iter()
        .find(|m| m.is_primary)
        .map(|m| m.index)
        .unwrap_or(0);
    capture_monitor(index)
}

fn capture_monitor(index: usize) -> Result<CapturedScreen, ScreenshotError> {
    let monitors = list_monitors()?;
    let monitor = monitors
        .get(index)
        .ok_or(ScreenshotError::InvalidMonitorIndex(index, monitors.len()))?;

    #[cfg(target_os = "macos")]
    {
        return capture_macos_display(Some(monitor.id));
    }

    #[cfg(not(target_os = "macos"))]
    {
        return capture_display_via_screenshots(monitor.id);
    }
}

fn capture_all() -> Result<CapturedScreen, ScreenshotError> {
    #[cfg(target_os = "macos")]
    {
        // screencapture without -D captures the full desktop spanning all displays.
        return capture_macos_display(None);
    }

    #[cfg(not(target_os = "macos"))]
    {
        return capture_all_via_screenshots();
    }
}

#[cfg(target_os = "macos")]
fn macos_screen_capture_granted() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    unsafe {
        if CGPreflightScreenCaptureAccess() {
            return true;
        }
        CGRequestScreenCaptureAccess()
    }
}

#[cfg(target_os = "macos")]
fn capture_macos_display(display_id: Option<u32>) -> Result<CapturedScreen, ScreenshotError> {
    use std::process::Command;

    if !macos_screen_capture_granted() {
        return Err(ScreenshotError::Capture(
            "Screen Recording permission is required for screenshots. Open System Settings → \
             Privacy & Security → Screen Recording, enable answerplz, then restart the app."
                .into(),
        ));
    }

    let path = std::env::temp_dir().join(format!(
        "answerplz-{}.jpg",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));

    let path_str = path
        .to_str()
        .ok_or_else(|| ScreenshotError::Capture("invalid temp path".into()))?;

    let mut command = Command::new("/usr/sbin/screencapture");
    command.args(["-x", "-t", "jpg"]);
    if let Some(id) = display_id {
        command.arg("-D").arg(id.to_string());
    }
    command.arg(path_str);

    let status = command
        .status()
        .map_err(|e| ScreenshotError::Capture(e.to_string()))?;

    if !status.success() {
        return Err(ScreenshotError::Capture(format!(
            "screencapture exited with {status}"
        )));
    }

    let jpeg_bytes = std::fs::read(&path).map_err(|e| ScreenshotError::Capture(e.to_string()))?;
    let _ = std::fs::remove_file(&path);

    if jpeg_bytes.is_empty() {
        return Err(ScreenshotError::Capture("empty screenshot".into()));
    }

    Ok(CapturedScreen {
        jpeg_base64: STANDARD.encode(&jpeg_bytes),
        width: 0,
        height: 0,
    })
}

#[cfg(not(target_os = "macos"))]
fn capture_display_via_screenshots(display_id: u32) -> Result<CapturedScreen, ScreenshotError> {
    use screenshots::Screen;

    let screen = Screen::all()
        .map_err(|e| ScreenshotError::Capture(e.to_string()))?
        .into_iter()
        .find(|s| s.display_info.id == display_id)
        .ok_or_else(|| ScreenshotError::Capture(format!("display {display_id} not found")))?;

    let rgba = screen
        .capture()
        .map_err(|e| ScreenshotError::Capture(e.to_string()))?;

    rgba_to_jpeg(&rgba)
}

#[cfg(not(target_os = "macos"))]
fn capture_all_via_screenshots() -> Result<CapturedScreen, ScreenshotError> {
    use screenshots::Screen;

    let screens = Screen::all().map_err(|e| ScreenshotError::Capture(e.to_string()))?;
    if screens.is_empty() {
        return Err(ScreenshotError::NoDisplays);
    }

    if screens.len() == 1 {
        let rgba = screens[0]
            .capture()
            .map_err(|e| ScreenshotError::Capture(e.to_string()))?;
        return rgba_to_jpeg(&rgba);
    }

    let mut captures = Vec::with_capacity(screens.len());
    for screen in screens {
        let info = &screen.display_info;
        let rgba = screen
            .capture()
            .map_err(|e| ScreenshotError::Capture(e.to_string()))?;
        captures.push((info.x, info.y, rgba));
    }

    stitch_and_encode(&captures)
}

#[cfg(not(target_os = "macos"))]
fn stitch_and_encode(
    captures: &[(i32, i32, image::RgbaImage)],
) -> Result<CapturedScreen, ScreenshotError> {
    use image::RgbaImage;

    let min_x = captures.iter().map(|(x, _, _)| *x).min().unwrap_or(0);
    let min_y = captures.iter().map(|(y, _, _)| *y).min().unwrap_or(0);
    let max_x = captures
        .iter()
        .map(|(x, _, img)| x + img.width() as i32)
        .max()
        .unwrap_or(0);
    let max_y = captures
        .iter()
        .map(|(y, _, img)| y + img.height() as i32)
        .max()
        .unwrap_or(0);

    let canvas_w = (max_x - min_x).max(1) as u32;
    let canvas_h = (max_y - min_y).max(1) as u32;
    let mut canvas = RgbaImage::new(canvas_w, canvas_h);

    for (x, y, img) in captures {
        image::imageops::overlay(
            &mut canvas,
            img,
            (*x - min_x) as i64,
            (*y - min_y) as i64,
        );
    }

    rgba_to_jpeg(&canvas)
}

#[cfg(not(target_os = "macos"))]
fn rgba_to_jpeg(rgba: &image::RgbaImage) -> Result<CapturedScreen, ScreenshotError> {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ColorType, DynamicImage};

    let width = rgba.width();
    let height = rgba.height();
    let rgb = DynamicImage::ImageRgba8(rgba.clone()).into_rgb8();

    let mut jpeg_bytes: Vec<u8> = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 82);
    encoder
        .encode(rgb.as_raw(), width, height, ColorType::Rgb8)
        .map_err(|e| ScreenshotError::Encode(e.to_string()))?;

    Ok(CapturedScreen {
        jpeg_base64: STANDARD.encode(&jpeg_bytes),
        width,
        height,
    })
}
