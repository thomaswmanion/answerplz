use base64::{engine::general_purpose::STANDARD, Engine};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScreenshotError {
    #[error("no displays found")]
    NoDisplays,
    #[error("capture failed: {0}")]
    Capture(String),
    #[error("encode failed: {0}")]
    Encode(String),
}

pub struct CapturedScreen {
    pub jpeg_base64: String,
    pub width: u32,
    pub height: u32,
}

/// Capture the primary display and return JPEG base64 for vision APIs.
pub fn capture_primary_screen() -> Result<CapturedScreen, ScreenshotError> {
    #[cfg(target_os = "macos")]
    {
        return capture_macos();
    }

    #[cfg(not(target_os = "macos"))]
    {
        return capture_via_screenshots();
    }
}

#[cfg(target_os = "macos")]
fn capture_macos() -> Result<CapturedScreen, ScreenshotError> {
    use std::process::Command;

    let path = std::env::temp_dir().join(format!(
        "answerplz-{}.jpg",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));

    let status = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-t", "jpg", path.to_str().ok_or_else(|| {
            ScreenshotError::Capture("invalid temp path".into())
        })?])
        .status()
        .map_err(|e| ScreenshotError::Capture(e.to_string()))?;

    if !status.success() {
        return Err(ScreenshotError::Capture(format!(
            "screencapture exited with {status}"
        )));
    }

    let jpeg_bytes =
        std::fs::read(&path).map_err(|e| ScreenshotError::Capture(e.to_string()))?;
    let _ = std::fs::remove_file(&path);

    if jpeg_bytes.is_empty() {
        return Err(ScreenshotError::Capture("empty screenshot".into()));
    }

    // JPEG dimensions are not needed by the vision API; keep placeholders.
    Ok(CapturedScreen {
        jpeg_base64: STANDARD.encode(&jpeg_bytes),
        width: 0,
        height: 0,
    })
}

#[cfg(not(target_os = "macos"))]
fn capture_via_screenshots() -> Result<CapturedScreen, ScreenshotError> {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ColorType, DynamicImage};
    use screenshots::Screen;

    let screens = Screen::all().map_err(|e| ScreenshotError::Capture(e.to_string()))?;
    let screen = screens
        .into_iter()
        .next()
        .ok_or(ScreenshotError::NoDisplays)?;

    let rgba = screen
        .capture()
        .map_err(|e| ScreenshotError::Capture(e.to_string()))?;

    let width = rgba.width();
    let height = rgba.height();
    let rgb = DynamicImage::ImageRgba8(rgba).into_rgb8();

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
