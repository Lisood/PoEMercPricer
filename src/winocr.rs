//! Windows.Media.Ocr, line-by-line with bounding boxes.
//! The inspect panel prints skill names as text; `$result.Text` concatenates
//! them into one blob, which breaks parsing. Always use `Lines`.

use anyhow::{Context, Result};
use image::RgbaImage;

#[derive(Clone, Debug)]
pub struct OcrLine {
    pub text: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub fn recognize_lines(img: &RgbaImage) -> Result<Vec<OcrLine>> {
    #[cfg(windows)]
    {
        windows_ocr_lines(img)
    }
    #[cfg(not(windows))]
    {
        let _ = img;
        anyhow::bail!("screen OCR is Windows-only (Windows.Media.Ocr)")
    }
}

pub fn recognize_text(img: &RgbaImage) -> Result<String> {
    let lines = recognize_lines(img)?;
    Ok(lines
        .into_iter()
        .map(|l| l.text)
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(windows)]
fn windows_ocr_lines(img: &RgbaImage) -> Result<Vec<OcrLine>> {
    let started = std::time::Instant::now();
    use windows::core::Interface;
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::Buffer;
    use windows::Win32::System::WinRT::{
        IBufferByteAccess, RoInitialize, RoUninitialize, RO_INIT_SINGLETHREADED,
    };

    struct WinRtGuard;
    impl Drop for WinRtGuard {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    // scan_rgba runs on a short-lived worker thread. Native WinRT avoids the
    // former PowerShell process, temporary PNG, and repeated assembly loading.
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }.context("initializing WinRT OCR")?;
    let _winrt = WinRtGuard;

    let bytes = img.as_raw();
    let byte_len = u32::try_from(bytes.len()).context("OCR image is too large")?;
    let buffer = Buffer::Create(byte_len).context("allocating OCR bitmap buffer")?;
    buffer.SetLength(byte_len)?;
    let access: IBufferByteAccess = buffer.cast()?;
    let destination = unsafe { access.Buffer()? };
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len()) };

    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Rgba8,
        img.width() as i32,
        img.height() as i32,
        BitmapAlphaMode::Straight,
    )
    .context("creating native OCR bitmap")?;
    let engine =
        OcrEngine::TryCreateFromUserProfileLanguages().context("creating Windows OCR engine")?;
    let result = engine
        .RecognizeAsync(&bitmap)?
        .get()
        .context("recognizing inspect-panel text")?;
    let native_lines = result.Lines()?;
    let mut lines = Vec::new();
    for index in 0..native_lines.Size()? {
        let line = native_lines.GetAt(index)?;
        let text = line.Text()?.to_string_lossy().replace('\t', " ");
        if text.trim().is_empty() {
            continue;
        }
        let words = line.Words()?;
        let mut left = f32::MAX;
        let mut top = f32::MAX;
        let mut right = 0.0f32;
        let mut bottom = 0.0f32;
        for word_index in 0..words.Size()? {
            let rect = words.GetAt(word_index)?.BoundingRect()?;
            left = left.min(rect.X);
            top = top.min(rect.Y);
            right = right.max(rect.X + rect.Width);
            bottom = bottom.max(rect.Y + rect.Height);
        }
        if left == f32::MAX {
            left = 0.0;
            top = 0.0;
        }
        lines.push(OcrLine {
            text: text.trim().to_string(),
            x: left.max(0.0).round() as u32,
            y: top.max(0.0).round() as u32,
            w: (right - left).max(0.0).round() as u32,
            h: (bottom - top).max(0.0).round() as u32,
        });
    }
    if std::env::var_os("POEMERC_PROFILE_OCR").is_some() {
        eprintln!(
            "ocr {}x{}: {:.1} ms, {} lines",
            img.width(),
            img.height(),
            started.elapsed().as_secs_f64() * 1_000.0,
            lines.len()
        );
    }
    Ok(lines)
}
