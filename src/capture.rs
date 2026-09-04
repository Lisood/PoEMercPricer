use anyhow::{Context, Result};
use arboard::ImageData;
use image::RgbaImage;
use std::path::{Path, PathBuf};
use xcap::Window;

/// Read an image from the system clipboard.
///
/// `Ok(None)` means that the clipboard does not currently contain an image.
/// Clipboard access/conversion failures are returned so callers can show a useful
/// status instead of silently falling through to an unrelated capture source.
pub fn clipboard_image_rgba() -> Result<Option<RgbaImage>> {
    fn read() -> std::result::Result<ImageData<'static>, arboard::Error> {
        arboard::Clipboard::new()?.get_image()
    }
    let mut data = read();
    if !matches!(data, Ok(_) | Err(arboard::Error::ContentNotAvailable)) {
        // Another process holding the clipboard open is transient; retry once.
        std::thread::sleep(std::time::Duration::from_millis(20));
        data = read();
    }
    let data = match data {
        Ok(data) => data,
        Err(arboard::Error::ContentNotAvailable) => return clipboard_image_file(),
        Err(error) => return Err(error).context("reading image from system clipboard"),
    };

    clipboard_image_data_to_rgba(data).map(Some)
}

/// Largest image side accepted from the clipboard or a copied file.
const MAX_IMAGE_SIDE: u32 = 8192;

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
    anyhow::ensure!(
        width <= MAX_IMAGE_SIDE && height <= MAX_IMAGE_SIDE,
        "clipboard image {width}x{height} exceeds {MAX_IMAGE_SIDE} px per side"
    );
    anyhow::ensure!(
        u64::from(width) * u64::from(height) <= 40_000_000,
        "clipboard image {width}x{height} exceeds 40,000,000 px total"
    );
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

/// Capture the Path of Exile window. Never falls back to the desktop: OCR-ing
/// whatever is on screen produced confident nonsense verdicts (an editor
/// window scanned as a "Kineticist" bricked by "Power Siphon").
pub fn capture_poe_or_primary(window_title: &str) -> Result<RgbaImage> {
    capture_named_window(window_title)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Path of Exile window not found (looking for a title starting with {window_title:?}); start the game or fix the window title in Settings"
        )
    })
}

pub fn capture_named_window(title_substr: &str) -> Result<Option<RgbaImage>> {
    anyhow::ensure!(
        !title_substr.trim().is_empty(),
        "PoE window title is empty in settings; an empty title would match every window"
    );
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
        Some((_, _, false)) => Err(anyhow::anyhow!(
            "the Path of Exile window is minimized; restore it and scan again"
        )),
        // PrintWindow returns Ok with a black bitmap for occluded or
        // exclusive-fullscreen DirectX clients, so treat blank like an error.
        Some((w, _, true)) => match w.capture_image() {
            Ok(image) if !is_blank_frame(&image) => Ok(Some(image)),
            window_result => {
                let reason = window_result.map_or_else(|e| e.to_string(), |_| "black frame".into());
                let image = capture_visible_monitor(&w)
                    .with_context(|| format!("capturing PoE window failed: {reason}"))?;
                anyhow::ensure!(
                    !is_blank_frame(&image),
                    "captured frame is black; set Path of Exile to Windowed Fullscreen"
                );
                Ok(Some(image))
            }
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

/// Sample every 64th pixel; fewer than 1% above luminance 16 means black.
fn is_blank_frame(img: &RgbaImage) -> bool {
    let pixels = img.pixels().step_by(64);
    let total = pixels.len().max(1);
    let lit = pixels
        .filter(|p| (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000 > 16)
        .count();
    lit * 100 < total
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
///
/// `Window::x/y/width/height` come from `GetWindowInfo`, which Windows scales
/// to a caller's own coordinate space when that caller is not DPI aware; the
/// GUI hotkey path only reaches this code after winit has declared the
/// process per-monitor DPI aware, but `dump-window-scan` can call it before
/// that happens, so the window rect is not reliably in the same physical
/// pixel space as `Monitor::x/y/width/height`. Rather than crop the monitor
/// image to a rect that might be at the wrong scale, only accept the monitor
/// capture when the window rect reports covering the whole monitor (exclusive
/// and borderless fullscreen games do), and otherwise fail the same way as an
/// unreadable `PrintWindow` result: a windowed game only fills part
/// of the monitor, and returning the full desktop for it would let downstream
/// geometry scan whatever is behind the game window.
fn capture_visible_monitor(window: &Window) -> Result<RgbaImage> {
    let monitor = window
        .current_monitor()
        .context("locating monitor containing PoE window")?;
    let win_x = i64::from(window.x().context("reading PoE window position")?);
    let win_y = i64::from(window.y().context("reading PoE window position")?);
    let win_w = i64::from(window.width().context("reading PoE window size")?);
    let win_h = i64::from(window.height().context("reading PoE window size")?);
    let mon_x = i64::from(monitor.x().context("reading monitor position")?);
    let mon_y = i64::from(monitor.y().context("reading monitor position")?);
    let mon_w = i64::from(monitor.width().context("reading monitor size")?);
    let mon_h = i64::from(monitor.height().context("reading monitor size")?);
    anyhow::ensure!(
        win_x <= mon_x
            && win_y <= mon_y
            && win_x + win_w >= mon_x + mon_w
            && win_y + win_h >= mon_y + mon_h,
        "captured frame is black; set Path of Exile to Windowed Fullscreen"
    );
    monitor
        .capture_image()
        .context("capturing visible PoE monitor")
}

pub fn load_image(path: &Path) -> Result<RgbaImage> {
    let mut reader = image::ImageReader::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("detecting image format of {}", path.display()))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(64 << 20);
    reader.limits(limits);
    let img = reader
        .decode()
        .with_context(|| format!("decoding {}", path.display()))?;
    Ok(img.into_rgba8())
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

    #[cfg(windows)]
    #[test]
    fn missing_game_window_is_an_error_not_a_desktop_scan() {
        let error = super::capture_poe_or_primary("zz-no-such-window-title-zz")
            .expect_err("no window matches");
        assert!(error.to_string().contains("window not found"), "{error}");
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
