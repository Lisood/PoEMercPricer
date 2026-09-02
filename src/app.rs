use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui::{
    self, Color32, FontFamily, FontId, Frame, Margin, RichText, Stroke, TextStyle, TextureHandle,
    Ui, Vec2, Visuals,
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
use poemercpricer::scoring::score_mercenary;

const GOLD: Color32 = Color32::from_rgb(199, 160, 91);
const TEXT: Color32 = Color32::from_rgb(224, 214, 195);
const MUTED: Color32 = Color32::from_rgb(176, 161, 137);
const SUBTLE: Color32 = Color32::from_rgb(154, 139, 114);
const BG: Color32 = Color32::from_rgb(10, 9, 8);
const SURFACE: Color32 = Color32::from_rgb(19, 17, 14);
const SURFACE_RAISED: Color32 = Color32::from_rgb(29, 25, 20);
const BORDER: Color32 = Color32::from_rgb(79, 64, 43);
const BORDER_DARK: Color32 = Color32::from_rgb(44, 36, 26);
const ORANGE: Color32 = Color32::from_rgb(211, 106, 40);
const RED: Color32 = Color32::from_rgb(232, 118, 112);
const GREEN: Color32 = Color32::from_rgb(130, 200, 137);
const TRADE_OPEN_COOLDOWN: Duration = Duration::from_secs(2);

enum ScanMsg {
    Ok(Box<(Mercenary, ScoreResult, Duration)>),
    Err(String),
    HotkeyPressed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScanRequest {
    Hotkey,
    Screen,
    Clipboard,
    Image(PathBuf),
}

#[derive(Default)]
struct IconCache {
    textures: HashMap<String, Option<TextureHandle>>,
}

impl IconCache {
    fn skill(&mut self, ctx: &egui::Context, name: &str) -> Option<TextureHandle> {
        self.load(ctx, "skills", skill_icon(name)?)
    }

    fn support(&mut self, ctx: &egui::Context, canonical: &str) -> Option<TextureHandle> {
        self.load(ctx, "supports", support_icon(canonical)?)
    }

    fn load(&mut self, ctx: &egui::Context, kind: &str, file: &str) -> Option<TextureHandle> {
        let key = format!("{kind}/{file}");
        if let Some(cached) = self.textures.get(&key) {
            return cached.clone();
        }
        let texture = icon_asset_path(kind, file)
            .and_then(|path| image::open(path).ok())
            .map(|image| image.to_rgba8())
            .map(|image| {
                let size = [image.width() as usize, image.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                ctx.load_texture(&key, color, egui::TextureOptions::LINEAR)
            });
        self.textures.insert(key, texture.clone());
        texture
    }
}

fn icon_asset_path(kind: &str, file: &str) -> Option<PathBuf> {
    let relative = Path::new("assets").join("icons").join(kind).join(file);
    let mut candidates = vec![std::env::current_dir().ok()?.join(&relative)];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(&relative));
            candidates.push(dir.join("..").join("..").join(&relative));
        }
    }
    candidates.into_iter().find(|path| path.is_file())
}

pub struct PricerApp {
    cfg: AppConfig,
    status: String,
    scanning: bool,
    show_settings: bool,
    result: Option<(Mercenary, ScoreResult)>,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SupportChoice {
    skill_index: usize,
    support_index: usize,
    canonical: String,
}

/// Win32 HWND of the overlay window, for raw SetWindowPos topmost control.
/// eframe's own with_always_on_top()/WindowLevel keeps glow in a continuous
/// redraw+present loop on Windows 11 (the overlay flicker), so we bypass it.
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

impl PricerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cfg: AppConfig,
        pending_image: Option<PathBuf>,
    ) -> Self {
        apply_theme(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();
        let (hotkeys, scan_hotkey) = register_hotkey(&cfg.hotkey);
        install_hotkey_handler(scan_hotkey, tx.clone(), cc.egui_ctx.clone());
        let applied_always_on_top = cfg.always_on_top;
        let hwnd = window_hwnd(cc);
        if cfg.always_on_top {
            if let Some(h) = hwnd {
                set_topmost(h, true);
            }
        }
        let mut app = Self {
            status: format!("Press {} to scan a mercenary panel", cfg.hotkey),
            cfg,
            scanning: false,
            show_settings: false,
            result: None,
            error: None,
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
        };
        if let Some(path) = app.pending_image.take() {
            app.start_scan(ScanRequest::Image(path));
        }
        app
    }

    pub fn with_fixture(cc: &eframe::CreationContext<'_>, cfg: AppConfig) -> Self {
        let mut app = Self::new(cc, cfg, None);
        let (merc, result) = kineticist_jackpot_fixture();
        app.apply_result(merc, result);
        app.status = "Fixture: premium Kineticist (no game required)".into();
        app
    }

    fn start_scan(&mut self, request: ScanRequest) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        self.error = None;
        self.status = match &request {
            ScanRequest::Clipboard => "Reading clipboard…",
            ScanRequest::Image(_) => "Scanning image…",
            ScanRequest::Hotkey | ScanRequest::Screen => "Capturing Path of Exile…",
        }
        .into();
        let cfg = self.cfg.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        thread::spawn(move || {
            let started = Instant::now();
            let msg = match run_scan(&cfg, request) {
                Ok((m, r)) => ScanMsg::Ok(Box::new((m, r, started.elapsed()))),
                Err(e) => ScanMsg::Err(format!("{e:#}")),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    fn apply_result(&mut self, merc: Mercenary, result: ScoreResult) {
        self.status = "Scan complete".into();
        self.result = Some((merc, result));
    }

    fn copy_summary(&self) {
        let Some((merc, result)) = &self.result else {
            return;
        };
        let mut s = format!(
            "{} Lvl {}  score {:.0} [{}{}]\n{}\n",
            merc.class_name,
            merc.level.unwrap_or(0),
            result.score,
            result.band,
            if result.estimate { " est" } else { "" },
            result.action
        );
        for sk in &merc.skills {
            let gems: Vec<_> = sk
                .supports
                .iter()
                .map(|g| {
                    if matches!(g.canonical.as_str(), "unknown" | "ambiguous")
                        || g.canonical.is_empty()
                    {
                        format!("{} T{}", g.name, g.tier as u8)
                    } else {
                        format!(
                            "{} T{}",
                            support_display_for_tier(&g.canonical, g.tier as u8),
                            g.tier as u8
                        )
                    }
                })
                .collect();
            let level = sk
                .level
                .map(|value| format!(" Lv {value}"))
                .unwrap_or_default();
            s.push_str(&format!(
                "  {}{}: {}\n",
                sk.canonical,
                level,
                gems.join(", ")
            ));
        }
        if !result.bricks.is_empty() {
            s.push_str(&format!("Bricks: {}\n", result.bricks.join(", ")));
        }
        if let Ok(mut clip) = arboard::Clipboard::new() {
            let _ = clip.set_text(s);
        }
    }

    fn open_official_trade(&mut self) {
        if self.scanning {
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
        let Some((merc, _)) = &self.result else {
            return;
        };
        let search = match poemercpricer::trade::trade_search(merc, &self.cfg.trade_league) {
            Ok(search) => search,
            Err(error) => {
                self.error = Some(error.to_string());
                self.status = format!("Could not build trade search: {error}");
                return;
            }
        };
        if let Err(error) = open::that(&search.url) {
            self.error = Some(error.to_string());
            self.status = format!("Could not open Path of Exile trade: {error}");
            return;
        }
        self.error = None;
        self.last_trade_open = Some(Instant::now());
        self.status = format!(
            "Opened {} trade: {} with {} exact filters from {} scanned (1 of {} skills)",
            self.cfg.trade_league,
            search.selected_skill,
            search.included_filters,
            search.available_filters,
            search.available_skills
        );
    }
}

impl eframe::App for PricerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(always_on_top) =
            take_always_on_top_change(&mut self.applied_always_on_top, self.cfg.always_on_top)
        {
            // Raw SetWindowPos, not ViewportCommand::WindowLevel — the winit
            // path re-enters the topmost redraw loop (see window_hwnd docs).
            if let Some(h) = self.hwnd {
                set_topmost(h, always_on_top);
            }
        }

        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                ScanMsg::Ok(boxed) => {
                    self.scanning = false;
                    let (m, r, elapsed) = *boxed;
                    self.last_scan_duration = Some(elapsed);
                    self.apply_result(m, r);
                }
                ScanMsg::Err(e) => {
                    self.scanning = false;
                    self.last_scan_duration = None;
                    self.error = Some(e.clone());
                    self.status = format!("Scan failed: {e}");
                }
                ScanMsg::HotkeyPressed => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    self.start_scan(ScanRequest::Hotkey);
                }
            }
        }

        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.show_settings {
                self.show_settings = false;
            } else if self.cfg.hide_on_escape {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        egui::TopBottomPanel::top("command_bar")
            .frame(
                Frame::new()
                    .fill(SURFACE)
                    .inner_margin(Margin::symmetric(12, 8))
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(poe_text("PoEMercPricer", 18.0, GOLD).strong());
                    ui.label(RichText::new("3.29").color(SUBTLE).small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("Settings").selected(self.show_settings))
                            .on_hover_text("App and scan settings")
                            .clicked()
                        {
                            self.show_settings = !self.show_settings;
                        }
                        if ui
                            .add_enabled(!self.scanning, egui::Button::new("Clipboard"))
                            .on_hover_text("Scan Warrant text or an image on the clipboard")
                            .clicked()
                        {
                            self.start_scan(ScanRequest::Clipboard);
                        }
                        if ui
                            .add_enabled(
                                !self.scanning,
                                egui::Button::new(RichText::new("Scan").color(BG).strong())
                                    .fill(GOLD.gamma_multiply(0.9)),
                            )
                            .on_hover_text(format!("Capture the game window ({})", self.cfg.hotkey))
                            .clicked()
                        {
                            self.start_scan(ScanRequest::Screen);
                        }
                    });
                });
            });

        if self.show_settings {
            let mut open = self.show_settings;
            let response = egui::Window::new("Settings")
                .id(egui::Id::new("settings_window"))
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .default_width(360.0)
                .max_width(420.0)
                .show(ctx, |ui| settings_panel(ui, &mut self.cfg));
            self.show_settings = open;
            if let Some(action) = response.and_then(|inner| inner.inner).flatten() {
                match action {
                    SettingsAction::Save => match self.cfg.save() {
                        Ok(_) => {
                            self.error = None;
                            self.status = "Settings saved".into();
                        }
                        Err(error) => {
                            self.error = Some(error.to_string());
                            self.status = format!("Could not save settings: {error}");
                        }
                    },
                    SettingsAction::OpenFolder => {
                        if let Err(error) = open::that(AppConfig::dir()) {
                            self.error = Some(error.to_string());
                            self.status = format!("Could not open config folder: {error}");
                        }
                    }
                }
            }
        }

        if self.result.is_some() {
            egui::TopBottomPanel::bottom("result_actions")
                .frame(
                    Frame::new()
                        .fill(SURFACE)
                        .inner_margin(Margin::symmetric(12, 8))
                        .stroke(Stroke::new(1.0, BORDER)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Copy summary").clicked() {
                            self.copy_summary();
                            self.error = None;
                            self.status = "Summary copied".into();
                        }
                        let trade_open_ready = !self.scanning
                            && self.last_trade_open.map_or(true, |opened| {
                                opened.elapsed() >= TRADE_OPEN_COOLDOWN
                            });
                        if ui
                            .add_enabled(
                                trade_open_ready,
                                egui::Button::new("Search official trade"),
                            )
                            .on_hover_text(
                                "Open current buyout listings for the exact warrant type, level, and most-supported skill",
                            )
                            .clicked()
                        {
                            self.open_official_trade();
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Local processing").color(SUBTLE).small());
                        });
                    });
                });
        }

        let mut support_choice = None;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(Margin::same(12)))
            .show(ctx, |ui| {
                status_line(
                    ui,
                    &self.status,
                    self.error.is_some(),
                    self.scanning,
                    self.last_scan_duration,
                );
                ui.add_space(10.0);
                ui.set_width(ui.available_width());
                if let Some((merc, result)) = &self.result {
                    score_card(ui, merc, result, &mut self.icon_cache, &mut support_choice);
                } else {
                    idle_help(ui, &self.cfg.hotkey);
                }
            });

        if let Some(choice) = support_choice {
            let assume_projectile_speed = self.cfg.assume_projectile_speed;
            if let Some((merc, result)) = &mut self.result {
                if apply_support_choice(merc, &choice) {
                    *result = score_mercenary(merc, assume_projectile_speed);
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

fn idle_help(ui: &mut Ui, hotkey: &str) {
    Frame::new()
        .fill(SURFACE)
        .corner_radius(2)
        .inner_margin(Margin::same(16))
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.label(
                RichText::new("Ready to scan")
                    .color(TEXT)
                    .size(18.0)
                    .strong(),
            );
            ui.label(
                RichText::new("Price-screen a mercenary without leaving the game.").color(MUTED),
            );
            ui.add_space(14.0);
            instruction_row(
                ui,
                "1",
                "Open the mercenary inspect panel or hover a Warrant.",
            );
            instruction_row(ui, "2", &format!("Press {hotkey}, or use Scan above."));
            instruction_row(
                ui,
                "3",
                "Use Clipboard for copied Warrant text, images, or image files.",
            );
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("The scan runs locally. No screenshots are uploaded.").color(MUTED),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Scores prioritize resale screening; verify exceptional rolls on trade.",
                )
                .color(SUBTLE)
                .small(),
            );
        });
}

fn instruction_row(ui: &mut Ui, step: &str, text: &str) {
    ui.horizontal(|ui| {
        badge(ui, step, GOLD);
        ui.add(egui::Label::new(RichText::new(text).color(TEXT)).wrap());
    });
}

fn status_line(
    ui: &mut Ui,
    status: &str,
    is_error: bool,
    scanning: bool,
    elapsed: Option<Duration>,
) {
    ui.horizontal(|ui| {
        let color = if is_error {
            RED
        } else if scanning {
            GOLD
        } else {
            MUTED
        };
        let message = status_message(status, is_error, scanning, elapsed);
        let text = RichText::new(message).color(color).size(13.0);
        let text = if is_error || scanning {
            text.strong()
        } else {
            text
        };
        ui.add(egui::Label::new(text).truncate());
    });
}

fn status_message(
    status: &str,
    is_error: bool,
    scanning: bool,
    elapsed: Option<Duration>,
) -> String {
    if !is_error && !scanning && status == "Scan complete" {
        if let Some(duration) = elapsed {
            return format!("Scan complete in {} ms", duration.as_millis());
        }
    }
    status.to_owned()
}

fn score_card(
    ui: &mut Ui,
    merc: &Mercenary,
    result: &ScoreResult,
    icons: &mut IconCache,
    support_choice: &mut Option<SupportChoice>,
) {
    let mut accent = band_color(&result.band, result.jackpot);
    if result.estimate {
        accent = accent.gamma_multiply(0.8);
    }
    Frame::new()
        .fill(SURFACE_RAISED)
        .corner_radius(2)
        .inner_margin(Margin::same(3))
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            Frame::new()
                .fill(BG)
                .corner_radius(1)
                .inner_margin(Margin::symmetric(14, 10))
                .stroke(Stroke::new(1.0, BORDER_DARK))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if !merc.name.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.label(poe_text(&merc.name, 21.0, GOLD));
                        });
                        ui.add_space(5.0);
                        ui.separator();
                        ui.add_space(5.0);
                    }
                    ui.horizontal(|ui| {
                        ui.label(poe_text(&merc.class_name, 16.0, TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !merc.attributes.is_empty() {
                                ui.label(poe_text(&merc.attributes, 14.0, MUTED));
                            }
                            if let Some(level) = merc.level {
                                ui.label(poe_text(format!("Lvl {level}"), 15.0, TEXT));
                            }
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    Frame::new()
                        .fill(accent.gamma_multiply(0.12))
                        .inner_margin(Margin::symmetric(10, 7))
                        .stroke(Stroke::new(1.0, accent))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(poe_text(verdict_title(result), 19.0, accent).strong());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            poe_text(score_display(result), 16.0, TEXT).strong(),
                                        );
                                    },
                                );
                            });
                            if let Some(detail) = verdict_detail(result) {
                                ui.label(poe_text(detail, 14.0, TEXT).strong());
                            }
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&result.action)
                                        .color(ORANGE)
                                        .size(13.0)
                                        .strong(),
                                )
                                .wrap(),
                            );
                        });
                    if result.estimate {
                        ui.label(
                            RichText::new(
                                "Catalog coverage is complete; market evidence is limited.",
                            )
                            .color(MUTED)
                            .small(),
                        );
                    }
                });
        });

    ui.add_space(8.0);
    if !result.highlights.is_empty() {
        disclosure_section(ui, "why_verdict", "Why this verdict", false, |ui| {
            ui.add_space(3.0);
            Frame::new()
                .fill(SURFACE)
                .inner_margin(Margin::symmetric(10, 7))
                .stroke(Stroke::new(1.0, BORDER_DARK))
                .show(ui, |ui| {
                    for highlight in result.highlights.iter().take(3) {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("•").color(GOLD).strong());
                            ui.add(egui::Label::new(RichText::new(highlight).color(MUTED)).wrap());
                        });
                    }
                });
        });
    }

    ui.add_space(8.0);
    skills_heading(ui, merc.skills.len());
    ui.label(
        RichText::new("Scroll inside this panel to see every skill. Select a skill for supports.")
            .color(MUTED)
            .size(11.5),
    );
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
        ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(1.0, GOLD.gamma_multiply(0.72));
        ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(1.0, GOLD);
        ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(1.0, ORANGE);

        egui::ScrollArea::vertical()
            .id_salt("skills_scroll")
            .max_height(skills_height)
            .auto_shrink([false, false])
            .animated(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                Frame::new()
                    .fill(SURFACE_RAISED)
                    .corner_radius(2)
                    .inner_margin(Margin::same(3))
                    .stroke(Stroke::new(1.0, BORDER))
                    .show(ui, |ui| {
                        Frame::new()
                            .fill(BG)
                            .inner_margin(Margin::same(0))
                            .stroke(Stroke::new(1.0, BORDER_DARK))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                for (index, skill) in merc.skills.iter().enumerate() {
                                    let is_brick = result
                                        .bricks
                                        .iter()
                                        .any(|brick| brick.eq_ignore_ascii_case(&skill.canonical));
                                    skill_row(ui, skill, icons, index, is_brick, support_choice);
                                    if index + 1 < merc.skills.len() {
                                        ui.separator();
                                    }
                                }
                            });
                    });
                ui.add_space(8.0);

                if !result.breakdown.is_empty() && result.band != "unsupported" {
                    disclosure_section(ui, "score_breakdown", "Score breakdown", false, |ui| {
                        ui.add_space(3.0);
                        Frame::new()
                            .fill(SURFACE)
                            .inner_margin(Margin::symmetric(10, 7))
                            .stroke(Stroke::new(1.0, BORDER_DARK))
                            .show(ui, |ui| {
                                egui::Grid::new("score_breakdown_grid")
                                    .num_columns(2)
                                    .striped(true)
                                    .spacing(Vec2::new(12.0, 7.0))
                                    .show(ui, |ui| {
                                        for item in &result.breakdown {
                                            if item.points.abs() < 0.05 {
                                                continue;
                                            }
                                            let color = if item.points > 0.0 { GREEN } else { RED };
                                            ui.label(
                                                RichText::new(format!("{:+.0}", item.points))
                                                    .color(color)
                                                    .strong(),
                                            );
                                            ui.label(RichText::new(&item.label).color(MUTED))
                                                .on_hover_text(&item.detail);
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                }
            });
    });
}

fn disclosure_section(
    ui: &mut Ui,
    id_source: impl std::hash::Hash,
    title: &str,
    default_open: bool,
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
        disclosure_chevron(ui, expanded, true, 22.0);
        ui.label(RichText::new(title).color(MUTED).strong());
    });
    let clicked = ui
        .interact(
            header.response.rect,
            ui.id().with(("section_disclosure", title)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked();
    if clicked {
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
) {
    let row_fill = if is_brick {
        Color32::from_rgb(34, 16, 14)
    } else {
        SURFACE
    };
    Frame::new()
        .fill(row_fill)
        .inner_margin(Margin::symmetric(8, 5))
        .stroke(if is_brick {
            Stroke::new(1.0, RED)
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
            let clicked = skill_header(ui, skill, icons, is_brick, expandable, state.is_open());
            if expandable && clicked {
                state.toggle(ui);
                state.store(ui.ctx());
                ui.ctx().request_repaint();
            }
            if expandable && state.is_open() {
                ui.add_space(5.0);
                support_grid(ui, skill, icons, index, support_choice);
            }
        });
}

fn skill_header(
    ui: &mut Ui,
    skill: &poemercpricer::Skill,
    icons: &mut IconCache,
    is_brick: bool,
    expandable: bool,
    expanded: bool,
) -> bool {
    let row = ui.horizontal(|ui| {
        ui.set_min_height(42.0);
        ui.spacing_mut().item_spacing.x = 8.0;
        disclosure_chevron(ui, expanded, expandable, 42.0);
        ui.allocate_ui_with_layout(
            Vec2::splat(34.0),
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                if let Some(texture) = icons.skill(ui.ctx(), &skill.canonical) {
                    Frame::new()
                        .fill(BG)
                        .inner_margin(Margin::same(1))
                        .stroke(Stroke::new(1.0, BORDER))
                        .show(ui, |ui| {
                            ui.image((texture.id(), Vec2::splat(30.0)));
                        });
                }
            },
        );
        ui.label(poe_text(&skill.canonical, 15.0, TEXT));
        if let Some(level) = skill.level {
            ui.label(poe_text(format!("Lvl {level}"), 12.0, MUTED));
        }
        if is_brick {
            badge(ui, "BRICK", RED);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            support_summary_ui(ui, skill);
        });
    });
    if expandable {
        ui.interact(
            row.response.rect,
            ui.id().with(("skill_header_click", &skill.canonical)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    } else {
        false
    }
}

fn disclosure_chevron(ui: &mut Ui, expanded: bool, visible: bool, slot_height: f32) {
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
        .add(egui::Shape::line(points, Stroke::new(1.5, GOLD)));
}

fn support_summary_ui(ui: &mut Ui, skill: &poemercpricer::Skill) {
    if skill.supports.is_empty() {
        ui.label(RichText::new("No supports").color(MUTED).size(12.0));
        return;
    }

    let tiers = support_tier_summary(skill);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{} supports", skill.supports.len()))
                    .color(TEXT)
                    .size(12.5)
                    .strong(),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(tiers).color(GOLD).size(12.0).strong());
        });
    });
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

fn support_grid(
    ui: &mut Ui,
    skill: &poemercpricer::Skill,
    icons: &mut IconCache,
    skill_index: usize,
    support_choice: &mut Option<SupportChoice>,
) {
    let column_count = if ui.available_width() >= 560.0 { 2 } else { 1 };
    ui.columns(column_count, |columns| {
        for (index, gem) in skill.supports.iter().enumerate() {
            support_detail(
                &mut columns[index % column_count],
                gem,
                icons,
                skill_index,
                index,
                support_choice,
            );
            columns[index % column_count].add_space(4.0);
        }
    });
}

fn support_detail(
    ui: &mut Ui,
    gem: &poemercpricer::SupportGem,
    icons: &mut IconCache,
    skill_index: usize,
    support_index: usize,
    support_choice: &mut Option<SupportChoice>,
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
    );
}

fn support_detail_named(
    ui: &mut Ui,
    name: &str,
    gem: &poemercpricer::SupportGem,
    icons: &mut IconCache,
    skill_index: usize,
    support_index: usize,
    support_choice: &mut Option<SupportChoice>,
) {
    let ambiguous = gem.canonical == "ambiguous";
    Frame::new()
        .fill(SURFACE_RAISED)
        .corner_radius(1)
        .inner_margin(Margin::symmetric(4, 2))
        .stroke(Stroke::new(1.0, if ambiguous { ORANGE } else { BORDER }))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(texture) = icons.support(ui.ctx(), &gem.canonical) {
                    ui.image((texture.id(), Vec2::splat(22.0)));
                }
                ui.label(
                    RichText::new(name)
                        .color(if ambiguous { ORANGE } else { MUTED })
                        .size(12.0),
                )
                .on_hover_text(format!("OCR confidence: {:.0}%", gem.confidence * 100.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(poe_text(format!("T{}", gem.tier as u8), 12.0, GOLD).strong());
                });
            });
            if ambiguous {
                ui.add_space(3.0);
                ui.label(
                    RichText::new("Shared icon: select the support shown in game")
                        .color(MUTED)
                        .size(11.5),
                );
                ui.horizontal_wrapped(|ui| {
                    for (canonical, display) in ambiguous_support_choices(gem) {
                        if ui
                            .add(
                                egui::Button::new(RichText::new(display).color(TEXT))
                                    .fill(SURFACE)
                                    .stroke(Stroke::new(1.0, GOLD.gamma_multiply(0.65))),
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
                        "Tip: hovering the gem and scanning again can identify it automatically.",
                    )
                    .color(MUTED)
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

fn badge(ui: &mut Ui, text: &str, color: Color32) {
    Frame::new()
        .fill(color.gamma_multiply(0.16))
        .corner_radius(2)
        .inner_margin(Margin::symmetric(6, 2))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.65)))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(color).small().strong());
        });
}

fn poe_text(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::new(size, FontFamily::Name("poe_serif".into())))
        .color(color)
}

fn skills_heading(ui: &mut Ui, count: usize) {
    ui.horizontal(|ui| {
        ui.label(poe_text("Mercenary skills", 16.0, GOLD).strong());
        ui.label(poe_text(format!("({count})"), 13.0, MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            scroll_cue(ui);
        });
    });
}

fn scroll_cue(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 16.0), egui::Sense::hover());
        let center = rect.center();
        let stroke = Stroke::new(1.25, GOLD);
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
                .color(MUTED)
                .size(12.0),
        );
    });
}

fn score_display(result: &ScoreResult) -> String {
    if result.band == "unsupported" {
        "Not scored".into()
    } else {
        format!("{:.0} / 100", result.score)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsAction {
    Save,
    OpenFolder,
}

fn settings_panel(ui: &mut Ui, cfg: &mut AppConfig) -> Option<SettingsAction> {
    let mut action = None;
    ui.set_min_width(340.0);
    ui.label(RichText::new("Scanning").color(TEXT).strong());
    ui.add_space(4.0);
    ui.label(RichText::new("Global scan hotkey").color(MUTED).small());
    ui.text_edit_singleline(&mut cfg.hotkey);
    ui.label(
        RichText::new("A hotkey change takes effect after restart.")
            .color(SUBTLE)
            .small(),
    );
    ui.checkbox(
        &mut cfg.scan_clipboard_first,
        "Read copied Warrant text before screen capture",
    );
    ui.checkbox(
        &mut cfg.assume_projectile_speed,
        "Assume 150%+ projectile speed for Frost Blades",
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(10.0);
    ui.label(RichText::new("Window").color(TEXT).strong());
    ui.add_space(4.0);
    ui.checkbox(&mut cfg.always_on_top, "Always on top");
    ui.checkbox(
        &mut cfg.hide_on_escape,
        "Hide the window when Escape is pressed",
    );
    ui.add_space(8.0);
    egui::CollapsingHeader::new(RichText::new("Advanced").color(MUTED))
        .id_salt("advanced_settings")
        .show(ui, |ui| {
            ui.checkbox(&mut cfg.dump_debug, "Save the last capture for diagnostics");
            ui.label(
                RichText::new("Path of Exile window title")
                    .color(MUTED)
                    .small(),
            );
            ui.text_edit_singleline(&mut cfg.poe_window_title);
            ui.label(RichText::new("Trade league").color(MUTED).small());
            ui.text_edit_singleline(&mut cfg.trade_league);
        });
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(RichText::new("Save settings").color(BG).strong())
                    .fill(GOLD.gamma_multiply(0.9)),
            )
            .clicked()
        {
            action = Some(SettingsAction::Save);
        }
        if ui.button("Open config folder").clicked() {
            action = Some(SettingsAction::OpenFolder);
        }
    });
    ui.add_space(10.0);
    ui.label(
        RichText::new("Scores are deterministic screening heuristics, not sale-price quotes.")
            .color(SUBTLE)
            .small(),
    );
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Path of Exile and its game artwork are owned by or licensed to Grinding Gear Games Limited.",
        )
        .color(SUBTLE)
        .small(),
    );
    ui.label(
        RichText::new(
            "This product isn't affiliated with or endorsed by Grinding Gear Games in any way.",
        )
        .color(SUBTLE)
        .small(),
    );
    action
}

fn band_color(band: &str, jackpot: bool) -> Color32 {
    if jackpot || band == "jackpot" {
        return Color32::from_rgb(201, 162, 39);
    }
    match band {
        "skip" => Color32::from_rgb(138, 58, 58),
        "common" => Color32::from_rgb(138, 106, 42),
        "check" => Color32::from_rgb(122, 122, 42),
        "good" => Color32::from_rgb(47, 107, 58),
        "very-valuable" => Color32::from_rgb(31, 107, 90),
        "jackpot-band" => Color32::from_rgb(107, 74, 18),
        "unsupported" => Color32::from_rgb(58, 53, 48),
        _ => Color32::from_rgb(58, 53, 48),
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let serif_family = FontFamily::Name("poe_serif".into());
    let mut serif_fonts = Vec::new();
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\georgia.ttf") {
        fonts
            .font_data
            .insert("poe_serif".into(), egui::FontData::from_owned(bytes).into());
        serif_fonts.push("poe_serif".into());
    }
    if let Some(fallbacks) = fonts.families.get(&FontFamily::Proportional) {
        serif_fonts.extend(fallbacks.iter().cloned());
    }
    fonts.families.insert(serif_family, serif_fonts);
    ctx.set_fonts(fonts);

    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = SURFACE;
    visuals.faint_bg_color = SURFACE_RAISED;
    visuals.override_text_color = Some(TEXT);
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.bg_fill = SURFACE_RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(53, 42, 27);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, GOLD);
    visuals.widgets.active.bg_fill = Color32::from_rgb(68, 44, 24);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, GOLD);
    visuals.widgets.open.bg_fill = SURFACE_RAISED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, GOLD.gamma_multiply(0.7));
    visuals.selection.bg_fill = GOLD.gamma_multiply(0.45);
    visuals.hyperlink_color = GOLD;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
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

fn register_hotkey(spec: &str) -> (Option<GlobalHotKeyManager>, Option<HotKey>) {
    let Ok(manager) = GlobalHotKeyManager::new() else {
        return (None, None);
    };
    let Ok(hk) = parse_hotkey(spec) else {
        return (Some(manager), None);
    };
    if manager.register(hk).is_err() {
        return (Some(manager), None);
    }
    (Some(manager), Some(hk))
}

fn install_hotkey_handler(hotkey: Option<HotKey>, tx: Sender<ScanMsg>, ctx: egui::Context) {
    let hotkey_id = hotkey.map(|value| value.id());
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if is_scan_hotkey_press(hotkey_id, event.id, event.state) {
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
                code = Some(letter_code(c));
            }
            other => anyhow::bail!("unknown hotkey token: {other}"),
        }
    }
    let code = code.ok_or_else(|| anyhow::anyhow!("hotkey missing a key"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn letter_code(c: char) -> Code {
    match c.to_ascii_uppercase() {
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
        _ => Code::KeyM,
    }
}

fn run_scan(cfg: &AppConfig, request: ScanRequest) -> Result<(Mercenary, ScoreResult)> {
    let tries_clipboard = should_try_clipboard(&request, cfg.scan_clipboard_first);
    if tries_clipboard {
        if let Some(merc) = warrant_from_clipboard() {
            let result = score_mercenary(&merc, cfg.assume_projectile_speed);
            return Ok((merc, result));
        }
    }

    let img = match request {
        ScanRequest::Image(path) => capture::load_image(&path)?,
        ScanRequest::Clipboard => capture::clipboard_image_rgba()?.ok_or_else(|| {
            anyhow::anyhow!("Clipboard does not contain Warrant text or an image")
        })?,
        ScanRequest::Hotkey | ScanRequest::Screen => {
            capture::capture_poe_or_primary(&cfg.poe_window_title)?
        }
    };
    if cfg.dump_debug {
        let path = AppConfig::dir().join("debug").join("last-capture.png");
        let _ = capture::save_debug(&img, &path);
    }
    let merc = poemercpricer::scan::scan_rgba(&img)?;
    let result = score_mercenary(&merc, cfg.assume_projectile_speed);
    Ok((merc, result))
}

fn should_try_clipboard(request: &ScanRequest, clipboard_first: bool) -> bool {
    matches!(request, ScanRequest::Clipboard)
        || (matches!(request, ScanRequest::Hotkey) && clipboard_first)
}

fn warrant_from_clipboard() -> Option<Mercenary> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let text = clipboard.get_text().ok()?;
    looks_like_warrant(&text).then(|| parse_warrant_text(&text))
}

#[cfg(test)]
mod tests {
    use global_hotkey::HotKeyState;
    use poemercpricer::models::{Mercenary, ScoreResult, Skill, SupportGem, SupportTier};

    use super::{
        ambiguous_support_choices, apply_support_choice, is_scan_hotkey_press, score_display,
        should_try_clipboard, status_message, take_always_on_top_change, verdict_detail,
        verdict_title, ScanRequest, SupportChoice, BG, GOLD, MUTED, SUBTLE, SURFACE,
        SURFACE_RAISED, TEXT,
    };

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
                "Scan complete",
                false,
                false,
                Some(std::time::Duration::from_millis(131))
            ),
            "Scan complete in 131 ms"
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
    fn every_text_tone_meets_normal_text_contrast_on_every_surface() {
        for foreground in [TEXT, MUTED, SUBTLE, GOLD] {
            for background in [BG, SURFACE, SURFACE_RAISED] {
                let ratio = contrast_ratio(foreground, background);
                assert!(ratio >= 4.5, "text contrast {ratio:.2}:1 is below 4.5:1");
            }
        }
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
        assert_eq!(score_display(&skip), "0 / 100");

        let unsupported = ScoreResult::default();
        assert_eq!(verdict_title(&unsupported), "NO RELIABLE PRICE VERDICT");
        assert_eq!(score_display(&unsupported), "Not scored");

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
}
