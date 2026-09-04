use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::LazyLock;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use eframe::egui::{
    self, Color32, FontFamily, FontId, Frame, Margin, RichText, Stroke, TextFormat, TextStyle,
    TextureHandle, Ui, Vec2, Visuals,
};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::capture;
use poemercpricer::catalog::{
    canonical_support, skill_icon, support_display_for_tier, support_icon,
};
use poemercpricer::config::AppConfig;
use poemercpricer::models::{kineticist_jackpot_fixture, Mercenary, ScoreResult};
use poemercpricer::parse::{looks_like_warrant, parse_warrant_text};
use poemercpricer::pricing::{estimate_for, MarketEstimate};
use poemercpricer::scoring::score_mercenary;
use poemercpricer::summary::summary;
use poemercpricer::update::{self, UpdateState};

static POE_SERIF: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("poe_serif".into()));

/// One consistent set of colours for a frame, chosen once by `palette_for`
/// from `AppConfig::theme` (and the OS theme, for `"system"`). Every drawing
/// function takes `&Palette` instead of reaching for a global constant, so a
/// theme switch repaints correctly without any widget carrying stale colour.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Palette {
    bg: Color32,
    surface: Color32,
    raised: Color32,
    border: Color32,
    border_dark: Color32,
    text: Color32,
    muted: Color32,
    subtle: Color32,
    gold: Color32,
    gold_fill: Color32,
    gold_fill_text: Color32,
    orange: Color32,
    red: Color32,
    green: Color32,
    take_green: Color32,
    check: Color32,
    brick_row: Color32,
    hovered: Color32,
    /// Verdict-box fill alpha for a take-item result (`accent.gamma_multiply(fill_take)`).
    fill_take: f32,
    /// Verdict-box fill alpha for a "skip" result.
    fill_skip: f32,
    /// Verdict-box fill alpha for a "check" result.
    fill_check: f32,
    /// Verdict-box fill alpha for every other band (including "unsupported").
    fill_none: f32,
    /// Fill alpha for an error block accent (RED.gamma_multiply(fill_error)).
    fill_error: f32,
}

const STANDARD: Palette = Palette {
    bg: Color32::from_rgb(10, 9, 8),
    surface: Color32::from_rgb(19, 17, 14),
    raised: Color32::from_rgb(29, 25, 20),
    border: Color32::from_rgb(79, 64, 43),
    border_dark: Color32::from_rgb(44, 36, 26),
    text: Color32::from_rgb(224, 214, 195),
    muted: Color32::from_rgb(176, 161, 137),
    subtle: Color32::from_rgb(154, 139, 114),
    gold: Color32::from_rgb(199, 160, 91),
    gold_fill: Color32::from_rgb(179, 144, 82),
    gold_fill_text: Color32::from_rgb(10, 9, 8),
    orange: Color32::from_rgb(211, 106, 40),
    red: Color32::from_rgb(232, 118, 112),
    green: Color32::from_rgb(130, 200, 137),
    take_green: Color32::from_rgb(96, 230, 120),
    check: Color32::from_rgb(178, 178, 78),
    brick_row: Color32::from_rgb(34, 16, 14),
    hovered: Color32::from_rgb(53, 42, 27),
    fill_take: 0.30,
    fill_skip: 0.12,
    fill_check: 0.12,
    fill_none: 0.12,
    fill_error: 0.10,
};

const DARK: Palette = Palette {
    bg: Color32::from_rgb(0, 0, 0),
    surface: Color32::from_rgb(14, 14, 14),
    raised: Color32::from_rgb(24, 24, 24),
    border: Color32::from_rgb(51, 51, 48),
    border_dark: Color32::from_rgb(34, 34, 32),
    text: Color32::from_rgb(236, 235, 232),
    muted: Color32::from_rgb(168, 166, 159),
    subtle: Color32::from_rgb(140, 138, 131),
    gold: Color32::from_rgb(212, 168, 87),
    gold_fill: Color32::from_rgb(199, 160, 91),
    gold_fill_text: Color32::from_rgb(0, 0, 0),
    orange: Color32::from_rgb(224, 122, 58),
    red: Color32::from_rgb(239, 122, 116),
    green: Color32::from_rgb(130, 200, 137),
    take_green: Color32::from_rgb(96, 230, 120),
    check: Color32::from_rgb(178, 178, 78),
    brick_row: Color32::from_rgb(36, 16, 16),
    hovered: Color32::from_rgb(36, 36, 36),
    fill_take: 0.30,
    fill_skip: 0.12,
    fill_check: 0.12,
    fill_none: 0.12,
    fill_error: 0.10,
};

const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(255, 255, 255),
    surface: Color32::from_rgb(246, 246, 247),
    raised: Color32::from_rgb(255, 255, 255),
    border: Color32::from_rgb(208, 208, 213),
    border_dark: Color32::from_rgb(236, 236, 239),
    text: Color32::from_rgb(17, 17, 20),
    muted: Color32::from_rgb(107, 107, 114),
    // #86868E from the spec table fails 4.5:1 on the raised surface
    // (3.61:1); darkened at the same hue/saturation to clear the gate.
    subtle: Color32::from_rgb(112, 112, 119),
    gold: Color32::from_rgb(161, 98, 7),
    gold_fill: Color32::from_rgb(240, 180, 41),
    gold_fill_text: Color32::from_rgb(17, 17, 20),
    orange: Color32::from_rgb(194, 65, 12),
    // #DC2626 clears 4.5:1 on bg/raised but not on the surface tone
    // (4.47:1); darkened slightly at the same hue to clear all three.
    red: Color32::from_rgb(218, 35, 35),
    green: Color32::from_rgb(21, 128, 61),
    take_green: Color32::from_rgb(21, 128, 61),
    check: Color32::from_rgb(122, 106, 10),
    brick_row: Color32::from_rgb(254, 242, 242),
    hovered: Color32::from_rgb(237, 237, 239),
    fill_take: 0.10,
    fill_skip: 0.08,
    fill_check: 0.09,
    fill_none: 0.10,
    fill_error: 0.07,
};

/// Pure so the fallback rules are unit-tested instead of eyeballed: `"system"`
/// with no OS answer, and any unrecognized string, both land on `STANDARD`.
fn palette_for(theme: &str, system: Option<egui::Theme>) -> Palette {
    match theme {
        "dark" => DARK,
        "light" => LIGHT,
        "system" => match system {
            Some(egui::Theme::Dark) => DARK,
            Some(egui::Theme::Light) => LIGHT,
            None => STANDARD,
        },
        _ => STANDARD,
    }
}

const TRADE_OPEN_COOLDOWN: Duration = Duration::from_secs(2);
/// `Windows.Media.Ocr` and the window capture can both block indefinitely (a
/// display change during capture is the realistic case). A wedged worker never
/// sends a `ScanMsg`, so after this long a scan stops counting as in progress:
/// Scan and Clipboard become clickable again and a new scan orphans the old
/// worker instead of waiting for it.
const SCAN_WATCHDOG: Duration = Duration::from_secs(30);
/// Awakened PoE Trade re-checks every 16 h; a Path of Exile patch can land
/// mid-session and one GET is cheap, so this is shorter.
const UPDATE_RECHECK: Duration = Duration::from_secs(6 * 60 * 60);

/// Scan results carry the generation of the scan that produced them, so a
/// result arriving late from a worker the watchdog already abandoned is
/// dropped instead of overwriting the scan that replaced it.
enum ScanMsg {
    /// Result, elapsed time, and the input source ("clipboard", "screen", ...).
    Ok(u64, Box<(Mercenary, ScoreResult, Duration, &'static str)>),
    Err(u64, String),
    HotkeyPressed,
    /// One update-worker state change: `Checking` first, then a terminal state.
    Update(UpdateState),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScanRequest {
    Hotkey,
    Screen,
    Clipboard,
    Image(PathBuf),
}

/// Keyed on the raw skill name / support id so the catalog lookup
/// (`skill_icon` normalizes 267 names) runs once per name, not per frame.
#[derive(Default)]
struct IconCache {
    skills: HashMap<String, Option<TextureHandle>>,
    supports: HashMap<String, Option<TextureHandle>>,
}

impl IconCache {
    fn skill(&mut self, ctx: &egui::Context, name: &str) -> Option<TextureHandle> {
        if let Some(cached) = self.skills.get(name) {
            return cached.clone();
        }
        let texture = skill_icon(name).and_then(|file| load_icon(ctx, "skills", file));
        self.skills.insert(name.to_owned(), texture.clone());
        texture
    }

    fn support(&mut self, ctx: &egui::Context, canonical: &str) -> Option<TextureHandle> {
        if let Some(cached) = self.supports.get(canonical) {
            return cached.clone();
        }
        let texture = support_icon(canonical).and_then(|file| load_icon(ctx, "supports", file));
        self.supports.insert(canonical.to_owned(), texture.clone());
        texture
    }
}

fn load_icon(ctx: &egui::Context, kind: &str, file: &str) -> Option<TextureHandle> {
    let image = image::load_from_memory(poemercpricer::icons::bytes(kind, file)?)
        .ok()?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Some(ctx.load_texture(
        format!("{kind}/{file}"),
        color,
        egui::TextureOptions::LINEAR,
    ))
}

pub struct PricerApp {
    cfg: AppConfig,
    palette: Palette,
    status: String,
    scanning: bool,
    /// When the in-flight scan was spawned, for the [`SCAN_WATCHDOG`] check.
    scan_started: Option<Instant>,
    /// Incremented on every start; a worker reports back under the value it
    /// was spawned with (see [`ScanMsg`]).
    scan_generation: u64,
    show_settings: bool,
    /// True while the Settings pane is capturing a key combination for the
    /// scan hotkey; Escape cancels the capture instead of closing the pane.
    hotkey_recording: bool,
    /// The market estimate is computed once per result change, never per frame.
    result: Option<(Mercenary, ScoreResult, Option<MarketEstimate>)>,
    error: Option<String>,
    tx: Sender<ScanMsg>,
    rx: Receiver<ScanMsg>,
    _hotkeys: Option<GlobalHotKeyManager>,
    egui_ctx: egui::Context,
    applied_always_on_top: bool,
    last_scan_duration: Option<Duration>,
    pending_image: Option<PathBuf>,
    hwnd: Option<isize>,
    icon_cache: IconCache,
    last_trade_open: Option<Instant>,
    text_field_focused: bool,
    update_state: UpdateState,
    /// True while the visible update state came from a button the user pressed,
    /// which is the only case that writes the status line.
    update_manual: bool,
    /// Captured at startup: Windows keeps reporting the load-time path even
    /// after the file is replaced underneath the running process.
    exe: Option<PathBuf>,
    no_updates: bool,
    first_frame: bool,
    /// When the last check started, not when it finished: a failed check should
    /// wait out the same interval before trying again.
    last_update_check: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SupportChoice {
    skill_index: usize,
    support_index: usize,
    canonical: String,
}

/// Win32 HWND of the overlay window, for raw SetWindowPos topmost control.
/// eframe's own with_always_on_top()/WindowLevel keeps glow in a continuous
/// redraw+present loop on Windows 11 (the overlay flicker), so it is bypassed.
fn window_hwnd(cc: &eframe::CreationContext<'_>) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match cc.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
        _ => None,
    }
}

#[cfg(windows)]
fn set_topmost(hwnd: isize, on: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };
    unsafe {
        let _ = SetWindowPos(
            HWND(hwnd as *mut core::ffi::c_void),
            if on { HWND_TOPMOST } else { HWND_NOTOPMOST },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(windows))]
fn set_topmost(_hwnd: isize, _on: bool) {}

/// A window hidden with `Visible(false)` gets no WM_PAINT, so `update()` never
/// runs to process the re-show command. Show it directly from the hotkey
/// handler, which runs on the winit thread.
#[cfg(windows)]
fn show_window(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
    unsafe {
        let _ = ShowWindow(HWND(hwnd as *mut core::ffi::c_void), SW_SHOWNOACTIVATE);
    }
}

#[cfg(not(windows))]
fn show_window(_hwnd: isize) {}

impl PricerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        mut cfg: AppConfig,
        cfg_error: Option<String>,
        pending_image: Option<PathBuf>,
        no_updates: bool,
    ) -> Self {
        install_fonts(&cc.egui_ctx);
        let palette = palette_for(&cfg.theme, cc.egui_ctx.system_theme());
        apply_theme(&cc.egui_ctx, &palette);
        let (tx, rx) = mpsc::channel();
        let hwnd = window_hwnd(cc);
        let (hotkeys, scan_hotkey, hotkey_error) = match register_hotkey(&cfg.hotkey) {
            Ok((manager, hk)) => (Some(manager), Some(hk), None),
            Err(e) => (
                None,
                None,
                Some(format!("Hotkey {} unavailable: {e:#}", cfg.hotkey)),
            ),
        };
        if hotkey_error.is_some() {
            // Without a hotkey nothing can bring a hidden window back, so this
            // session ignores the setting; the config file keeps it.
            cfg.hide_on_escape = false;
        }
        install_hotkey_handler(scan_hotkey, tx.clone(), cc.egui_ctx.clone(), hwnd);
        let applied_always_on_top = cfg.always_on_top;
        if cfg.always_on_top {
            if let Some(h) = hwnd {
                set_topmost(h, true);
            }
        }
        let error = cfg_error.or(hotkey_error);
        let mut app = Self {
            status: error
                .clone()
                .unwrap_or_else(|| format!("Press {} to scan a mercenary panel", cfg.hotkey)),
            cfg,
            palette,
            scanning: false,
            scan_started: None,
            scan_generation: 0,
            show_settings: false,
            hotkey_recording: false,
            result: None,
            error,
            tx,
            rx,
            _hotkeys: hotkeys,
            egui_ctx: cc.egui_ctx.clone(),
            applied_always_on_top,
            last_scan_duration: None,
            pending_image,
            hwnd,
            icon_cache: IconCache::default(),
            last_trade_open: None,
            text_field_focused: false,
            update_state: UpdateState::Idle,
            update_manual: false,
            exe: std::env::current_exe().ok(),
            no_updates,
            first_frame: true,
            last_update_check: None,
        };
        if let Some(path) = app.pending_image.take() {
            app.start_scan(ScanRequest::Image(path));
        }
        app
    }

    pub fn with_fixture(
        cc: &eframe::CreationContext<'_>,
        cfg: AppConfig,
        cfg_error: Option<String>,
        no_updates: bool,
    ) -> Self {
        let mut app = Self::new(cc, cfg, cfg_error, None, no_updates);
        let (merc, result) = kineticist_jackpot_fixture();
        app.apply_result(merc, result, "fixture");
        if app.error.is_none() {
            app.status = "Fixture: premium Kineticist (no game required)".into();
        }
        app
    }

    /// True while a scan counts as in progress: it blocks a new one and greys
    /// out the buttons. A scan past the watchdog stops counting.
    fn scan_busy(&self) -> bool {
        self.scanning && !scan_is_stuck(self.scanning, self.scan_started)
    }

    fn start_scan(&mut self, request: ScanRequest) {
        let abandoned = scan_is_stuck(self.scanning, self.scan_started);
        if self.scanning && !abandoned {
            self.status = "Scan already in progress".into();
            return;
        }
        self.scanning = true;
        self.scan_started = Some(Instant::now());
        self.scan_generation += 1;
        let generation = self.scan_generation;
        self.error = None;
        let action = match &request {
            ScanRequest::Clipboard => "Reading clipboard…",
            ScanRequest::Image(_) => "Scanning image…",
            ScanRequest::Hotkey | ScanRequest::Screen => "Capturing Path of Exile…",
        };
        self.status = if abandoned {
            format!("Previous scan was still running and was abandoned. {action}")
        } else {
            action.to_owned()
        };
        let cfg = self.cfg.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        thread::spawn(move || {
            let started = Instant::now();
            // A panic here must still report back, or `scanning` stays true
            // until the watchdog expires.
            let msg = match std::panic::catch_unwind(AssertUnwindSafe(|| run_scan(&cfg, request))) {
                Ok(Ok((m, r, source))) => {
                    ScanMsg::Ok(generation, Box::new((m, r, started.elapsed(), source)))
                }
                Ok(Err(e)) => ScanMsg::Err(generation, format!("{e:#}")),
                Err(_) => ScanMsg::Err(
                    generation,
                    "scan panicked; see the console for details".into(),
                ),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    fn apply_result(&mut self, merc: Mercenary, result: ScoreResult, source: &str) {
        self.status = format!("Scan complete ({source})");
        let market = estimate_for(&merc, !result.bricks.is_empty());
        self.result = Some((merc, result, market));
    }

    fn copy_summary(&self) -> Result<()> {
        let Some((merc, result, market)) = &self.result else {
            return Ok(());
        };
        let s = summary(merc, result, market.as_ref());
        with_clipboard(|clip| clip.set_text(s.as_str())).context("writing the clipboard")
    }

    fn open_official_trade(&mut self) {
        if self.scan_busy() {
            self.status = "Finish the current scan before opening trade".into();
            return;
        }
        if self
            .last_trade_open
            .is_some_and(|opened| opened.elapsed() < TRADE_OPEN_COOLDOWN)
        {
            self.status = "Trade search already opened".into();
            return;
        }
        let Some((merc, _, _)) = &self.result else {
            return;
        };
        let search = match poemercpricer::trade::trade_search(
            merc,
            &self.cfg.trade_league,
            self.cfg.trade_every_skill,
        ) {
            Ok(search) => search,
            Err(error) => {
                self.error = Some(error.to_string());
                self.status = format!("Could not build trade search: {error}");
                return;
            }
        };
        if let Err(error) = open::that_detached(&search.url) {
            self.error = Some(error.to_string());
            self.status = format!("Could not open Path of Exile trade: {error}");
            return;
        }
        self.error = None;
        self.last_trade_open = Some(Instant::now());
        self.status = if search.included_skills > 1 {
            format!(
                "Opened {} trade: {} of {} skills, {} exact filters",
                self.cfg.trade_league,
                search.included_skills,
                search.available_skills,
                search.included_filters
            )
        } else {
            format!(
                "Opened {} trade: {} with {} exact filters (1 of {} skills; log in and enable \"Search every skill\" for the whole warrant)",
                self.cfg.trade_league,
                search.selected_skill,
                search.included_filters,
                search.available_skills
            )
        };
        if !search.dropped.is_empty() {
            self.status += &format!("; skipped unresolved: {}", search.dropped.join(", "));
        }
    }

    /// Spawns the GitHub lookup, and the install straight after it when
    /// "Install updates automatically" is on. The blocking work never touches
    /// the UI thread; the worker reports back on the scan channel.
    /// The same rule for the startup check and every re-check: release builds
    /// only, the setting on, and no `--no-updates`. **Check now** ignores it.
    fn update_checks_enabled(&self) -> bool {
        !cfg!(debug_assertions) && self.cfg.check_updates && !self.no_updates
    }

    fn start_update_check(&mut self, manual: bool) {
        if matches!(
            self.update_state,
            UpdateState::Checking | UpdateState::Downloading(_)
        ) {
            return;
        }
        self.last_update_check = Some(Instant::now());
        self.update_manual = manual;
        self.update_state = UpdateState::Checking;
        if manual {
            self.error = None;
            self.status = "Checking GitHub for a newer release…".into();
        }
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        let exe = self.exe.clone();
        // Check now works in a debug build, but the swap never does: it would
        // replace target/debug/poemercpricer.exe.
        let auto_install = self.cfg.install_updates_automatically && !cfg!(debug_assertions);
        thread::spawn(move || {
            let work = || match update::check() {
                Ok(None) => UpdateState::UpToDate {
                    checked: Instant::now(),
                },
                Ok(Some(release)) => match (auto_install, &exe) {
                    (true, Some(exe)) => {
                        let _ = tx.send(ScanMsg::Update(UpdateState::Downloading(release.clone())));
                        ctx.request_repaint();
                        installed_state(&release, exe)
                    }
                    _ => UpdateState::Available(release),
                },
                Err(e) => UpdateState::Failed {
                    message: format!("{e:#}"),
                },
            };
            // A panic here must still report back, or the state stays Checking.
            let state =
                std::panic::catch_unwind(AssertUnwindSafe(work)).unwrap_or(UpdateState::Failed {
                    message: "update worker panicked; see the console".into(),
                });
            let _ = tx.send(ScanMsg::Update(state));
            ctx.request_repaint();
        });
    }

    /// The manual half of the check: download and swap a release the user has
    /// already been shown.
    fn start_install(&mut self) {
        let UpdateState::Available(release) = self.update_state.clone() else {
            return;
        };
        self.update_manual = true;
        self.error = None;
        let Some(exe) = self.exe.clone() else {
            let message = "cannot locate poemercpricer.exe".to_string();
            self.error = Some(message.clone());
            self.status = format!("Could not update: {message}");
            self.update_state = UpdateState::Failed { message };
            return;
        };
        self.status = format!(
            "Downloading {} ({} MB)…",
            release.version,
            megabytes(release.size)
        );
        self.update_state = UpdateState::Downloading(release.clone());
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        thread::spawn(move || {
            let state =
                std::panic::catch_unwind(AssertUnwindSafe(|| installed_state(&release, &exe)))
                    .unwrap_or(UpdateState::Failed {
                        message: "update worker panicked; see the console".into(),
                    });
            let _ = tx.send(ScanMsg::Update(state));
            ctx.request_repaint();
        });
    }

    /// Only ever from a button press; the app never restarts on its own.
    fn restart(&mut self, ctx: &egui::Context) {
        let Some(exe) = self.exe.clone() else {
            let message = "cannot locate poemercpricer.exe";
            self.error = Some(message.to_string());
            self.status = format!("Could not restart: {message}");
            return;
        };
        // Dropping GlobalHotKeyManager calls DestroyWindow on its hidden window,
        // which frees the RegisterHotKey binding, so the new process can claim
        // the same hotkey without racing this one.
        let _ = self._hotkeys.take();
        match std::process::Command::new(&exe)
            .args(std::env::args_os().skip(1))
            .spawn()
        {
            Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(error) => {
                // The restart did not happen, so this process keeps running:
                // take the hotkey back.
                self._hotkeys = register_hotkey(&self.cfg.hotkey).ok().map(|(m, _)| m);
                self.error = Some(error.to_string());
                self.status = format!("Could not restart: {error}");
            }
        }
    }
}

impl eframe::App for PricerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let palette = palette_for(&self.cfg.theme, ctx.system_theme());
        if palette != self.palette {
            self.palette = palette;
            apply_theme(ctx, &palette);
        }
        if self.first_frame {
            self.first_frame = false;
            // Debug builds never self-update: `cargo run` in a checkout must not
            // replace target/release/poemercpricer.exe under the developer.
            if self.update_checks_enabled() {
                self.start_update_check(false);
            }
        }
        // Periodic re-check without a timer thread. eframe turns the last
        // frame's request_repaint_after into a WaitUntil, so asking on every
        // frame only moves that one deadline and never repaints continuously.
        if let Some(wait) = recheck_in(
            self.last_update_check,
            &self.update_state,
            self.update_checks_enabled(),
        ) {
            if wait.is_zero() {
                self.start_update_check(false);
            } else {
                ctx.request_repaint_after(wait);
            }
        }
        // A wedged worker never sends a message, so ask for one wake-up at the
        // watchdog deadline. egui keeps the shortest pending deadline, so this
        // coexists with the update re-check above and adds no per-frame repaint.
        if let Some(wake) = scan_watchdog_wake(self.scanning, self.scan_started) {
            ctx.request_repaint_after(wake);
        }

        if let Some(always_on_top) =
            take_always_on_top_change(&mut self.applied_always_on_top, self.cfg.always_on_top)
        {
            // Raw SetWindowPos, not ViewportCommand::WindowLevel: the winit
            // path re-enters the topmost redraw loop (see window_hwnd docs).
            if let Some(h) = self.hwnd {
                set_topmost(h, always_on_top);
            }
        }

        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                // An abandoned worker can still finish; its generation is stale,
                // so it must not overwrite the scan that replaced it.
                ScanMsg::Ok(generation, _) | ScanMsg::Err(generation, _)
                    if generation != self.scan_generation => {}
                ScanMsg::Ok(_, boxed) => {
                    self.scanning = false;
                    self.scan_started = None;
                    let (m, r, elapsed, source) = *boxed;
                    self.last_scan_duration = Some(elapsed);
                    self.apply_result(m, r, source);
                }
                ScanMsg::Err(_, e) => {
                    self.scanning = false;
                    self.scan_started = None;
                    self.last_scan_duration = None;
                    self.error = Some(e.clone());
                    self.status = format!("Scan failed: {e}");
                }
                ScanMsg::HotkeyPressed => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    self.start_scan(ScanRequest::Hotkey);
                }
                // A background check stays in Settings; only a button press and
                // a finished install are worth the status line.
                ScanMsg::Update(state) => {
                    if self.update_manual {
                        self.error = None;
                        match &state {
                            UpdateState::Idle => {}
                            UpdateState::Checking => {
                                self.status = "Checking GitHub for a newer release…".into();
                            }
                            UpdateState::UpToDate { .. } => {
                                self.status = format!("{} is the latest release", update::CURRENT);
                            }
                            UpdateState::Available(release) => {
                                self.status = format!("{} is available", release.version);
                            }
                            UpdateState::Downloading(release) => {
                                self.status = format!(
                                    "Downloading {} ({} MB)…",
                                    release.version,
                                    megabytes(release.size)
                                );
                            }
                            UpdateState::Ready { version } => {
                                self.status = format!(
                                    "{version} is installed. It runs on the next start, or use Restart to update."
                                );
                            }
                            UpdateState::Failed { message } => {
                                self.error = Some(message.clone());
                                self.status = format!("Could not update: {message}");
                            }
                        }
                    } else {
                        match &state {
                            // A periodic check that finds a release says where
                            // the button is instead of acting on its own.
                            UpdateState::Available(release) => {
                                self.status = format!(
                                    "{} is available. Update from the command bar.",
                                    release.version
                                );
                            }
                            UpdateState::Ready { version } => {
                                self.status = format!(
                                    "{version} is installed. It runs on the next start, or use Restart to update."
                                );
                            }
                            UpdateState::Failed { message } => {
                                eprintln!("update check skipped: {message}");
                            }
                            _ => {}
                        }
                    }
                    self.update_state = state;
                }
            }
        }

        // Escape inside a focused text field only drops that focus. egui clears
        // focus before update() runs, so last frame's focus is the one to check.
        let escape =
            ctx.input(|input| input.key_pressed(egui::Key::Escape)) && !self.text_field_focused;
        let mut close_settings = false;
        // Hotkey recording owns the keyboard for the frame: the Escape that
        // cancels a capture must not also close the pane behind it, so the
        // guard below reads the recording flag as it was before the capture.
        let was_recording = self.hotkey_recording;
        let mut recorded = None;
        if was_recording {
            let pressed = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                })
            });
            if let Some((key, modifiers)) = pressed {
                recorded = Some(if key == egui::Key::Escape {
                    SettingsAction::CancelRecord
                } else {
                    // egui reports no Win/Super modifier, so `Win` cannot be recorded.
                    let hotkey = hotkey_spec(modifiers, key);
                    let error = parse_hotkey(&hotkey).err().map(|e| format!("{e}"));
                    SettingsAction::Recorded { hotkey, error }
                });
            }
        }
        if escape && !was_recording {
            if self.show_settings {
                close_settings = true;
            } else if self.cfg.hide_on_escape {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        let p = self.palette;
        let mut install_update = false;
        let mut restart_for_update = false;
        egui::TopBottomPanel::top("command_bar")
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .inner_margin(Margin::symmetric(12, 8))
                    .stroke(Stroke::new(1.0_f32, p.border)),
            )
            .show(ctx, |ui| {
                // At most one update button, immediately left of Settings.
                // Computed before the horizontal layout so the yield rule
                // below can see whether one will be shown.
                let update_button = match &self.update_state {
                    UpdateState::Ready { version } => Some((
                        "Restart to update".to_string(),
                        format!(
                            "{version} is installed and runs on the next start. Restarting now takes about a second."
                        ),
                        !self.scan_busy(),
                        false,
                    )),
                    UpdateState::Available(release) => Some((
                        format!("Update to {}", release.version),
                        format!(
                            "Downloads {} MB from GitHub, verifies it, and replaces poemercpricer.exe. You choose when to restart.",
                            megabytes(release.size)
                        ),
                        true,
                        true,
                    )),
                    _ => None,
                };
                let show_meta = bar_shows_meta(ui.available_width(), update_button.is_some());
                ui.horizontal(|ui| {
                    ui.label(poe_text("PoEMercPricer", 18.0, p.gold));
                    if show_meta {
                        ui.label(
                            RichText::new(format!("3.29 · v{}", update::CURRENT))
                                .color(p.subtle)
                                .small(),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let settings_text = if self.show_settings {
                            RichText::new("Settings").color(p.gold)
                        } else {
                            RichText::new("Settings")
                        };
                        let mut settings_button = egui::Button::new(settings_text)
                            .corner_radius(2)
                            .wrap_mode(egui::TextWrapMode::Extend);
                        if self.show_settings {
                            settings_button = settings_button.stroke(Stroke::new(1.0_f32, p.gold));
                        }
                        if ui
                            .add(settings_button)
                            .on_hover_text("App and scan settings")
                            .clicked()
                        {
                            if !self.show_settings {
                                self.error = None;
                                self.status =
                                    "Changes apply immediately and save when you go back."
                                        .into();
                            }
                            self.show_settings = !self.show_settings;
                        }
                        if let Some((label, hover, enabled, installs)) = update_button {
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(RichText::new(label).color(p.gold))
                                        .stroke(Stroke::new(1.0_f32, p.gold))
                                        .corner_radius(2)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .on_hover_text(hover)
                                .clicked()
                            {
                                install_update = installs;
                                restart_for_update = !installs;
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.scan_busy(),
                                egui::Button::new("Clipboard")
                                    .corner_radius(2)
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            )
                            .on_hover_text("Scan Warrant text or an image on the clipboard")
                            .clicked()
                        {
                            self.start_scan(ScanRequest::Clipboard);
                        }
                        let mut scan_label = egui::text::LayoutJob::default();
                        scan_label.append(
                            "Scan",
                            0.0,
                            TextFormat {
                                font_id: FontId::new(14.0, FontFamily::Proportional),
                                color: p.gold_fill_text,
                                ..Default::default()
                            },
                        );
                        if show_meta {
                            scan_label.append(
                                &format!(" {}", self.cfg.hotkey),
                                0.0,
                                TextFormat {
                                    font_id: FontId::new(11.0, FontFamily::Proportional),
                                    color: p.gold_fill_text,
                                    ..Default::default()
                                },
                            );
                        }
                        if ui
                            .add_enabled(
                                !self.scan_busy(),
                                egui::Button::new(scan_label)
                                    .fill(p.gold_fill)
                                    .corner_radius(2)
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            )
                            .on_hover_text(format!("Capture the game window ({})", self.cfg.hotkey))
                            .clicked()
                        {
                            self.start_scan(ScanRequest::Screen);
                        }
                    });
                });
            });

        if install_update {
            self.start_install();
        }
        if restart_for_update {
            self.restart(ctx);
        }

        egui::TopBottomPanel::bottom("result_actions")
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .inner_margin(Margin::symmetric(12, 8))
                    .stroke(Stroke::new(1.0_f32, p.border)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.show_settings {
                            let mut done_label = egui::text::LayoutJob::default();
                            done_label.append(
                                "Done",
                                0.0,
                                TextFormat {
                                    font_id: FontId::new(14.0, FontFamily::Proportional),
                                    color: p.gold_fill_text,
                                    ..Default::default()
                                },
                            );
                            if ui
                                .add(
                                    egui::Button::new(done_label)
                                        .fill(p.gold_fill)
                                        .corner_radius(2)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .clicked()
                            {
                                close_settings = true;
                            }
                        } else if self.result.is_some() {
                            let trade_open_ready = !self.scan_busy()
                                && self.last_trade_open.is_none_or(|opened| {
                                    opened.elapsed() >= TRADE_OPEN_COOLDOWN
                                });
                            if ui
                                .add_enabled(
                                    trade_open_ready,
                                    egui::Button::new("Search official trade")
                                        .corner_radius(2)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .on_hover_text(
                                    "Open instant-buyout listings for this warrant type and level with the money skill's supports matched exactly",
                                )
                                .clicked()
                            {
                                self.open_official_trade();
                            }
                            if ui
                                .add(
                                    egui::Button::new("Copy summary")
                                        .corner_radius(2)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .clicked()
                            {
                                match self.copy_summary() {
                                    Ok(()) => {
                                        self.error = None;
                                        self.status = "Summary copied".into();
                                    }
                                    Err(error) => {
                                        self.error = Some(error.to_string());
                                        self.status = format!("Could not copy summary: {error:#}");
                                    }
                                }
                            }
                        }

                        let width = ui.available_width();
                        let color = if self.error.is_some() {
                            p.red
                        } else if self.scan_busy() {
                            p.gold
                        } else {
                            p.muted
                        };
                        let message = status_message(
                            &self.status,
                            self.error.is_some(),
                            self.scan_busy(),
                            self.last_scan_duration,
                        );
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = width;
                        job.wrap.max_rows = 2;
                        job.wrap.break_anywhere = false;
                        job.wrap.overflow_character = Some('…');
                        job.append(
                            &message,
                            0.0,
                            TextFormat {
                                font_id: FontId::new(13.0, FontFamily::Proportional),
                                color,
                                ..Default::default()
                            },
                        );
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(job).wrap()).on_hover_text(message);
                        });
                    });
                });
            });

        let mut support_choice = None;
        let scan_error = scan_failed(self.error.as_deref(), &self.status);
        let recheck = recheck_in(
            self.last_update_check,
            &self.update_state,
            self.update_checks_enabled(),
        );
        let mut settings_action = None;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::same(12)))
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());
                if self.show_settings {
                    settings_action = settings_panel(
                        ui,
                        &mut self.cfg,
                        &self.update_state,
                        self.no_updates,
                        recheck,
                        self.hotkey_recording,
                        &p,
                    );
                } else {
                    if let Some(message) = &scan_error {
                        error_block(ui, message, &p);
                    }
                    if let Some((merc, result, market)) = &self.result {
                        score_card(
                            ui,
                            merc,
                            result,
                            market.as_ref(),
                            &mut self.icon_cache,
                            &mut support_choice,
                            &p,
                        );
                    } else {
                        idle_help(ui, &self.cfg.hotkey, &p);
                    }
                }
            });

        if self.show_settings {
            let action = settings_action.or(recorded);
            match action {
                Some(SettingsAction::Back) => close_settings = true,
                // A theme click writes through immediately; the pane stays open.
                Some(SettingsAction::Save) => {
                    if let Err(error) = self.cfg.save() {
                        self.error = Some(error.to_string());
                        self.status = format!("Settings not saved: {error:#}");
                    }
                }
                Some(SettingsAction::OpenFolder) => {
                    if let Err(error) = open::that_detached(AppConfig::dir()) {
                        self.error = Some(error.to_string());
                        self.status = format!("Could not open config folder: {error}");
                    }
                }
                Some(SettingsAction::OpenReleases) => {
                    if let Err(error) = open::that_detached(update::RELEASES_URL) {
                        self.error = Some(error.to_string());
                        self.status = format!("Could not open the releases page: {error}");
                    }
                }
                Some(SettingsAction::CheckNow) => self.start_update_check(true),
                Some(SettingsAction::Install) => self.start_install(),
                // Restart is enabled in Settings and gated here, the same way
                // the command-bar button is disabled while a scan runs.
                Some(SettingsAction::Restart) => {
                    if self.scan_busy() {
                        self.status = "Finish the current scan before restarting".into();
                    } else {
                        self.restart(ctx);
                    }
                }
                Some(SettingsAction::Record) => self.hotkey_recording = true,
                Some(SettingsAction::CancelRecord) => {
                    self.hotkey_recording = false;
                    self.status = "Hotkey recording cancelled".into();
                }
                // An unparseable capture is still stored, so the field shows in
                // red what was pressed instead of silently discarding it.
                Some(SettingsAction::Recorded { hotkey, error }) => {
                    self.hotkey_recording = false;
                    match error {
                        None => {
                            self.error = None;
                            self.status = format!("Hotkey set to {hotkey}. Applies after restart.");
                        }
                        Some(error) => {
                            self.status = format!("Settings not saved: {error}");
                            self.error = Some(error);
                        }
                    }
                    self.cfg.hotkey = hotkey;
                }
                None => {}
            }
            // Edits apply live, so closing also saves; an invalid hotkey keeps
            // the pane open so memory and file never silently diverge. A save
            // that fails (read-only AppData) still closes: the settings are
            // live and the pane cannot fix the disk.
            if close_settings {
                match parse_hotkey(&self.cfg.hotkey) {
                    Ok(_) => {
                        match self.cfg.save() {
                            Ok(()) => {
                                self.error = None;
                                self.status = "Settings saved".into();
                            }
                            Err(error) => {
                                self.error = Some(error.to_string());
                                self.status = format!("Settings not saved: {error:#}");
                            }
                        }
                        self.show_settings = false;
                        self.hotkey_recording = false;
                    }
                    Err(error) => {
                        self.error = Some(error.to_string());
                        self.status = format!("Settings not saved: {error:#}");
                    }
                }
            }
        }

        if let Some(choice) = support_choice {
            let assume_projectile_speed = self.cfg.assume_projectile_speed;
            if let Some((merc, result, market)) = &mut self.result {
                if apply_support_choice(merc, &choice) {
                    *result = score_mercenary(merc, assume_projectile_speed);
                    *market = estimate_for(merc, !result.bricks.is_empty());
                    let skill_name = &merc.skills[choice.skill_index].canonical;
                    let support = &merc.skills[choice.skill_index].supports[choice.support_index];
                    self.error = None;
                    self.status = format!(
                        "{} selected for {}. Score updated.",
                        support_display_for_tier(&support.canonical, support.tier as u8),
                        skill_name
                    );
                }
            }
        }

        self.text_field_focused = ctx
            .memory(|memory| memory.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some());
    }
}

fn take_always_on_top_change(applied: &mut bool, desired: bool) -> Option<bool> {
    if *applied == desired {
        None
    } else {
        *applied = desired;
        Some(desired)
    }
}

fn idle_help(ui: &mut Ui, hotkey: &str, p: &Palette) {
    Frame::new()
        .fill(p.surface)
        .corner_radius(2)
        .inner_margin(Margin::same(16))
        .stroke(Stroke::new(1.0_f32, p.border))
        .show(ui, |ui| {
            ui.label(
                RichText::new("No mercenary scanned yet")
                    .color(p.text)
                    .size(18.0),
            );
            ui.add_space(14.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.spacing_mut().interact_size.y = 0.0;
            instruction_row(
                ui,
                "1",
                RichText::new("Open the mercenary inspect panel or hover a Warrant.").color(p.text),
                p,
            );
            let mut hotkey_line = egui::text::LayoutJob::default();
            hotkey_line.wrap.max_width = ui.available_width();
            hotkey_line.append(
                "Press ",
                0.0,
                TextFormat {
                    font_id: FontId::new(14.0, FontFamily::Proportional),
                    color: p.text,
                    ..Default::default()
                },
            );
            hotkey_line.append(
                hotkey,
                0.0,
                TextFormat {
                    font_id: FontId::new(14.0, POE_SERIF.clone()),
                    color: p.gold,
                    ..Default::default()
                },
            );
            hotkey_line.append(
                ", or use Scan above.",
                0.0,
                TextFormat {
                    font_id: FontId::new(14.0, FontFamily::Proportional),
                    color: p.text,
                    ..Default::default()
                },
            );
            instruction_row(ui, "2", hotkey_line, p);
            instruction_row(
                ui,
                "3",
                RichText::new("Use Clipboard for copied Warrant text, images, or image files.")
                    .color(p.text),
                p,
            );
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("The scan runs locally. No screenshots are uploaded.")
                    .color(p.muted)
                    .size(14.0),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Scores prioritize resale screening; verify exceptional rolls on trade.",
                )
                .color(p.subtle)
                .size(11.5),
            );
        });
}

fn instruction_row(ui: &mut Ui, step: &str, text: impl Into<egui::WidgetText>, p: &Palette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().interact_size.y = 0.0;
        ui.spacing_mut().item_spacing.x = 10.0;
        let h = ui.text_style_height(&egui::TextStyle::Body);
        ui.allocate_ui_with_layout(
            Vec2::new(16.0, h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(poe_text(format!("{step}."), 14.0, p.gold));
            },
        );
        ui.add(egui::Label::new(text.into()).wrap());
    });
}

/// After a failed scan, the error text (with a trailing period) to show in
/// the error block, or `None` if there is no error, or the current error
/// belongs to some other action (e.g. "Settings not saved") rather than a
/// scan.
fn scan_failed(error: Option<&str>, status: &str) -> Option<String> {
    let error = error?;
    if !status.starts_with("Scan failed") {
        return None;
    }
    if error.ends_with('.') {
        Some(error.to_string())
    } else {
        Some(format!("{error}."))
    }
}

fn error_block(ui: &mut Ui, message: &str, p: &Palette) {
    Frame::new()
        .fill(p.red.gamma_multiply(p.fill_error))
        .stroke(Stroke::new(1.0_f32, p.red))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().interact_size.y = 0.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(poe_text("Scan failed", 15.0, p.red));
            ui.add(egui::Label::new(RichText::new(message).color(p.text).size(13.0)).wrap());
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Hover the Warrant and press Ctrl+C, copy a screenshot, or use Scan while Path of Exile runs in Windowed Fullscreen.",
                    )
                    .color(p.muted)
                    .size(12.0),
                )
                .wrap(),
            );
        });
    ui.add_space(10.0);
}

fn status_message(
    status: &str,
    is_error: bool,
    scanning: bool,
    elapsed: Option<Duration>,
) -> String {
    if !is_error && !scanning && status.starts_with("Scan complete") {
        if let Some(duration) = elapsed {
            return format!("{status} in {} ms", duration.as_millis());
        }
    }
    status.to_owned()
}

/// Score bar tick marks, as fractions of the bar's width.
const SCORE_TICKS: [f32; 4] = [0.50, 0.65, 0.80, 0.90];

fn score_card(
    ui: &mut Ui,
    merc: &Mercenary,
    result: &ScoreResult,
    market: Option<&MarketEstimate>,
    icons: &mut IconCache,
    support_choice: &mut Option<SupportChoice>,
    p: &Palette,
) {
    let accent = band_color(&result.band, result.jackpot, p);
    let take = is_take_item(&result.band, result.jackpot);
    let fill_alpha = if take {
        p.fill_take
    } else {
        match result.band.as_str() {
            "skip" => p.fill_skip,
            "check" => p.fill_check,
            _ => p.fill_none,
        }
    };
    let range_color = if take && result.bricks.is_empty() {
        accent
    } else {
        p.text
    };
    Frame::new()
        .fill(p.raised)
        .corner_radius(2)
        .inner_margin(Margin::same(3))
        .stroke(Stroke::new(1.0_f32, p.border))
        .show(ui, |ui| {
            Frame::new()
                .fill(p.bg)
                .corner_radius(1)
                .inner_margin(Margin::symmetric(14, 10))
                .stroke(Stroke::new(1.0_f32, p.border_dark))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.spacing_mut().interact_size.y = 0.0;
                    // Identity row: class, level, and the mercenary name
                    // right-aligned on the same line.
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        ui.label(poe_text(&merc.class_name, 16.0, p.text));
                        if let Some(level) = merc.level {
                            ui.label(poe_text(format!("Lvl {level}"), 14.0, p.muted));
                        }
                        if !merc.name.is_empty() {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(poe_text(&merc.name, 14.0, p.muted));
                                },
                            );
                        }
                    });
                    Frame::new()
                        .fill(accent.gamma_multiply(fill_alpha))
                        .inner_margin(Margin::symmetric(10, 8))
                        .stroke(Stroke::new(if take { 2.0_f32 } else { 1.0 }, accent))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 7.0;
                            ui.spacing_mut().interact_size.y = 0.0;
                            ui.horizontal_top(|ui| {
                                let left_w = ui.available_width() - 64.0 - 12.0;
                                ui.allocate_ui_with_layout(
                                    Vec2::new(left_w, 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.set_width(left_w);
                                        ui.spacing_mut().item_spacing.y = 4.0;
                                        ui.add(
                                            egui::Label::new(
                                                poe_text(verdict_title(result), 20.0, accent)
                                                    .line_height(Some(23.0)),
                                            )
                                            .wrap(),
                                        );
                                        if let Some(detail) = verdict_detail(result) {
                                            ui.label(poe_text(detail, 14.0, p.text));
                                        }
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&result.action)
                                                    .color(p.orange)
                                                    .size(13.0),
                                            )
                                            .wrap(),
                                        );
                                    },
                                );
                                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                                    ui.set_min_width(64.0);
                                    ui.label(poe_text(score_number(result), 30.0, p.text));
                                    ui.label(
                                        RichText::new(score_caption(result))
                                            .color(p.muted)
                                            .size(11.0),
                                    );
                                });
                            });
                            if result.band != "unsupported" {
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 6.0),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter();
                                painter.rect_filled(rect, 0.0, p.border_dark);
                                let pct = (result.score / 100.0).clamp(0.0, 1.0);
                                let fill_rect = egui::Rect::from_min_size(
                                    rect.min,
                                    Vec2::new(rect.width() * pct, rect.height()),
                                );
                                painter.rect_filled(fill_rect, 0.0, accent);
                                for tick in SCORE_TICKS {
                                    let x = rect.min.x + rect.width() * tick;
                                    painter.line_segment(
                                        [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                                        Stroke::new(1.0_f32, p.bg),
                                    );
                                }
                            }
                            for highlight in result.highlights.iter().take(3) {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("•").color(p.gold).size(13.0));
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(highlight).color(p.muted).size(13.0),
                                        )
                                        .wrap(),
                                    );
                                });
                            }
                            market_rows(ui, market, range_color, p);
                        });
                    if result.estimate {
                        ui.label(
                            RichText::new(
                                "Catalog coverage is complete; market evidence is limited.",
                            )
                            .color(p.muted)
                            .size(12.0),
                        );
                    }
                });
        });

    ui.add_space(10.0);
    // Overflow is only known after the scroll area lays out, so the cue uses
    // last frame's answer and asks for one more frame when it changes.
    let overflow_id = ui.id().with("skills_overflow");
    let overflows = ui
        .ctx()
        .data(|data| data.get_temp(overflow_id))
        .unwrap_or(false);
    skills_heading(ui, merc.skills.len(), overflows, p);
    ui.add_space(6.0);
    let skills_height = ui.available_height().max(140.0);
    ui.scope(|ui| {
        // A solid, allocated track remains visible at rest. This is deliberately
        // scoped to the skills pane so other compact controls keep their sizing.
        {
            let scroll = &mut ui.style_mut().spacing.scroll;
            scroll.floating = false;
            scroll.bar_width = 10.0;
            scroll.handle_min_length = 36.0;
            scroll.bar_inner_margin = 6.0;
            scroll.bar_outer_margin = 2.0;
            scroll.foreground_color = true;
        }
        ui.visuals_mut().widgets.inactive.fg_stroke =
            Stroke::new(1.0_f32, p.gold.gamma_multiply(0.72));
        ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.gold);
        ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(1.0_f32, p.orange);

        let output = egui::ScrollArea::vertical()
            .id_salt("skills_scroll")
            .max_height(skills_height)
            .auto_shrink([false, false])
            .animated(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                Frame::new()
                    .fill(p.raised)
                    .corner_radius(2)
                    .inner_margin(Margin::same(3))
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .show(ui, |ui| {
                        Frame::new()
                            .fill(p.bg)
                            .inner_margin(Margin::same(0))
                            .stroke(Stroke::new(1.0_f32, p.border_dark))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.spacing_mut().item_spacing.y = 0.0;
                                if merc.skills.is_empty() {
                                    Frame::new().inner_margin(Margin::symmetric(8, 6)).show(
                                        ui,
                                        |ui| {
                                            ui.label(
                                                RichText::new("No skills recognized in this scan.")
                                                    .color(p.muted),
                                            );
                                        },
                                    );
                                }
                                for (index, skill) in merc.skills.iter().enumerate() {
                                    let is_brick = result
                                        .bricks
                                        .iter()
                                        .any(|brick| brick.eq_ignore_ascii_case(&skill.canonical));
                                    skill_row(ui, skill, icons, index, is_brick, support_choice, p);
                                    if index + 1 < merc.skills.len() {
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), 1.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().rect_filled(rect, 0, p.border_dark);
                                    }
                                }
                            });
                    });
                ui.add_space(8.0);

                if !result.breakdown.is_empty() && result.band != "unsupported" {
                    disclosure_section(ui, "score_breakdown", "Score breakdown", false, p, |ui| {
                        ui.add_space(3.0);
                        Frame::new()
                            .fill(p.surface)
                            .inner_margin(Margin::symmetric(10, 6))
                            .stroke(Stroke::new(1.0_f32, p.border_dark))
                            .show(ui, |ui| {
                                egui::Grid::new("score_breakdown_grid")
                                    .num_columns(2)
                                    .striped(false)
                                    .spacing(Vec2::new(12.0, 3.0))
                                    .min_col_width(40.0)
                                    .show(ui, |ui| {
                                        for item in &result.breakdown {
                                            if item.points.abs() < 0.05 {
                                                continue;
                                            }
                                            let color =
                                                if item.points > 0.0 { p.green } else { p.red };
                                            ui.allocate_ui_with_layout(
                                                Vec2::new(40.0, 0.0),
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:+.0}",
                                                            item.points
                                                        ))
                                                        .color(color)
                                                        .size(13.0),
                                                    );
                                                },
                                            );
                                            ui.label(
                                                RichText::new(&item.label)
                                                    .color(p.muted)
                                                    .size(13.0),
                                            )
                                            .on_hover_text(&item.detail);
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                }
            });
        let now = scroll_cue_visible(output.content_size.y, output.inner_rect.height());
        if now != overflows {
            ui.ctx().data_mut(|data| data.insert_temp(overflow_id, now));
            ui.ctx().request_repaint();
        }
    });
}

const MARKET_HOVER: &str = "Cheapest and typical instant-buyout asks for the closest matching package on the official trade site at the snapshot date. Not a live price.";

/// Market ask range inside the verdict box. The range colour only repeats the
/// verdict accent; the confidence word and detail text carry the caveats.
fn market_rows(ui: &mut Ui, market: Option<&MarketEstimate>, range_color: Color32, p: &Palette) {
    ui.spacing_mut().item_spacing.y = 2.0;
    let (rule, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().hline(
        rule.x_range(),
        rule.center().y,
        Stroke::new(1.0_f32, p.border_dark),
    );
    ui.add_space(6.0);
    let Some(market) = market else {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new("Market").color(p.muted).size(12.0));
            ui.label(poe_text("no listings data for this build", 15.0, p.muted));
        })
        .response
        .on_hover_text(MARKET_HOVER);
        ui.add(egui::Label::new(RichText::new(MARKET_HOVER).color(p.muted).size(12.0)).wrap())
            .on_hover_text(MARKET_HOVER);
        return;
    };
    let confidence = market.confidence();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(RichText::new("Market").color(p.muted).size(12.0));
        ui.label(poe_text(market.range_label(), 15.0, range_color));
        if !confidence.is_empty() {
            ui.label(RichText::new(confidence).color(p.orange).size(12.0));
        }
    })
    .response
    .on_hover_text(MARKET_HOVER);
    // 12 px like the scroll cue: the caveats have to stay readable at rest.
    ui.add(
        egui::Label::new(
            RichText::new(market.detail_line())
                .color(p.muted)
                .size(12.0),
        )
        .wrap(),
    )
    .on_hover_text(MARKET_HOVER);
}

fn disclosure_section(
    ui: &mut Ui,
    id_source: impl std::hash::Hash,
    title: &str,
    default_open: bool,
    p: &Palette,
    body: impl FnOnce(&mut Ui),
) {
    let id = ui.make_persistent_id(id_source);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        default_open,
    );
    let expanded = state.is_open();
    let header = ui.horizontal(|ui| {
        disclosure_chevron(ui, expanded, true, 22.0, p);
        ui.label(RichText::new(title).color(p.muted));
    });
    let response = ui
        .interact(
            header.response.rect,
            ui.id().with(("section_disclosure", title)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    focus_ring(ui, &response, p);
    if response.clicked() {
        state.toggle(ui);
        state.store(ui.ctx());
        ui.ctx().request_repaint();
    }
    if state.is_open() {
        body(ui);
    }
}

fn skill_row(
    ui: &mut Ui,
    skill: &poemercpricer::Skill,
    icons: &mut IconCache,
    index: usize,
    is_brick: bool,
    support_choice: &mut Option<SupportChoice>,
    p: &Palette,
) {
    let row_fill = if is_brick { p.brick_row } else { p.surface };
    Frame::new()
        .fill(row_fill)
        .inner_margin(Margin::symmetric(8, 5))
        .stroke(if is_brick {
            Stroke::new(1.0_f32, p.red)
        } else {
            Stroke::NONE
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let expandable = !skill.supports.is_empty();
            let id = ui.make_persistent_id(("merc_skill", index, &skill.canonical));
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                id,
                is_brick,
            );
            let clicked = skill_header(
                ui,
                skill,
                icons,
                index,
                is_brick,
                expandable,
                state.is_open(),
                p,
            );
            if expandable && clicked {
                state.toggle(ui);
                state.store(ui.ctx());
                ui.ctx().request_repaint();
            }
            if expandable && state.is_open() {
                ui.add_space(5.0);
                support_grid(ui, skill, icons, index, support_choice, p);
            }
        });
}

// The 8th argument is `p: &Palette`, threaded through every drawing function
// per the theming design.
#[allow(clippy::too_many_arguments)]
fn skill_header(
    ui: &mut Ui,
    skill: &poemercpricer::Skill,
    icons: &mut IconCache,
    index: usize,
    is_brick: bool,
    expandable: bool,
    expanded: bool,
    p: &Palette,
) -> bool {
    let row = ui.horizontal(|ui| {
        let row_width = ui.available_width();
        ui.set_min_height(42.0);
        ui.spacing_mut().item_spacing.x = 8.0;
        disclosure_chevron(ui, expanded, expandable, 42.0, p);
        ui.allocate_ui_with_layout(
            Vec2::splat(34.0),
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                if let Some(texture) = icons.skill(ui.ctx(), &skill.canonical) {
                    Frame::new()
                        .fill(p.bg)
                        .inner_margin(Margin::same(1))
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .show(ui, |ui| {
                            ui.image((texture.id(), Vec2::splat(30.0)));
                        });
                }
            },
        );
        if is_brick {
            ui.vertical(|ui| {
                ui.spacing_mut().interact_size.y = 0.0;
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(poe_text(&skill.canonical, 15.0, p.text));
                    if let Some(level) = skill.level {
                        ui.label(poe_text(format!("Lvl {level}"), 12.0, p.muted));
                    }
                });
                ui.label(poe_text("BRICK: competing attack", 12.0, p.red));
            });
        } else {
            ui.label(poe_text(&skill.canonical, 15.0, p.text));
            if let Some(level) = skill.level {
                ui.label(poe_text(format!("Lvl {level}"), 12.0, p.muted));
            }
        }
        let show_icons = shows_icon_strip(row_width);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            support_summary_ui(ui, skill, icons, show_icons, p);
        });
    });
    if expandable {
        let response = ui
            .interact(
                row.response.rect,
                ui.id()
                    .with(("skill_header_click", index, &skill.canonical)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        focus_ring(ui, &response, p);
        response.clicked()
    } else {
        false
    }
}

fn disclosure_chevron(ui: &mut Ui, expanded: bool, visible: bool, slot_height: f32, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, slot_height), egui::Sense::hover());
    if !visible {
        return;
    }
    let center = rect.center();
    let points = if expanded {
        vec![
            egui::pos2(center.x - 4.0, center.y - 2.0),
            egui::pos2(center.x, center.y + 2.0),
            egui::pos2(center.x + 4.0, center.y - 2.0),
        ]
    } else {
        vec![
            egui::pos2(center.x - 2.0, center.y - 4.0),
            egui::pos2(center.x + 2.0, center.y),
            egui::pos2(center.x - 2.0, center.y + 4.0),
        ]
    };
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.5_f32, p.gold)));
}

/// True when the row's icon strip (support gem icons, left of the stacked
/// count/tier labels) has room to show: purely a function of row width.
fn shows_icon_strip(width: f32) -> bool {
    width >= 500.0
}

fn support_summary_ui(
    ui: &mut Ui,
    skill: &poemercpricer::Skill,
    icons: &mut IconCache,
    show_icons: bool,
    p: &Palette,
) {
    if skill.supports.is_empty() {
        ui.label(RichText::new("No supports").color(p.muted).size(12.0));
        return;
    }

    let tiers = support_tier_summary(skill);
    ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
        ui.spacing_mut().interact_size.y = 0.0;
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.label(
            RichText::new(format!("{} supports", skill.supports.len()))
                .color(p.text)
                .size(12.5),
        );
        ui.label(RichText::new(tiers).color(p.gold).size(12.0));
    });
    if show_icons {
        ui.horizontal(|ui| {
            ui.spacing_mut().interact_size.y = 0.0;
            ui.spacing_mut().item_spacing.x = 2.0;
            for gem in skill.supports.iter().rev() {
                if let Some(texture) = icons.support(ui.ctx(), &gem.canonical) {
                    ui.image((texture.id(), Vec2::splat(18.0)));
                }
            }
        });
    }
}

fn support_tier_summary(skill: &poemercpricer::Skill) -> String {
    let mut tiers = [0_usize; 4];
    for support in &skill.supports {
        tiers[support.tier as usize] += 1;
    }
    let mut parts = Vec::new();
    for tier in (1..=3).rev() {
        if tiers[tier] > 0 {
            parts.push(format!("{}×T{tier}", tiers[tier]));
        }
    }
    parts.join(" · ")
}

/// Support-grid column count for the given available width: two columns
/// once there is room, one otherwise.
fn support_columns(width: f32) -> usize {
    if width >= 560.0 {
        2
    } else {
        1
    }
}

fn support_grid(
    ui: &mut Ui,
    skill: &poemercpricer::Skill,
    icons: &mut IconCache,
    skill_index: usize,
    support_choice: &mut Option<SupportChoice>,
    p: &Palette,
) {
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.vertical(|ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.x = 4.0;
            let columns_n = support_columns(ui.ctx().screen_rect().width());
            let mut index = 0;
            while index < skill.supports.len() {
                let gem = &skill.supports[index];
                if gem.canonical == "ambiguous" {
                    support_detail(ui, gem, icons, skill_index, index, support_choice, p);
                    ui.add_space(4.0);
                    index += 1;
                    continue;
                }
                let mut batch = 1;
                while batch < columns_n
                    && index + batch < skill.supports.len()
                    && skill.supports[index + batch].canonical != "ambiguous"
                {
                    batch += 1;
                }
                ui.columns(batch, |columns| {
                    for (offset, col) in columns.iter_mut().enumerate() {
                        let idx = index + offset;
                        support_detail(
                            col,
                            &skill.supports[idx],
                            icons,
                            skill_index,
                            idx,
                            support_choice,
                            p,
                        );
                        col.add_space(4.0);
                    }
                });
                index += batch;
            }
        });
    });
}

fn support_detail(
    ui: &mut Ui,
    gem: &poemercpricer::SupportGem,
    icons: &mut IconCache,
    skill_index: usize,
    support_index: usize,
    support_choice: &mut Option<SupportChoice>,
    p: &Palette,
) {
    let name = if gem.canonical == "ambiguous" {
        "Select exact support".to_owned()
    } else if gem.canonical == "unknown" || gem.canonical.is_empty() {
        gem.name.clone()
    } else {
        support_display_for_tier(&gem.canonical, gem.tier as u8)
    };
    support_detail_named(
        ui,
        &name,
        gem,
        icons,
        skill_index,
        support_index,
        support_choice,
        p,
    );
}

// See the note on `skill_header`: the 8th argument is the threaded palette.
#[allow(clippy::too_many_arguments)]
fn support_detail_named(
    ui: &mut Ui,
    name: &str,
    gem: &poemercpricer::SupportGem,
    icons: &mut IconCache,
    skill_index: usize,
    support_index: usize,
    support_choice: &mut Option<SupportChoice>,
    p: &Palette,
) {
    let ambiguous = gem.canonical == "ambiguous";
    Frame::new()
        .fill(p.raised)
        .corner_radius(1)
        .inner_margin(Margin::symmetric(6, 3))
        .stroke(Stroke::new(
            1.0_f32,
            if ambiguous { p.orange } else { p.border },
        ))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().interact_size.y = 0.0;
            ui.horizontal(|ui| {
                if let Some(texture) = icons.support(ui.ctx(), &gem.canonical) {
                    ui.image((texture.id(), Vec2::splat(22.0)));
                }
                ui.label(
                    RichText::new(name)
                        .color(if ambiguous { p.orange } else { p.muted })
                        .size(12.0),
                )
                .on_hover_text(format!("OCR confidence: {:.0}%", gem.confidence * 100.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(poe_text(format!("T{}", gem.tier as u8), 12.0, p.gold));
                });
            });
            if ambiguous {
                ui.add_space(3.0);
                ui.label(
                    RichText::new("Shared icon: select the support shown in game.")
                        .color(p.muted)
                        .size(11.5),
                );
                ui.horizontal_wrapped(|ui| {
                    for (canonical, display) in ambiguous_support_choices(gem) {
                        if ui
                            .add(
                                egui::Button::new(RichText::new(display).color(p.text).size(13.0))
                                    .fill(p.surface)
                                    .stroke(Stroke::new(1.0_f32, p.gold))
                                    .corner_radius(2)
                                    .min_size(Vec2::new(0.0, 26.0))
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            )
                            .clicked()
                        {
                            *support_choice = Some(SupportChoice {
                                skill_index,
                                support_index,
                                canonical,
                            });
                        }
                    }
                });
                ui.label(
                    RichText::new(
                        "Tip: hover the gem in game and scan again to identify it automatically.",
                    )
                    .color(p.muted)
                    .size(11.5),
                );
            }
        });
}

fn ambiguous_support_choices(gem: &poemercpricer::SupportGem) -> Vec<(String, String)> {
    if gem.canonical != "ambiguous" {
        return Vec::new();
    }

    let mut choices = Vec::new();
    for candidate in gem.name.split(" / ").map(str::trim) {
        let (canonical, confidence) = canonical_support(candidate);
        if confidence < 0.99
            || canonical.is_empty()
            || choices.iter().any(|(existing, _)| existing == &canonical)
        {
            continue;
        }
        let display = support_display_for_tier(&canonical, gem.tier as u8);
        choices.push((canonical, display));
    }
    choices
}

fn apply_support_choice(merc: &mut Mercenary, choice: &SupportChoice) -> bool {
    let Some(gem) = merc
        .skills
        .get_mut(choice.skill_index)
        .and_then(|skill| skill.supports.get_mut(choice.support_index))
    else {
        return false;
    };
    if !ambiguous_support_choices(gem)
        .iter()
        .any(|(canonical, _)| canonical == &choice.canonical)
    {
        return false;
    }

    gem.canonical.clone_from(&choice.canonical);
    gem.name = support_display_for_tier(&choice.canonical, gem.tier as u8);
    gem.confidence = 1.0;
    gem.raw.push_str("; user selected exact support");
    true
}

/// Rect-sensed rows draw no widget chrome, so keyboard focus is shown here.
fn focus_ring(ui: &Ui, response: &egui::Response, p: &Palette) {
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            2,
            Stroke::new(1.0_f32, p.gold),
            egui::StrokeKind::Inside,
        );
    }
}

fn poe_text(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::new(size, POE_SERIF.clone()))
        .color(color)
}

/// Whether the command bar has room for the version tag and the Scan
/// button's hotkey hint: both drop below 520 px, and whenever an update
/// button is present (it already competes for the same right-aligned row).
fn bar_shows_meta(width: f32, update_button_visible: bool) -> bool {
    width >= 520.0 && !update_button_visible
}

fn skills_heading(ui: &mut Ui, count: usize, overflows: bool, p: &Palette) {
    ui.horizontal(|ui| {
        ui.label(poe_text("Mercenary skills", 16.0, p.gold));
        ui.label(poe_text(format!("({count})"), 13.0, p.muted));
        if overflows {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                scroll_cue(ui, p);
            });
        }
    });
}

fn scroll_cue_visible(content_height: f32, viewport_height: f32) -> bool {
    content_height > viewport_height + 0.5
}

fn scroll_cue(ui: &mut Ui, p: &Palette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 16.0), egui::Sense::hover());
        let center = rect.center();
        let stroke = Stroke::new(1.25_f32, p.gold);
        ui.painter().line_segment(
            [
                egui::pos2(center.x, center.y - 4.0),
                egui::pos2(center.x, center.y + 4.0),
            ],
            stroke,
        );
        ui.painter().line_segment(
            [
                egui::pos2(center.x - 2.5, center.y - 1.5),
                egui::pos2(center.x, center.y - 4.0),
            ],
            stroke,
        );
        ui.painter().line_segment(
            [
                egui::pos2(center.x + 2.5, center.y + 1.5),
                egui::pos2(center.x, center.y + 4.0),
            ],
            stroke,
        );
        ui.label(
            RichText::new("Scroll for all skills")
                .color(p.muted)
                .size(12.0),
        );
    });
}

fn score_number(result: &ScoreResult) -> String {
    if result.band == "unsupported" {
        "—".into()
    } else {
        format!("{:.0}", result.score)
    }
}

fn score_caption(result: &ScoreResult) -> &'static str {
    if result.band == "unsupported" {
        "Not scored"
    } else {
        "of 100"
    }
}

fn verdict_title(result: &ScoreResult) -> &'static str {
    if !result.bricks.is_empty() {
        "SKIP: BRICKED"
    } else if result.jackpot || matches!(result.band.as_str(), "jackpot" | "jackpot-band") {
        "TAKE ITEM: JACKPOT CANDIDATE"
    } else {
        match result.band.as_str() {
            "skip" => "SKIP THIS WARRANT",
            "common" => "LOW VALUE",
            "check" => "TAKE ITEM: PRICE CHECK",
            "good" => "TAKE ITEM: GOOD",
            "very-valuable" => "TAKE ITEM: VERY VALUABLE",
            "unsupported" => "NO RELIABLE PRICE VERDICT",
            _ => "REVIEW THIS WARRANT",
        }
    }
}

fn verdict_detail(result: &ScoreResult) -> Option<String> {
    if result.bricks.is_empty() {
        None
    } else {
        Some(format!("BRICKED BY: {}", result.bricks.join(", ")))
    }
}

/// The two Settings lines for one update state. Pure, so the copy table in
/// `docs/updater.md` section 5.1 is unit-tested instead of eyeballed.
fn update_copy(
    state: &UpdateState,
    checks_enabled: bool,
    no_updates: bool,
    recheck: Option<Duration>,
) -> (String, String) {
    match state {
        UpdateState::Idle if checks_enabled => (
            "Not checked yet.".into(),
            "Checks when the app starts and every 6 hours.".into(),
        ),
        UpdateState::Idle => (
            "Update checks are off.".into(),
            if no_updates {
                "Off with --no-updates.".into()
            } else {
                "Off in Settings.".into()
            },
        ),
        UpdateState::Checking => ("Checking GitHub for a newer release…".into(), String::new()),
        UpdateState::UpToDate { checked } => (
            format!("{} is the latest release.", update::CURRENT),
            match recheck {
                Some(wait) => format!(
                    "{} Checks again in about {} h.",
                    checked_ago(checked.elapsed()),
                    wait.as_secs().div_ceil(3600)
                ),
                None => checked_ago(checked.elapsed()),
            },
        ),
        UpdateState::Available(release) => (
            format!("{} is available.", release.version),
            format!(
                "Install updates automatically is off. Update to {} downloads it now; it runs after a restart.",
                release.version
            ),
        ),
        UpdateState::Downloading(release) => (
            format!(
                "Downloading {} ({} MB)…",
                release.version,
                megabytes(release.size)
            ),
            "The current version keeps working until you restart.".into(),
        ),
        UpdateState::Ready { version } => (
            format!("{version} is installed."),
            format!(
                "It runs the next time you start PoEMercPricer, or restart now. Until then {} keeps running.",
                update::CURRENT
            ),
        ),
        UpdateState::Failed { message } => (
            format!("Could not update: {message}."),
            "Nothing was changed. You can download it yourself.".into(),
        ),
    }
}

/// How long until the next automatic check, or None when there must not be one.
/// `Ready` is excluded on purpose: an installed update waits for the restart
/// instead of being downloaded over.
fn recheck_in(last: Option<Instant>, state: &UpdateState, enabled: bool) -> Option<Duration> {
    if !enabled
        || matches!(
            state,
            UpdateState::Checking | UpdateState::Downloading(_) | UpdateState::Ready { .. }
        )
    {
        return None;
    }
    match last {
        None => Some(Duration::ZERO),
        Some(last) => Some(UPDATE_RECHECK.saturating_sub(last.elapsed())),
    }
}

/// A scan still running after [`SCAN_WATCHDOG`] is treated as wedged. Its
/// worker is left to finish or block forever; the UI stops waiting for it.
fn scan_is_stuck(scanning: bool, started: Option<Instant>) -> bool {
    scanning && started.is_some_and(|started| started.elapsed() >= SCAN_WATCHDOG)
}

/// How long until a running scan hits the watchdog, so the UI can schedule one
/// wake-up at that moment and re-enable the buttons. `None` once it is stuck or
/// when nothing is running: neither needs a further repaint.
fn scan_watchdog_wake(scanning: bool, started: Option<Instant>) -> Option<Duration> {
    let started = started.filter(|_| scanning)?;
    let remaining = SCAN_WATCHDOG.checked_sub(started.elapsed())?;
    (!remaining.is_zero()).then_some(remaining)
}

/// Computed at paint time from an elapsed timer, so it never schedules a repaint.
fn checked_ago(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    if seconds < 60 {
        "Checked just now.".into()
    } else if seconds < 3600 {
        format!("Checked {} min ago.", seconds / 60)
    } else {
        format!("Checked {} h ago.", seconds / 3600)
    }
}

/// Download size the way GitHub reports it: decimal megabytes, whole numbers.
fn megabytes(size: u64) -> u64 {
    (size as f64 / 1_000_000.0).round() as u64
}

fn installed_state(release: &update::Release, exe: &Path) -> UpdateState {
    match update::install(release, exe) {
        Ok(()) => UpdateState::Ready {
            version: release.version.to_string(),
        },
        Err(e) => UpdateState::Failed {
            message: format!("{e:#}"),
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SettingsAction {
    Back,
    Save,
    OpenFolder,
    CheckNow,
    Install,
    Restart,
    OpenReleases,
    Record,
    CancelRecord,
    Recorded {
        hotkey: String,
        error: Option<String>,
    },
}

/// The scan-hotkey spec for one captured key press, in the token order
/// `parse_hotkey` understands. egui never reports the Windows key, so a
/// recorded spec can only ever carry Ctrl, Shift and Alt.
fn hotkey_spec(modifiers: egui::Modifiers, key: egui::Key) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    parts.push(key.name());
    parts.join("+")
}

/// What the hotkey field shows mid-capture: the modifiers held so far, with a
/// trailing `+` so the following ellipsis reads as "and one more key".
fn modifiers_prefix(m: egui::Modifiers) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if m.ctrl {
        parts.push("Ctrl");
    }
    if m.shift {
        parts.push("Shift");
    }
    if m.alt {
        parts.push("Alt");
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{}+", parts.join("+"))
    }
}

/// Breathing room above a section title. 4, not the mock's 7: only at 4 does
/// the whole Ledger fit a 780 px window without a scrollbar clipping the
/// attribution line.
const SETTINGS_TITLE_PAD_TOP: f32 = 4.0;

/// A section title over a hairline rule, optionally with a right-aligned link.
/// Returns true on the frame that link is clicked.
fn settings_title(ui: &mut Ui, title: &str, right: Option<&str>, p: &Palette) -> bool {
    let mut clicked = false;
    ui.add_space(SETTINGS_TITLE_PAD_TOP);
    ui.horizontal(|ui| {
        ui.set_min_height(18.0);
        ui.label(poe_text(title, 15.0, p.gold));
        if let Some(right) = right {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                clicked = ui.link(right).clicked();
            });
        }
    });
    ui.add_space(3.0);
    settings_rule(ui, p);
    clicked
}

/// The 1 px hairline a section title and the attribution line sit against.
fn settings_rule(ui: &mut Ui, p: &Palette) {
    let (rule, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rule, 0.0, p.border_dark);
}

/// One settings line: a wrapping label on the left, its control hard right.
fn settings_row(
    ui: &mut Ui,
    label: impl Into<egui::WidgetText>,
    min_height: f32,
    control: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.set_min_height(min_height);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            control(ui);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(label).wrap());
            });
        });
    });
}

/// A settings line whose control is a checkbox, with an optional dimmer
/// clarifier trailing the label on the same line.
fn settings_checkbox(
    ui: &mut Ui,
    value: &mut bool,
    label: &str,
    suffix: Option<&str>,
    p: &Palette,
) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        label,
        0.0,
        TextFormat {
            font_id: FontId::new(14.0, FontFamily::Proportional),
            color: p.text,
            ..Default::default()
        },
    );
    if let Some(suffix) = suffix {
        job.append(
            suffix,
            6.0,
            TextFormat {
                font_id: FontId::new(11.5, FontFamily::Proportional),
                color: p.subtle,
                ..Default::default()
            },
        );
    }
    let checked = *value;
    settings_row(ui, job, 30.0, |ui| {
        ui.spacing_mut().icon_width = 14.0;
        // egui paints the tick with one stroke colour whatever the state, so
        // the box outline, not the tick, is what turns gold once checked.
        let widgets = &mut ui.visuals_mut().widgets;
        for style in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
        ] {
            style.fg_stroke = Stroke::new(1.5_f32, p.text);
            style.bg_fill = p.raised;
            style.bg_stroke = Stroke::new(1.0_f32, if checked { p.gold } else { p.border });
        }
        ui.add(egui::Checkbox::without_text(value));
    });
}

/// A 26 px single-line text field in the pane's colours.
fn settings_field(ui: &mut Ui, text: &mut String, width: f32, p: &Palette) {
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        for style in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
        ] {
            style.bg_stroke = Stroke::new(1.0_f32, p.border);
        }
        ui.add(
            egui::TextEdit::singleline(text)
                .desired_width(width)
                .font(FontId::new(13.0, FontFamily::Proportional))
                .text_color(p.text)
                .background_color(p.surface)
                .margin(Margin::symmetric(6, 5)),
        );
    });
}

fn small_button_widget(text: &str, gold: bool, p: &Palette) -> egui::Button<'static> {
    let mut label = RichText::new(text).size(13.0);
    if gold {
        label = label.color(p.gold);
    }
    let mut button = egui::Button::new(label)
        .min_size(Vec2::new(0.0, 26.0))
        .corner_radius(2)
        .wrap_mode(egui::TextWrapMode::Extend);
    if gold {
        button = button.stroke(Stroke::new(1.0_f32, p.gold));
    }
    button
}

fn small_button(ui: &mut Ui, text: &str, gold: bool, p: &Palette) -> egui::Response {
    ui.add(small_button_widget(text, gold, p))
}

/// The left-hand label of a settings row.
fn settings_label(text: &str, p: &Palette) -> RichText {
    RichText::new(text).size(14.0).color(p.text)
}

/// The League dropdown's fixed choices: the bundled snapshot's league, its
/// Hardcore variant, then plain Standard and Hardcore, in that order.
fn league_options(snapshot_league: &str) -> Vec<String> {
    vec![
        snapshot_league.to_string(),
        format!("Hardcore {snapshot_league}"),
        "Standard".to_string(),
        "Hardcore".to_string(),
    ]
}

/// Whether `league` exactly matches one of `options` (case-sensitive).
fn league_is_listed(options: &[String], league: &str) -> bool {
    options.iter().any(|option| option == league)
}

/// The Settings pane: a header row over a ring-framed body, drawn in the
/// central panel in place of the result. Returns the frame's one action.
fn settings_panel(
    ui: &mut Ui,
    cfg: &mut AppConfig,
    update: &UpdateState,
    no_updates: bool,
    recheck: Option<Duration>,
    recording: bool,
    p: &Palette,
) -> Option<SettingsAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.set_min_height(30.0);
        let back = ui.add(
            egui::Button::new(RichText::new("     Back to result"))
                .corner_radius(2)
                .wrap_mode(egui::TextWrapMode::Extend),
        );
        if back.clicked() {
            action = Some(SettingsAction::Back);
        }
        let r = back.rect;
        let c = egui::pos2(r.left() + 14.0, r.center().y);
        ui.painter().add(egui::Shape::line(
            vec![
                egui::pos2(c.x + 2.5, c.y - 4.5),
                egui::pos2(c.x - 2.0, c.y),
                egui::pos2(c.x + 2.5, c.y + 4.5),
            ],
            Stroke::new(1.5_f32, p.gold),
        ));
        ui.add_space(6.0);
        ui.label(poe_text("Settings", 18.0, p.gold));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.link("Open config folder").clicked() {
                action = Some(SettingsAction::OpenFolder);
            }
        });
    });
    ui.add_space(4.0);

    let modifiers = ui.input(|input| input.modifiers);
    Frame::new()
        .fill(p.raised)
        .corner_radius(2)
        .inner_margin(Margin::same(3))
        .stroke(Stroke::new(1.0_f32, p.border))
        .show(ui, |ui| {
            Frame::new()
                .fill(p.bg)
                .inner_margin(Margin {
                    left: 14,
                    right: 14,
                    top: 6,
                    bottom: 10,
                })
                .stroke(Stroke::new(1.0_f32, p.border_dark))
                .show(ui, |ui| {
                    // A solid, allocated track, matching the skills pane.
                    {
                        let scroll = &mut ui.style_mut().spacing.scroll;
                        scroll.floating = false;
                        scroll.bar_width = 10.0;
                        scroll.handle_min_length = 36.0;
                        scroll.bar_inner_margin = 6.0;
                        scroll.bar_outer_margin = 2.0;
                        scroll.foreground_color = true;
                    }
                    ui.visuals_mut().widgets.inactive.fg_stroke =
                        Stroke::new(1.0_f32, p.gold.gamma_multiply(0.72));
                    ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.gold);
                    ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(1.0_f32, p.orange);
                    egui::ScrollArea::vertical()
                        .id_salt("settings_scroll")
                        .auto_shrink([false, false])
                        .animated(false)
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                        )
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.spacing_mut().interact_size.y = 0.0;
                            ui.spacing_mut().item_spacing.y = 0.0;
                            settings_body(
                                ui,
                                cfg,
                                update,
                                no_updates,
                                recheck,
                                recording,
                                modifiers,
                                p,
                                &mut action,
                            );
                        });
                });
        });
    action
}

/// The scrolling half of the Settings pane, split out only so `settings_panel`
/// stays readable.
#[allow(clippy::too_many_arguments)]
fn settings_body(
    ui: &mut Ui,
    cfg: &mut AppConfig,
    update: &UpdateState,
    no_updates: bool,
    recheck: Option<Duration>,
    recording: bool,
    modifiers: egui::Modifiers,
    p: &Palette,
    action: &mut Option<SettingsAction>,
) {
    settings_title(ui, "Appearance", None, p);
    settings_row(ui, settings_label("Theme", p), 34.0, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            // `ui.horizontal` inherits the row's right-to-left direction, so
            // the segments are added back to front to read left to right.
            for (value, label) in [
                ("standard", "Standard"),
                ("dark", "Dark"),
                ("light", "Light"),
                ("system", "Follow Windows"),
            ]
            .into_iter()
            .rev()
            {
                let selected = cfg.theme == value;
                let button =
                    egui::Button::new(RichText::new(label).size(13.0).color(if selected {
                        p.gold
                    } else {
                        p.text
                    }))
                    .fill(if selected { p.raised } else { p.surface })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if selected { p.gold } else { p.border },
                    ))
                    .min_size(Vec2::new(0.0, 26.0))
                    .corner_radius(2)
                    .wrap_mode(egui::TextWrapMode::Extend);
                if ui.add(button).clicked() {
                    cfg.theme = value.to_string();
                    *action = Some(SettingsAction::Save);
                }
            }
        });
    });

    settings_title(ui, "Scanning", None, p);
    let hotkey_error = if recording {
        None
    } else {
        parse_hotkey(&cfg.hotkey).err().map(|e| format!("{e}"))
    };
    settings_row(ui, settings_label("Scan hotkey", p), 34.0, |ui| {
        ui.horizontal(|ui| {
            // Right-to-left row: the button is added first so it sits right
            // of the field.
            if recording {
                if small_button(ui, "Cancel", false, p).clicked() {
                    *action = Some(SettingsAction::CancelRecord);
                }
                let prefix = modifiers_prefix(modifiers);
                Frame::new()
                    .fill(p.surface)
                    .corner_radius(2)
                    .stroke(Stroke::new(1.0_f32, p.gold))
                    .inner_margin(Margin::symmetric(6, 5))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(138.0, 16.0));
                        ui.label(RichText::new(format!("{prefix}…")).size(13.0).color(p.gold));
                    });
            } else {
                if small_button(ui, "Record", false, p).clicked() {
                    *action = Some(SettingsAction::Record);
                }
                if hotkey_error.is_some() {
                    // The TextEdit keeps its own frame; this only rings it red.
                    Frame::new()
                        .stroke(Stroke::new(1.0_f32, p.red))
                        .show(ui, |ui| settings_field(ui, &mut cfg.hotkey, 150.0, p));
                } else {
                    settings_field(ui, &mut cfg.hotkey, 150.0, p);
                }
            }
        });
    });
    let (help, help_color) = match (recording, &hotkey_error) {
        (true, _) => (
            "Press the key combination now. Escape cancels.".to_string(),
            p.gold,
        ),
        (false, Some(error)) => (format!("Invalid hotkey: {error}"), p.red),
        (false, None) => (
            "Applies after restart. Ctrl, Shift, Alt, Win plus a letter or digit.".to_string(),
            p.subtle,
        ),
    };
    // Inside a vertical parent a bare right_to_left child claims the whole
    // remaining height and centres in it, so the line needs its own row.
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(help).size(11.5).color(help_color)).wrap());
        });
    });
    settings_checkbox(
        ui,
        &mut cfg.scan_clipboard_first,
        "Read copied Warrant text before screen capture",
        None,
        p,
    );
    settings_checkbox(
        ui,
        &mut cfg.assume_projectile_speed,
        "Assume 150%+ projectile speed for Frost Blades",
        None,
        p,
    );

    settings_title(ui, "Window", None, p);
    settings_checkbox(ui, &mut cfg.always_on_top, "Always on top", None, p);
    settings_checkbox(
        ui,
        &mut cfg.hide_on_escape,
        "Hide the window when Escape is pressed",
        None,
        p,
    );

    settings_title(ui, "Trade", None, p);
    let (patch, snapshot_league, _source) = poemercpricer::trade::bundled_trade_stat_provenance();
    let league_choices = league_options(snapshot_league);
    let listed = league_is_listed(&league_choices, &cfg.trade_league);
    let custom_id = ui.id().with("league_custom");
    let mut custom_chosen = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(custom_id))
        .unwrap_or(false);
    settings_row(ui, settings_label("League", p), 34.0, |ui| {
        ui.spacing_mut().interact_size.y = 26.0;
        let selected_text = if listed && !custom_chosen {
            cfg.trade_league.clone()
        } else {
            "Custom".to_string()
        };
        egui::ComboBox::from_id_salt("trade_league")
            .width(150.0)
            .selected_text(RichText::new(selected_text).size(13.0).color(p.text))
            .show_ui(ui, |ui| {
                for option in &league_choices {
                    let selected = listed && !custom_chosen && cfg.trade_league == *option;
                    if ui
                        .selectable_label(selected, RichText::new(option.as_str()).size(13.0))
                        .clicked()
                    {
                        cfg.trade_league = option.clone();
                        custom_chosen = false;
                        ui.ctx().data_mut(|d| d.insert_temp(custom_id, false));
                        *action = Some(SettingsAction::Save);
                    }
                }
                if ui
                    .selectable_label(!listed || custom_chosen, RichText::new("Custom").size(13.0))
                    .clicked()
                {
                    custom_chosen = true;
                    ui.ctx().data_mut(|d| d.insert_temp(custom_id, true));
                }
            });
    });
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "From the bundled {patch} snapshot. Pick Custom for any other league name."
                    ))
                    .size(11.5)
                    .color(p.subtle),
                )
                .wrap(),
            );
        });
    });
    if !listed || custom_chosen {
        settings_row(ui, settings_label("Custom league", p), 34.0, |ui| {
            settings_field(ui, &mut cfg.trade_league, 150.0, p);
        });
    }
    settings_checkbox(
        ui,
        &mut cfg.trade_every_skill,
        "Search every skill",
        Some("(up to 5; needs a pathofexile.com login)"),
        p,
    );

    if settings_title(ui, "Updates", Some("Release notes"), p) {
        *action = Some(SettingsAction::OpenReleases);
    }
    let (line1, line2) = update_copy(
        update,
        cfg.check_updates && !no_updates,
        no_updates,
        recheck,
    );
    let line1_color = if matches!(update, UpdateState::Failed { .. }) {
        p.red
    } else {
        p.text
    };
    ui.horizontal(|ui| {
        ui.set_min_height(34.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            match update {
                UpdateState::Ready { .. } => {
                    if small_button(ui, "Restart to update", true, p).clicked() {
                        *action = Some(SettingsAction::Restart);
                    }
                }
                UpdateState::Available(release) => {
                    if small_button(ui, &format!("Update to {}", release.version), true, p)
                        .clicked()
                    {
                        *action = Some(SettingsAction::Install);
                    }
                }
                // A launch that failed offline has to be retryable without a restart.
                UpdateState::Failed { .. } => {
                    ui.horizontal(|ui| {
                        if small_button(ui, "Open releases page", false, p).clicked() {
                            *action = Some(SettingsAction::OpenReleases);
                        }
                        if small_button(ui, "Check now", false, p).clicked() {
                            *action = Some(SettingsAction::CheckNow);
                        }
                    });
                }
                state => {
                    let busy = matches!(state, UpdateState::Checking | UpdateState::Downloading(_));
                    if ui
                        .add_enabled(!busy, small_button_widget("Check now", false, p))
                        .clicked()
                    {
                        *action = Some(SettingsAction::CheckNow);
                    }
                }
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.add(
                        egui::Label::new(RichText::new(line1).size(14.0).color(line1_color)).wrap(),
                    );
                    if !line2.is_empty() {
                        ui.add(
                            egui::Label::new(RichText::new(line2).size(11.5).color(p.muted)).wrap(),
                        );
                    }
                });
            });
        });
    });
    settings_checkbox(
        ui,
        &mut cfg.check_updates,
        "Check for updates at startup and every 6 hours",
        None,
        p,
    );
    settings_checkbox(
        ui,
        &mut cfg.install_updates_automatically,
        "Install updates automatically",
        Some("(you still choose when to restart)"),
        p,
    );

    ui.add_space(SETTINGS_TITLE_PAD_TOP);
    let id = ui.make_persistent_id("advanced_settings");
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
    let expanded = state.is_open();
    let header = ui.horizontal(|ui| {
        ui.set_min_height(30.0);
        disclosure_chevron(ui, expanded, true, 22.0, p);
        ui.label(RichText::new("Advanced").size(14.0).color(p.muted));
        ui.label(
            RichText::new("Diagnostic capture, Path of Exile window title")
                .size(11.5)
                .color(p.subtle),
        );
    });
    let response = ui
        .interact(
            header.response.rect,
            ui.id().with("advanced_disclosure"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    focus_ring(ui, &response, p);
    if response.clicked() {
        state.toggle(ui);
        state.store(ui.ctx());
        ui.ctx().request_repaint();
    }
    if state.is_open() {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.vertical(|ui| {
                settings_checkbox(
                    ui,
                    &mut cfg.dump_debug,
                    "Save the last capture for diagnostics",
                    None,
                    p,
                );
                settings_row(
                    ui,
                    settings_label("Path of Exile window title", p),
                    34.0,
                    |ui| {
                        settings_field(ui, &mut cfg.poe_window_title, 150.0, p);
                    },
                );
            });
        });
    }

    // The attribution sits on the pane's floor, not under the last section.
    ui.add_space((ui.available_height() - 48.0).max(0.0));
    settings_rule(ui, p);
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            RichText::new(
                "Path of Exile and its game artwork are owned by or licensed to Grinding Gear Games Limited. Not affiliated with or endorsed by Grinding Gear Games.",
            )
            .size(11.0)
            .color(p.subtle),
        )
        .wrap(),
    );
}

/// Also the verdict title's text colour, so every entry keeps 4.5:1 on its palette's `bg`.
fn band_color(band: &str, jackpot: bool, p: &Palette) -> Color32 {
    if is_take_item(band, jackpot) {
        return p.take_green;
    }
    match band {
        "skip" => p.red,
        "check" => p.check,
        "common" => p.orange,
        _ => p.muted,
    }
}

/// Good, very valuable and jackpot all read as one unmistakable green so a
/// worthwhile Warrant is obvious at a glance; the title text carries the tier.
fn is_take_item(band: &str, jackpot: bool) -> bool {
    jackpot || matches!(band, "good" | "very-valuable" | "jackpot" | "jackpot-band")
}

/// Loads the serif display font once; independent of the palette, so it is
/// called only from `PricerApp::new`, never on a theme change.
/// Only Ubuntu-Light is embedded. egui's other three bundled fonts (Hack for
/// Monospace, two emoji faces) never draw a glyph in this UI, 1.05 MB of exe;
/// `tests/samples.rs` asserts every non-ASCII character in `src/` is covered.
const UBUNTU_LIGHT: &[u8] = include_bytes!("../assets/fonts/Ubuntu-Light.ttf");

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::empty();
    fonts.font_data.insert(
        "ubuntu_light".into(),
        egui::FontData::from_static(UBUNTU_LIGHT).into(),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.insert(family, vec!["ubuntu_light".into()]);
    }
    let mut serif_fonts = Vec::new();
    let windir =
        std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    if let Ok(bytes) = std::fs::read(windir.join("Fonts").join("georgia.ttf")) {
        fonts
            .font_data
            .insert("poe_serif".into(), egui::FontData::from_owned(bytes).into());
        serif_fonts.push("poe_serif".into());
    }
    if let Some(fallbacks) = fonts.families.get(&FontFamily::Proportional) {
        serif_fonts.extend(fallbacks.iter().cloned());
    }
    fonts.families.insert(POE_SERIF.clone(), serif_fonts);
    ctx.set_fonts(fonts);
}

/// Re-applies `Visuals` and `Style` for one palette; called once at startup
/// and again whenever `update()` sees the resolved palette change.
fn apply_theme(ctx: &egui::Context, p: &Palette) {
    let mut visuals = if *p == LIGHT {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    visuals.panel_fill = p.bg;
    visuals.window_fill = p.bg;
    visuals.extreme_bg_color = p.surface;
    visuals.faint_bg_color = p.raised;
    visuals.override_text_color = Some(p.text);
    visuals.widgets.noninteractive.bg_fill = p.surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.inactive.bg_fill = p.raised;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.hovered.bg_fill = p.hovered;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.gold);
    visuals.widgets.active.bg_fill = p.hovered;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.gold);
    visuals.widgets.open.bg_fill = p.raised;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.gold.gamma_multiply(0.7));
    visuals.selection.bg_fill = p.gold.gamma_multiply(0.45);
    visuals.hyperlink_color = p.gold;
    visuals.window_stroke = Stroke::new(1.0_f32, p.border);
    visuals.striped = true;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.interact_size.y = 30.0;
    style.visuals.menu_corner_radius = 2.into();
    style.visuals.window_corner_radius = 2.into();
    // Immediate hover/press states avoid a burst of animation-driven redraws
    // on Windows and keep the compact overlay visually stable while it is used.
    style.animation_time = 0.0;
    ctx.set_style(style);
}

fn register_hotkey(spec: &str) -> Result<(GlobalHotKeyManager, HotKey)> {
    let manager = GlobalHotKeyManager::new().context("creating the hotkey manager")?;
    let hk = parse_hotkey(spec)?;
    manager
        .register(hk)
        .with_context(|| format!("registering {spec} (already taken by another app?)"))?;
    Ok((manager, hk))
}

fn install_hotkey_handler(
    hotkey: Option<HotKey>,
    tx: Sender<ScanMsg>,
    ctx: egui::Context,
    hwnd: Option<isize>,
) {
    let hotkey_id = hotkey.map(|value| value.id());
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if is_scan_hotkey_press(hotkey_id, event.id, event.state) {
            if let Some(h) = hwnd {
                show_window(h);
            }
            let _ = tx.send(ScanMsg::HotkeyPressed);
            ctx.request_repaint();
        }
    }));
}

fn is_scan_hotkey_press(hotkey_id: Option<u32>, event_id: u32, state: HotKeyState) -> bool {
    hotkey_id == Some(event_id) && state == HotKeyState::Pressed
}

fn parse_hotkey(spec: &str) -> Result<HotKey> {
    let mut mods = Modifiers::empty();
    let mut code = None;
    for part in spec.split(['+', '-']) {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "ctl" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "win" | "super" | "meta" => mods |= Modifiers::SUPER,
            "esc" | "escape" => code = Some(Code::Escape),
            key if key.len() == 1 => {
                let c = key.chars().next().unwrap();
                code = Some(
                    letter_code(c).ok_or_else(|| anyhow::anyhow!("unknown hotkey key: {key}"))?,
                );
            }
            other => anyhow::bail!("unknown hotkey token: {other}"),
        }
    }
    let code = code.ok_or_else(|| anyhow::anyhow!("hotkey missing a key"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn letter_code(c: char) -> Option<Code> {
    Some(match c.to_ascii_uppercase() {
        '0' => Code::Digit0,
        '1' => Code::Digit1,
        '2' => Code::Digit2,
        '3' => Code::Digit3,
        '4' => Code::Digit4,
        '5' => Code::Digit5,
        '6' => Code::Digit6,
        '7' => Code::Digit7,
        '8' => Code::Digit8,
        '9' => Code::Digit9,
        'A' => Code::KeyA,
        'B' => Code::KeyB,
        'C' => Code::KeyC,
        'D' => Code::KeyD,
        'E' => Code::KeyE,
        'F' => Code::KeyF,
        'G' => Code::KeyG,
        'H' => Code::KeyH,
        'I' => Code::KeyI,
        'J' => Code::KeyJ,
        'K' => Code::KeyK,
        'L' => Code::KeyL,
        'M' => Code::KeyM,
        'N' => Code::KeyN,
        'O' => Code::KeyO,
        'P' => Code::KeyP,
        'Q' => Code::KeyQ,
        'R' => Code::KeyR,
        'S' => Code::KeyS,
        'T' => Code::KeyT,
        'U' => Code::KeyU,
        'V' => Code::KeyV,
        'W' => Code::KeyW,
        'X' => Code::KeyX,
        'Y' => Code::KeyY,
        'Z' => Code::KeyZ,
        _ => return None,
    })
}

/// Returns the scored mercenary and which input produced it.
fn run_scan(
    cfg: &AppConfig,
    request: ScanRequest,
) -> Result<(Mercenary, ScoreResult, &'static str)> {
    let tries_clipboard = should_try_clipboard(&request, cfg.scan_clipboard_first);
    if tries_clipboard {
        if let Some(merc) = warrant_from_clipboard() {
            let result = score_mercenary(&merc, cfg.assume_projectile_speed);
            return Ok((merc, result, "clipboard"));
        }
    }

    let (img, source) = match request {
        ScanRequest::Image(path) => (capture::load_image(&path)?, "image"),
        ScanRequest::Clipboard => (
            capture::clipboard_image_rgba()?.ok_or_else(|| {
                anyhow::anyhow!("Clipboard does not contain Warrant text or an image")
            })?,
            "clipboard image",
        ),
        ScanRequest::Hotkey | ScanRequest::Screen => (
            capture::capture_poe_or_primary(&cfg.poe_window_title)?,
            "screen",
        ),
    };
    if cfg.dump_debug {
        let path = AppConfig::dir().join("debug").join("last-capture.png");
        let _ = capture::save_debug(&img, &path);
    }
    let merc = poemercpricer::scan::scan_rgba(&img)?;
    let result = score_mercenary(&merc, cfg.assume_projectile_speed);
    Ok((merc, result, source))
}

fn should_try_clipboard(request: &ScanRequest, clipboard_first: bool) -> bool {
    matches!(request, ScanRequest::Clipboard)
        || (matches!(request, ScanRequest::Hotkey) && clipboard_first)
}

fn warrant_from_clipboard() -> Option<Mercenary> {
    let text = with_clipboard(|clip| clip.get_text()).ok()?;
    looks_like_warrant(&text).then(|| parse_warrant_text(&text))
}

/// arboard opens the clipboard per call and fails transiently while another
/// process holds it (ERROR_ACCESS_DENIED), so retry once after a short wait.
fn with_clipboard<T>(
    op: impl Fn(&mut arboard::Clipboard) -> Result<T, arboard::Error>,
) -> Result<T, arboard::Error> {
    let attempt = || arboard::Clipboard::new().and_then(|mut clip| op(&mut clip));
    match attempt() {
        Err(e) if !matches!(e, arboard::Error::ContentNotAvailable) => {
            thread::sleep(Duration::from_millis(20));
            attempt()
        }
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use global_hotkey::hotkey::{Code, Modifiers};
    use global_hotkey::HotKeyState;
    use poemercpricer::models::{Mercenary, ScoreResult, Skill, SupportGem, SupportTier};

    use poemercpricer::update::{Release, UpdateState, CURRENT};
    use std::time::{Duration, Instant};

    use super::{
        ambiguous_support_choices, apply_support_choice, band_color, bar_shows_meta, checked_ago,
        hotkey_spec, is_scan_hotkey_press, league_is_listed, league_options, megabytes,
        modifiers_prefix, palette_for, parse_hotkey, recheck_in, scan_failed, scan_is_stuck,
        scan_watchdog_wake, score_caption, score_number, scroll_cue_visible, should_try_clipboard,
        shows_icon_strip, status_message, support_columns, take_always_on_top_change, update_copy,
        verdict_detail, verdict_title, ScanRequest, SupportChoice, DARK, LIGHT, SCAN_WATCHDOG,
        STANDARD, UPDATE_RECHECK,
    };

    #[test]
    fn league_options_orders_snapshot_league_first() {
        assert_eq!(
            league_options("Allflame"),
            vec![
                "Allflame".to_string(),
                "Hardcore Allflame".to_string(),
                "Standard".to_string(),
                "Hardcore".to_string(),
            ]
        );
    }

    #[test]
    fn league_is_listed_matches_standard_but_not_ruthless() {
        let options = league_options("Allflame");
        assert!(league_is_listed(&options, "Standard"));
        assert!(!league_is_listed(&options, "Ruthless"));
    }

    #[test]
    fn hotkey_spec_orders_modifiers_and_names_the_key() {
        let ctrl_shift = super::egui::Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        assert_eq!(hotkey_spec(ctrl_shift, super::egui::Key::M), "Ctrl+Shift+M");
        assert_eq!(
            hotkey_spec(super::egui::Modifiers::default(), super::egui::Key::Num1),
            "1"
        );
    }

    #[test]
    fn hotkey_spec_can_name_a_key_parse_hotkey_rejects() {
        let alt = super::egui::Modifiers {
            alt: true,
            ..Default::default()
        };
        let spec = hotkey_spec(alt, super::egui::Key::F5);
        assert_eq!(spec, "Alt+F5");
        assert!(parse_hotkey(&spec).is_err());
    }

    #[test]
    fn modifiers_prefix_is_empty_until_a_modifier_is_held() {
        assert_eq!(modifiers_prefix(super::egui::Modifiers::default()), "");
        assert_eq!(
            modifiers_prefix(super::egui::Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            }),
            "Ctrl+Shift+"
        );
    }

    fn release() -> Release {
        Release {
            version: semver::Version::new(0, 3, 0),
            page: "https://github.com/Lisood/PoEMercPricer/releases/tag/v0.3.0".into(),
            download: "https://github.com/Lisood/PoEMercPricer/releases/download/v0.3.0/poemercpricer-windows-x64.exe".into(),
            compressed: None,
            size: 12_300_000,
            sha256: "0".repeat(64),
        }
    }

    #[test]
    fn update_copy_matches_the_spec_table_for_every_state() {
        assert_eq!(
            update_copy(&UpdateState::Idle, true, false, None),
            (
                "Not checked yet.".into(),
                "Checks when the app starts and every 6 hours.".into()
            )
        );
        assert_eq!(
            update_copy(&UpdateState::Idle, false, false, None),
            ("Update checks are off.".into(), "Off in Settings.".into())
        );
        assert_eq!(
            update_copy(&UpdateState::Idle, false, true, None),
            (
                "Update checks are off.".into(),
                "Off with --no-updates.".into()
            )
        );
        // --no-updates wins over the setting; this is how the Settings pane
        // combines the two.
        let (check_updates, no_updates) = (true, true);
        assert_eq!(
            update_copy(
                &UpdateState::Idle,
                check_updates && !no_updates,
                no_updates,
                None
            ),
            (
                "Update checks are off.".into(),
                "Off with --no-updates.".into()
            )
        );
        assert_eq!(
            update_copy(&UpdateState::Checking, true, false, None),
            ("Checking GitHub for a newer release…".into(), String::new())
        );
        assert_eq!(
            update_copy(
                &UpdateState::UpToDate {
                    checked: Instant::now()
                },
                true,
                false,
                None
            ),
            (
                format!("{CURRENT} is the latest release."),
                "Checked just now.".into()
            )
        );
        assert_eq!(
            update_copy(
                &UpdateState::UpToDate {
                    checked: Instant::now()
                },
                true,
                false,
                Some(Duration::from_secs(5 * 3600))
            )
            .1,
            "Checked just now. Checks again in about 5 h."
        );
        assert_eq!(
            update_copy(&UpdateState::Available(release()), true, false, None),
            (
                "0.3.0 is available.".into(),
                "Install updates automatically is off. Update to 0.3.0 downloads it now; it runs after a restart.".into()
            )
        );
        assert_eq!(
            update_copy(&UpdateState::Downloading(release()), true, false, None),
            (
                "Downloading 0.3.0 (12 MB)…".into(),
                "The current version keeps working until you restart.".into()
            )
        );
        assert_eq!(
            update_copy(
                &UpdateState::Ready {
                    version: "0.3.0".into()
                },
                true,
                false,
                None
            ),
            (
                "0.3.0 is installed.".into(),
                format!(
                    "It runs the next time you start PoEMercPricer, or restart now. Until then {CURRENT} keeps running."
                )
            )
        );
        assert_eq!(
            update_copy(
                &UpdateState::Failed {
                    message: "no internet connection".into()
                },
                true,
                false,
                None
            ),
            (
                "Could not update: no internet connection.".into(),
                "Nothing was changed. You can download it yourself.".into()
            )
        );
    }

    #[test]
    fn recheck_is_off_when_checks_are_off_or_an_update_is_in_flight() {
        let now = Instant::now();
        assert_eq!(recheck_in(None, &UpdateState::Idle, false), None);
        assert_eq!(
            recheck_in(
                Some(now),
                &UpdateState::Ready {
                    version: "0.3.0".into()
                },
                true
            ),
            None
        );
        assert_eq!(recheck_in(Some(now), &UpdateState::Checking, true), None);
        assert_eq!(
            recheck_in(Some(now), &UpdateState::Downloading(release()), true),
            None
        );
    }

    #[test]
    fn recheck_is_due_immediately_then_counts_down_the_interval() {
        assert_eq!(
            recheck_in(None, &UpdateState::Idle, true),
            Some(Duration::ZERO)
        );
        // Instant is monotonic from boot, so a machine up for less than the
        // offset cannot express the past instant; skip rather than panic.
        let now = Instant::now();
        let (Some(hour_ago), Some(seven_hours_ago)) = (
            now.checked_sub(Duration::from_secs(3600)),
            now.checked_sub(Duration::from_secs(7 * 3600)),
        ) else {
            return;
        };
        let wait = recheck_in(
            Some(hour_ago),
            &UpdateState::UpToDate { checked: hour_ago },
            true,
        )
        .expect("a checked state with checks on is due for a re-check");
        let expected = UPDATE_RECHECK - Duration::from_secs(3600);
        assert!(
            wait.abs_diff(expected) < Duration::from_secs(1),
            "{wait:?} is not about {expected:?}"
        );
        assert_eq!(
            recheck_in(Some(seven_hours_ago), &UpdateState::Idle, true),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn a_scan_counts_as_wedged_only_after_the_watchdog() {
        let now = Instant::now();
        assert!(!scan_is_stuck(false, None));
        assert!(!scan_is_stuck(true, Some(now)));
        // Instant is monotonic from boot, so a machine up for less than the
        // offset cannot express the past instant; skip rather than panic.
        let (Some(just_short), Some(past_deadline)) = (
            now.checked_sub(SCAN_WATCHDOG - Duration::from_secs(1)),
            now.checked_sub(SCAN_WATCHDOG + Duration::from_secs(1)),
        ) else {
            return;
        };
        assert!(!scan_is_stuck(true, Some(just_short)));
        assert!(scan_is_stuck(true, Some(past_deadline)));
        // A finished scan clears `scan_started`, so nothing is ever stuck.
        assert!(!scan_is_stuck(false, Some(past_deadline)));
        // The UI asks for one wake-up while the deadline is still ahead, and
        // none once it has passed: the buttons are already enabled by then.
        let wake = scan_watchdog_wake(true, Some(just_short))
            .expect("a running scan wakes the UI at its deadline");
        assert!(
            wake.abs_diff(Duration::from_secs(1)) < Duration::from_millis(500),
            "{wake:?} is not about 1 s"
        );
        assert_eq!(scan_watchdog_wake(true, Some(past_deadline)), None);
        assert_eq!(scan_watchdog_wake(false, None), None);
    }

    #[test]
    fn checked_ago_reads_in_seconds_then_minutes_then_hours() {
        assert_eq!(checked_ago(Duration::from_secs(5)), "Checked just now.");
        assert_eq!(checked_ago(Duration::from_secs(180)), "Checked 3 min ago.");
        assert_eq!(checked_ago(Duration::from_secs(7200)), "Checked 2 h ago.");
    }

    #[test]
    fn download_size_rounds_to_whole_megabytes() {
        assert_eq!(megabytes(12_300_000), 12);
        assert_eq!(megabytes(12_600_000), 13);
    }

    #[test]
    fn hotkey_digits_map_to_digit_codes_and_unknown_keys_fail() {
        let hk = parse_hotkey("Ctrl+Shift+1").unwrap();
        assert_eq!(hk.key, Code::Digit1);
        assert_eq!(hk.mods, Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(parse_hotkey("ctrl-m").unwrap().key, Code::KeyM);
        assert!(parse_hotkey("Ctrl+Shift+F5").is_err());
        assert!(parse_hotkey("Ctrl+Shift+!").is_err());
        assert!(parse_hotkey("Ctrl+Shift").is_err());
    }

    fn relative_luminance(color: egui::Color32) -> f32 {
        let [red, green, blue, _] = color.to_array();
        [red, green, blue]
            .into_iter()
            .zip([0.2126_f32, 0.7152, 0.0722])
            .map(|(channel, weight)| {
                let value = f32::from(channel) / 255.0;
                let linear = if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                };
                linear * weight
            })
            .sum()
    }

    fn contrast_ratio(foreground: egui::Color32, background: egui::Color32) -> f32 {
        let foreground = relative_luminance(foreground);
        let background = relative_luminance(background);
        (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
    }

    fn ambiguous_holy_flame_totem() -> Mercenary {
        let mut skill = Skill::new("Holy Flame Totem", "Holy Flame Totem");
        skill.supports.push(SupportGem {
            name: "Ironwood / Physical as Extra".into(),
            canonical: "ambiguous".into(),
            tier: SupportTier::T2,
            confidence: 0.91,
            raw: "icon@1,2 T2".into(),
        });
        Mercenary {
            skills: vec![skill],
            ..Default::default()
        }
    }

    #[test]
    fn window_level_command_is_only_needed_on_transition() {
        let mut applied = true;

        assert_eq!(take_always_on_top_change(&mut applied, true), None);
        assert_eq!(take_always_on_top_change(&mut applied, false), Some(false));
        assert_eq!(take_always_on_top_change(&mut applied, false), None);
        assert_eq!(take_always_on_top_change(&mut applied, true), Some(true));
    }

    #[test]
    fn status_copy_is_contextual_without_synthetic_ready_state() {
        assert_eq!(
            status_message("Press Ctrl+M to scan a mercenary panel", false, false, None),
            "Press Ctrl+M to scan a mercenary panel"
        );
        assert_eq!(
            status_message(
                "Scan complete (clipboard)",
                false,
                false,
                Some(std::time::Duration::from_millis(131))
            ),
            "Scan complete (clipboard) in 131 ms"
        );
        assert_eq!(
            status_message(
                "Summary copied",
                false,
                false,
                Some(std::time::Duration::from_millis(131))
            ),
            "Summary copied"
        );
    }

    #[test]
    fn scan_failed_reports_the_error_only_after_a_failed_scan() {
        assert_eq!(
            scan_failed(Some("x"), "Scan failed: x"),
            Some("x.".to_string())
        );
        assert_eq!(scan_failed(Some("x"), "Settings not saved"), None);
        assert_eq!(scan_failed(None, "Scan failed: x"), None);
        assert_eq!(
            scan_failed(Some("already ends."), "Scan failed: already ends."),
            Some("already ends.".to_string())
        );
    }

    #[test]
    fn every_text_tone_meets_normal_text_contrast_on_every_surface() {
        for (name, p) in [("standard", STANDARD), ("dark", DARK), ("light", LIGHT)] {
            for foreground in [
                p.text,
                p.muted,
                p.subtle,
                p.gold,
                p.orange,
                p.red,
                p.green,
                p.take_green,
                p.check,
            ] {
                for background in [p.bg, p.surface, p.raised] {
                    let ratio = contrast_ratio(foreground, background);
                    assert!(
                        ratio >= 4.5,
                        "{name} text contrast {ratio:.2}:1 is below 4.5:1"
                    );
                }
            }
            for band in [
                "skip",
                "common",
                "check",
                "good",
                "very-valuable",
                "jackpot-band",
                "unsupported",
                "jackpot",
            ] {
                let ratio = contrast_ratio(band_color(band, false, &p), p.bg);
                assert!(
                    ratio >= 4.5,
                    "{name} {band} verdict contrast {ratio:.2}:1 is below 4.5:1"
                );
            }
            let ratio = contrast_ratio(p.gold_fill_text, p.gold_fill);
            assert!(
                ratio >= 4.5,
                "{name} gold_fill_text contrast {ratio:.2}:1 is below 4.5:1"
            );
        }
    }

    #[test]
    fn palette_for_falls_back_to_standard_on_unknown_theme_or_missing_system_answer() {
        assert_eq!(palette_for("standard", None), STANDARD);
        assert_eq!(palette_for("dark", None), DARK);
        assert_eq!(palette_for("light", None), LIGHT);
        assert_eq!(palette_for("nonsense", None), STANDARD);
        assert_eq!(palette_for("system", None), STANDARD);
        assert_eq!(palette_for("system", Some(egui::Theme::Dark)), DARK);
        assert_eq!(palette_for("system", Some(egui::Theme::Light)), LIGHT);
    }

    #[test]
    fn scroll_cue_only_when_skills_overflow() {
        assert!(!scroll_cue_visible(300.0, 300.0));
        assert!(!scroll_cue_visible(200.0, 300.0));
        assert!(scroll_cue_visible(301.0, 300.0));
    }

    #[test]
    fn icon_strip_needs_room() {
        assert!(shows_icon_strip(560.0));
        assert!(!shows_icon_strip(499.0));
    }

    #[test]
    fn support_grid_goes_two_columns_at_560px() {
        assert_eq!(support_columns(560.0), 2);
        assert_eq!(support_columns(559.0), 1);
    }

    #[test]
    fn clipboard_button_never_depends_on_preference() {
        assert!(should_try_clipboard(&ScanRequest::Clipboard, false));
        assert!(should_try_clipboard(&ScanRequest::Clipboard, true));
        assert!(!should_try_clipboard(&ScanRequest::Hotkey, false));
        assert!(should_try_clipboard(&ScanRequest::Hotkey, true));
        assert!(!should_try_clipboard(&ScanRequest::Screen, true));
        assert!(!should_try_clipboard(
            &ScanRequest::Image("fixture.png".into()),
            true
        ));
    }

    #[test]
    fn hotkey_only_scans_on_matching_key_down() {
        assert!(is_scan_hotkey_press(Some(42), 42, HotKeyState::Pressed));
        assert!(!is_scan_hotkey_press(Some(42), 42, HotKeyState::Released));
        assert!(!is_scan_hotkey_press(Some(42), 7, HotKeyState::Pressed));
        assert!(!is_scan_hotkey_press(None, 42, HotKeyState::Pressed));
    }

    #[test]
    fn decision_copy_is_clear_and_consistent() {
        let skip = ScoreResult {
            band: "skip".into(),
            score: 0.0,
            ..Default::default()
        };
        assert_eq!(verdict_title(&skip), "SKIP THIS WARRANT");
        assert_eq!(score_number(&skip), "0");
        assert_eq!(score_caption(&skip), "of 100");

        let unsupported = ScoreResult::default();
        assert_eq!(verdict_title(&unsupported), "NO RELIABLE PRICE VERDICT");
        assert_eq!(score_number(&unsupported), "—");
        assert_eq!(score_caption(&unsupported), "Not scored");

        let jackpot = ScoreResult {
            band: "good".into(),
            jackpot: true,
            ..Default::default()
        };
        assert_eq!(verdict_title(&jackpot), "TAKE ITEM: JACKPOT CANDIDATE");

        let bricked = ScoreResult {
            band: "skip".into(),
            bricks: vec!["Icicle Rain".into()],
            ..Default::default()
        };
        assert_eq!(verdict_title(&bricked), "SKIP: BRICKED");
        assert_eq!(
            verdict_detail(&bricked).as_deref(),
            Some("BRICKED BY: Icicle Rain")
        );
    }

    #[test]
    fn ambiguous_support_exposes_only_catalog_backed_tier_labels() {
        let merc = ambiguous_holy_flame_totem();
        let choices = ambiguous_support_choices(&merc.skills[0].supports[0]);
        assert_eq!(
            choices,
            vec![
                ("ironwood".into(), "Ironwood".into()),
                ("physical_as_extra".into(), "Physical as Extra".into()),
            ]
        );
    }

    #[test]
    fn user_selection_replaces_ambiguity_with_exact_scoring_identity() {
        let mut merc = ambiguous_holy_flame_totem();
        let choice = SupportChoice {
            skill_index: 0,
            support_index: 0,
            canonical: "physical_as_extra".into(),
        };

        assert!(apply_support_choice(&mut merc, &choice));
        let support = &merc.skills[0].supports[0];
        assert_eq!(support.canonical, "physical_as_extra");
        assert_eq!(support.name, "Physical as Extra");
        assert_eq!(support.confidence, 1.0);
        assert!(support.raw.contains("user selected exact support"));
        assert!(!apply_support_choice(&mut merc, &choice));
    }

    #[test]
    fn user_selection_rejects_a_name_outside_the_visible_candidate_set() {
        let mut merc = ambiguous_holy_flame_totem();
        let choice = SupportChoice {
            skill_index: 0,
            support_index: 0,
            canonical: "added_fire".into(),
        };

        assert!(!apply_support_choice(&mut merc, &choice));
        assert_eq!(merc.skills[0].supports[0].canonical, "ambiguous");
    }

    #[test]
    fn command_bar_meta_yields_to_width_and_an_update_button() {
        assert!(bar_shows_meta(560.0, false));
        assert!(!bar_shows_meta(519.0, false));
        assert!(!bar_shows_meta(560.0, true));
    }
}
