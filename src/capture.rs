use anyhow::{Context, Result};
use arboard::ImageData;
use image::RgbaImage;
use std::path::{Path, PathBuf};
use xcap::{Monitor, Window};

/// Read an image from the system clipboard.
///
/// `Ok(None)` means that the clipboard does not currently contain an image.
/// Clipboard access/conversion failures are returned so callers can show a useful
/// status instead of silently falling through to an unrelated capture source.
pub fn clipboard_image_rgba() -> Result<Option<RgbaImage>> {
    let mut clipboard = arboard::Clipboard::new().context("opening system clipboard")?;
    let data = match clipboard.get_image() {
        Ok(data) => data,
        Err(arboard::Error::ContentNotAvailable) => {
            drop(clipboard);
            return clipboard_image_file();
        }
        Err(error) => return Err(error).context("reading image from system clipboard"),
    };

    clipboard_image_data_to_rgba(data).map(Some)
}

/// Decode the first image file copied in a file manager (Windows `CF_HDROP`).
/// This is distinct from copying pixels with an image editor/browser.
fn clipboard_image_file() -> Result<Option<RgbaImage>> {
    let Some(path) = clipboard_file_path()? else {
        return Ok(None);
    };
    if !is_supported_clipboard_image_path(&path) {
        return Ok(None);
    }

    load_image(&path)
        .with_context(|| format!("decoding copied image file {}", path.display()))
        .map(Some)
}

fn is_supported_clipboard_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        })
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn clipboard_file_path() -> Result<Option<PathBuf>> {
    Ok(None)
}

#[cfg(windows)]
fn clipboard_file_path() -> Result<Option<PathBuf>> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStringExt;

    const CF_HDROP: u32 = 15;
    const FIRST_FILE: u32 = 0;

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(owner: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn IsClipboardFormatAvailable(format: u32) -> i32;
        fn GetClipboardData(format: u32) -> *mut c_void;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn DragQueryFileW(
            drop_handle: *mut c_void,
            file_index: u32,
            file_path: *mut u16,
            file_path_capacity: u32,
        ) -> u32;
    }

    struct ClipboardGuard;
    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            // SAFETY: This guard is created only after OpenClipboard succeeds,
            // and exactly one guard owns the matching CloseClipboard call.
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }

    // SAFETY: All clipboard handles remain valid while ClipboardGuard keeps the
    // clipboard open. DragQueryFileW owns neither the handle nor output buffer;
    // the second call receives the exact UTF-16 capacity requested by the first.
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            anyhow::bail!("opening system clipboard for a copied image file");
        }
        let _guard = ClipboardGuard;

        if IsClipboardFormatAvailable(CF_HDROP) == 0 {
            return Ok(None);
        }
        let drop_handle = GetClipboardData(CF_HDROP);
        if drop_handle.is_null() {
            anyhow::bail!("reading copied file list from system clipboard");
        }

        let utf16_len = DragQueryFileW(drop_handle, FIRST_FILE, std::ptr::null_mut(), 0);
        if utf16_len == 0 {
            return Ok(None);
        }
        let capacity = utf16_len
            .checked_add(1)
            .context("copied image path length overflow")?;
        let mut utf16 = vec![0_u16; capacity as usize];
        let copied = DragQueryFileW(drop_handle, FIRST_FILE, utf16.as_mut_ptr(), capacity);
        anyhow::ensure!(
            copied == utf16_len,
            "copied image path changed while reading clipboard"
        );
        utf16.truncate(copied as usize);

        Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(&utf16))))
    }
}

/// Convert arboard's tightly packed, row-major RGBA clipboard representation.
///
/// Keeping this conversion separate makes the byte layout testable without a
/// real desktop clipboard, which is inherently process- and timing-dependent.
pub fn clipboard_image_data_to_rgba(data: ImageData<'_>) -> Result<RgbaImage> {
    anyhow::ensure!(
        data.width > 0 && data.height > 0,
        "invalid clipboard image dimensions: {}x{}",
        data.width,
        data.height
    );
    let width = u32::try_from(data.width).context("clipboard image width exceeds u32")?;
    let height = u32::try_from(data.height).context("clipboard image height exceeds u32")?;
    let expected_len = data
        .width
        .checked_mul(data.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .context("clipboard image dimensions overflow")?;

    anyhow::ensure!(
        data.bytes.len() == expected_len,
        "invalid clipboard RGBA data: {}x{} requires {} bytes, received {}",
        data.width,
        data.height,
        expected_len,
        data.bytes.len()
    );

    RgbaImage::from_raw(width, height, data.bytes.into_owned())
        .context("constructing RGBA image from clipboard data")
}

pub fn capture_poe_or_primary(window_title: &str) -> Result<RgbaImage> {
    if let Some(img) = capture_named_window(window_title)? {
        return Ok(img);
    }
    capture_primary()
}

pub fn capture_named_window(title_substr: &str) -> Result<Option<RgbaImage>> {
    let started = std::time::Instant::now();
    let windows = Window::all().context("enumerating windows")?;
    let enumerated_at = std::time::Instant::now();
    let mut best: Option<(Window, u8, bool)> = None;
    for w in windows {
        let Ok(title) = w.title() else { continue };
        let Some(rank) = window_title_match_rank(&title, title_substr) else {
            continue;
        };
        let usable = !w.is_minimized().unwrap_or(false);
        let should_replace = best
            .as_ref()
            .map(|(_, best_rank, best_usable)| (rank, usable) > (*best_rank, *best_usable))
            .unwrap_or(true);
        if should_replace {
            best = Some((w, rank, usable));
        }
        // An exact, non-minimized match cannot be improved upon. In
        // particular, do not accidentally capture a browser whose title only
        // starts with "Path of Exile" because it is above the game in Z order.
        if rank == 2 && usable {
            break;
        }
    }
    let selected_at = std::time::Instant::now();
    let result = match best {
        Some((w, _, _)) => match w.capture_image() {
            Ok(image) => Ok(Some(image)),
            Err(window_error) => capture_visible_monitor(&w)
                .with_context(|| format!("capturing PoE window failed: {window_error}"))
                .map(Some),
        },
        None => Ok(None),
    };
    if capture_profiling_enabled() {
        eprintln!(
            "capture window: enumerate {:.1} ms, select {:.1} ms, pixels {:.1} ms",
            enumerated_at.duration_since(started).as_secs_f64() * 1_000.0,
            selected_at.duration_since(enumerated_at).as_secs_f64() * 1_000.0,
            selected_at.elapsed().as_secs_f64() * 1_000.0
        );
    }
    result
}

fn window_title_match_rank(candidate: &str, needle: &str) -> Option<u8> {
    if candidate.eq_ignore_ascii_case(needle) {
        Some(2)
    } else if candidate
        .to_ascii_lowercase()
        .starts_with(&needle.to_ascii_lowercase())
    {
        Some(1)
    } else {
        None
    }
}

fn capture_profiling_enabled() -> bool {
    std::env::var_os("POEMERC_PROFILE_CAPTURE").is_some()
        || std::env::var_os("POEMERC_PROFILE_OCR").is_some()
}

/// Capture the pixels currently displayed on the monitor containing `window`.
///
/// Path of Exile renders through DirectX. Capturing the visible output avoids
/// the synchronous `PrintWindow` request used by `Window::capture_image`, which
/// can return a stale back-buffer for hardware-accelerated games.
fn capture_visible_monitor(window: &Window) -> Result<RgbaImage> {
    window
        .current_monitor()
        .context("locating monitor containing PoE window")?
        .capture_image()
        .context("capturing visible PoE monitor")
}

pub fn capture_primary() -> Result<RgbaImage> {
    let monitors = Monitor::all().context("enumerating monitors")?;
    let mon = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or_else(|| anyhow::anyhow!("no primary monitor"))?;
    mon.capture_image().context("capturing primary monitor")
}

pub fn load_image(path: &Path) -> Result<RgbaImage> {
    let img = image::open(path).with_context(|| format!("opening {}", path.display()))?;
    Ok(img.to_rgba8())
}

pub fn save_debug(img: &RgbaImage, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(path)
        .with_context(|| format!("saving {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        clipboard_image_data_to_rgba, clipboard_image_rgba, is_supported_clipboard_image_path,
    };
    use arboard::ImageData;
    use image::Rgba;
    use std::borrow::Cow;

    #[test]
    fn exact_window_title_outranks_prefix_and_matching_is_case_insensitive() {
        assert_eq!(
            super::window_title_match_rank("Path of Exile", "path of exile"),
            Some(2)
        );
        assert_eq!(
            super::window_title_match_rank("Path of Exile - Search", "Path of Exile"),
            Some(1)
        );
        assert_eq!(
            super::window_title_match_rank("PoEMercPricer", "Path of Exile"),
            None
        );
    }

    /// Manual timing probe for the real Path of Exile window. This remains
    /// ignored because CI has neither a desktop nor a running game client.
    #[cfg(windows)]
    #[test]
    #[ignore = "requires a visible Path of Exile window"]
    fn live_poe_window_capture_timing() {
        let windows = xcap::Window::all().expect("enumerate windows");
        let window = windows
            .iter()
            .find(|window| {
                window
                    .title()
                    .map(|title| title.starts_with("Path of Exile"))
                    .unwrap_or(false)
            })
            .expect("visible Path of Exile window");

        let started = std::time::Instant::now();
        let image = window.capture_image().expect("legacy window capture");
        let legacy_elapsed = started.elapsed();
        let started = std::time::Instant::now();
        let visible = super::capture_visible_monitor(window).expect("visible monitor capture");
        let visible_elapsed = started.elapsed();
        eprintln!(
            "window={} ms {}x{}; visible-monitor={} ms {}x{}",
            legacy_elapsed.as_millis(),
            image.width(),
            image.height(),
            visible_elapsed.as_millis(),
            visible.width(),
            visible.height()
        );
        let different_pixels = image
            .pixels()
            .zip(visible.pixels())
            .filter(|(left, right)| left != right)
            .count();
        eprintln!("different pixels={different_pixels}");
        assert!(image.width() >= 640 && image.height() >= 480);
        assert!(visible.width() >= image.width() && visible.height() >= image.height());
    }

    /// Measures persistent-process behaviour: the first call includes lazy OCR
    /// and support-template initialization; the second call exercises the hot
    /// path used by subsequent GUI scans.
    #[cfg(windows)]
    #[test]
    #[ignore = "requires a visible Path of Exile Warrant panel"]
    fn live_two_consecutive_capture_and_scan_timings() {
        for pass in 1..=2 {
            let started = std::time::Instant::now();
            let image = super::capture_named_window("Path of Exile")
                .expect("capture query")
                .expect("visible Path of Exile window");
            let captured_at = std::time::Instant::now();
            let merc = poemercpricer::scan::scan_rgba(&image).expect("scan captured panel");
            eprintln!(
                "pass {pass}: capture={:.1} ms scan={:.1} ms total={:.1} ms skills={}",
                captured_at.duration_since(started).as_secs_f64() * 1_000.0,
                captured_at.elapsed().as_secs_f64() * 1_000.0,
                started.elapsed().as_secs_f64() * 1_000.0,
                merc.skills.len()
            );
        }
    }

    #[test]
    fn clipboard_rgba_conversion_preserves_row_major_pixel_layout() {
        // Two rows of three distinct pixels. This catches width/height swaps,
        // vertical flips, channel reordering, and accidental row transposition.
        let bytes = vec![
            1, 2, 3, 4, // (0, 0)
            5, 6, 7, 8, // (1, 0)
            9, 10, 11, 12, // (2, 0)
            13, 14, 15, 16, // (0, 1)
            17, 18, 19, 20, // (1, 1)
            21, 22, 23, 24, // (2, 1)
        ];
        let image = clipboard_image_data_to_rgba(ImageData {
            width: 3,
            height: 2,
            bytes: Cow::Owned(bytes),
        })
        .expect("valid clipboard pixels");

        assert_eq!(image.dimensions(), (3, 2));
        assert_eq!(*image.get_pixel(0, 0), Rgba([1, 2, 3, 4]));
        assert_eq!(*image.get_pixel(2, 0), Rgba([9, 10, 11, 12]));
        assert_eq!(*image.get_pixel(0, 1), Rgba([13, 14, 15, 16]));
        assert_eq!(*image.get_pixel(2, 1), Rgba([21, 22, 23, 24]));
    }

    #[test]
    fn clipboard_rgba_conversion_rejects_truncated_or_padded_rows() {
        for bytes in [vec![0; 23], vec![0; 25]] {
            let error = clipboard_image_data_to_rgba(ImageData {
                width: 3,
                height: 2,
                bytes: Cow::Owned(bytes),
            })
            .expect_err("a 3x2 RGBA image must contain exactly 24 bytes");

            assert!(error.to_string().contains("requires 24 bytes"));
        }
    }

    #[test]
    fn clipboard_rgba_conversion_rejects_empty_dimensions() {
        let error = clipboard_image_data_to_rgba(ImageData {
            width: 0,
            height: 0,
            bytes: Cow::Borrowed(&[]),
        })
        .expect_err("an empty image is not useful for OCR");

        assert!(error.to_string().contains("dimensions: 0x0"));
    }

    #[test]
    fn copied_file_filter_accepts_decodable_image_extensions_case_insensitively() {
        assert!(is_supported_clipboard_image_path(std::path::Path::new(
            r"samples\manyshot_alara.png"
        )));
        assert!(is_supported_clipboard_image_path(std::path::Path::new(
            r"C:\screenshots\MERC.JPEG"
        )));
        assert!(is_supported_clipboard_image_path(std::path::Path::new(
            "merc.webp"
        )));
        assert!(!is_supported_clipboard_image_path(std::path::Path::new(
            "warrant.txt"
        )));
    }

    /// Manual acceptance check for the real Windows clipboard. Copy
    /// `samples\manyshot_alara.png` in Explorer before running this ignored test.
    #[cfg(windows)]
    #[test]
    #[ignore = "requires samples\\manyshot_alara.png on the desktop clipboard"]
    fn live_explorer_clipboard_decodes_alara_image() {
        let image = clipboard_image_rgba()
            .expect("read Windows clipboard")
            .expect("clipboard image or copied image file");
        assert_eq!(image.dimensions(), (850, 1189));
    }
}
