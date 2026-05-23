use base64::{engine::general_purpose::STANDARD, Engine};
use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use thiserror::Error;
use xcap::Monitor;

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
    let monitors = Monitor::all().map_err(|e| ScreenshotError::Capture(e.to_string()))?;
    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary())
        .or_else(|| Monitor::all().ok()?.into_iter().next())
        .ok_or(ScreenshotError::NoDisplays)?;

    let rgba = monitor
        .capture_image()
        .map_err(|e| ScreenshotError::Capture(e.to_string()))?;

    let width = rgba.width();
    let height = rgba.height();

    let mut jpeg_bytes: Vec<u8> = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 82);
    encoder
        .encode(
            rgba.as_raw(),
            width,
            height,
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| ScreenshotError::Encode(e.to_string()))?;

    Ok(CapturedScreen {
        jpeg_base64: STANDARD.encode(&jpeg_bytes),
        width,
        height,
    })
}
