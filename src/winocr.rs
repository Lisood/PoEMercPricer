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

#[cfg(windows)]
fn windows_ocr_lines(img: &RgbaImage) -> Result<Vec<OcrLine>> {
    let started = std::time::Instant::now();
    use windows::core::Interface;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::Buffer;
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::WinRT::{
        IBufferByteAccess, RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED,
    };

    struct WinRtGuard;
    impl Drop for WinRtGuard {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    // scan_rgba runs on a worker thread that never pumps messages, so use the
    // MTA. A thread already initialized in another mode keeps working; that
    // failed call must not be balanced by RoUninitialize.
    let _winrt = match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        Ok(()) => Some(WinRtGuard),
        Err(error) if error.code() == RPC_E_CHANGED_MODE => None,
        Err(error) => return Err(error).context("initializing WinRT OCR"),
    };

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
        BitmapAlphaMode::Ignore,
    )
    .context("creating native OCR bitmap")?;
    let languages = OcrEngine::AvailableRecognizerLanguages()?;
    if languages.Size()? == 0 {
        anyhow::bail!(
            "no Windows OCR language is installed; add English under Settings > Time & Language > Language & region > Add a language (with Optical character recognition), then scan again"
        );
    }
    // The game client is English regardless of the user's profile languages,
    // so any English OCR pack reads it (en-US first). A non-English engine
    // would silently produce garbage, so refuse rather than fall back.
    let tag = |language: &Language| language.LanguageTag().map(|t| t.to_string()).ok();
    let english = (0..languages.Size()?)
        .filter_map(|i| languages.GetAt(i).ok())
        .filter(|l| tag(l).is_some_and(|t| t.starts_with("en")))
        .min_by_key(|l| tag(l).as_deref() != Some("en-US"));
    let engine = english
        .and_then(|language| OcrEngine::TryCreateFromLanguage(&language).ok())
        .context("English is not installed for Windows OCR; add it under Settings > Time & Language > Language & region > Add a language (with Optical character recognition), then scan again")?;
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
