#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod capture;

use std::path::PathBuf;

use anyhow::Result;
use eframe::egui;

use crate::app::PricerApp;
use poemercpricer::config::AppConfig;
use poemercpricer::models::{Mercenary, Skill, SupportTier};
use poemercpricer::parse::parse_warrant_text;
use poemercpricer::scoring::score_mercenary;

/// The overlay is a `windows` subsystem exe so it never drags a console around,
/// but the CLI subcommands must still print where they were typed. Rust's stdio
/// fetches the handle per write, so `println!` after this reaches that console.
#[cfg(all(windows, not(debug_assertions)))]
fn attach_parent_console() {
    use windows::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };

    // Stdout already redirected to a pipe or file: nothing to attach.
    if let Ok(handle) = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) } {
        if !handle.is_invalid() {
            return;
        }
    }
    // Fails when launched from Explorer, where there is no parent console and
    // output goes nowhere. That is fine.
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}

fn main() -> Result<()> {
    #[cfg(all(windows, not(debug_assertions)))]
    attach_parent_console();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(|s| s.as_str()) == Some("--help")
        || args.first().map(|s| s.as_str()) == Some("-h")
    {
        print_help();
        return Ok(());
    }
    if args.first().map(|s| s.as_str()) == Some("--version")
        || args.first().map(|s| s.as_str()) == Some("-V")
    {
        println!("poemercpricer {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.first().map(|s| s.as_str()) == Some("score") {
        return cli_score(&args[1..]);
    }
    if args.first().map(|s| s.as_str()) == Some("dump-scan") {
        let path = args
            .get(1)
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("dump-scan <image>"))?;
        let img = crate::capture::load_image(&path)?;
        let merc = poemercpricer::scan::scan_rgba(&img)?;
        let result = score_mercenary(&merc, false);
        print_result(&merc, &result);
        return Ok(());
    }
    if args.first().map(|s| s.as_str()) == Some("dump-trade-query") {
        let path = args
            .get(1)
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("dump-trade-query <image> [league]"))?;
        let img = crate::capture::load_image(&path)?;
        let merc = poemercpricer::scan::scan_rgba(&img)?;
        let league = args.get(2).map(String::as_str).unwrap_or("Allflame");
        let every_skill = args.get(3).is_some_and(|flag| flag == "--every-skill");
        let query = poemercpricer::trade::trade_query(&merc, every_skill)?;
        println!("{}", serde_json::to_string_pretty(&query)?);
        println!(
            "{}",
            poemercpricer::trade::trade_search_url(&merc, league, every_skill)?
        );
        return Ok(());
    }
    if args.first().map(|s| s.as_str()) == Some("dump-clipboard-scan") {
        let img = crate::capture::clipboard_image_rgba()?
            .ok_or_else(|| anyhow::anyhow!("clipboard does not contain an image"))?;
        if let Some(path) = args.get(1) {
            crate::capture::save_debug(&img, &PathBuf::from(path))?;
        }
        let merc = poemercpricer::scan::scan_rgba(&img)?;
        let result = score_mercenary(&merc, false);
        print_result(&merc, &result);
        return Ok(());
    }
    if args.first().map(|s| s.as_str()) == Some("dump-window-scan") {
        // Diagnostics must not create or rewrite config.json as a side effect.
        let cfg: AppConfig = std::fs::read_to_string(AppConfig::path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let img = crate::capture::capture_poe_or_primary(&cfg.poe_window_title)?;
        if let Some(path) = args.get(1) {
            crate::capture::save_debug(&img, &PathBuf::from(path))?;
        }
        let merc = poemercpricer::scan::scan_rgba(&img)?;
        let result = score_mercenary(&merc, false);
        print_result(&merc, &result);
        return Ok(());
    }
    if args.first().map(|s| s.as_str()) == Some("clipboard") {
        let text = if let Some(path) = args.get(1) {
            std::fs::read_to_string(path)?
        } else {
            std::io::read_to_string(std::io::stdin())?
        };
        let merc = parse_warrant_text(&text);
        let result = score_mercenary(&merc, false);
        print_result(&merc, &result);
        return Ok(());
    }

    let args = parse_args(&args)?;

    poemercpricer::installation::mark_running()?;

    let (cfg, cfg_error) = AppConfig::load();
    // Topmost is applied via raw Win32 SetWindowPos in PricerApp::new, not
    // with_always_on_top(): eframe/winit topmost keeps glow in a continuous
    // ~60Hz redraw+present loop on Windows 11 (visible flicker over the game).
    // wgpu renderer for the same reason: glow presents flickered on mouse move.
    let viewport = egui::ViewportBuilder::default()
        .with_title("PoEMercPricer")
        .with_icon(app_icon())
        .with_inner_size([560.0, 780.0])
        .with_min_inner_size([440.0, 560.0]);
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: wgpu_options(),
        ..Default::default()
    };

    eframe::run_native(
        "PoEMercPricer",
        options,
        Box::new(move |cc| {
            if args.fixture {
                Ok(Box::new(PricerApp::with_fixture(
                    cc,
                    cfg,
                    cfg_error,
                    args.no_updates,
                )))
            } else {
                Ok(Box::new(PricerApp::new(
                    cc,
                    cfg,
                    cfg_error,
                    args.scan_image,
                    args.no_updates,
                )))
            }
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

/// eframe's default wgpu setup creates a Vulkan instance AND an OpenGL (WGL)
/// context it never uses, and under `MemoryHints::Performance` wgpu-hal's
/// Vulkan allocator maps a 128 MB host-visible staging chunk on the first
/// upload; `Manual { 2..64 MiB }` shrinks the first chunks further (81 -> 75 MB
/// private working set). DX12 is not compiled in (eframe disables wgpu's default features),
/// so Vulkan is the backend in use either way. GL stays the fallback only on
/// machines with no Vulkan adapter, and WGPU_BACKEND still overrides
/// everything for debugging. Numbers: docs/performance.md.
fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu;

    let backends = wgpu::Backends::from_env().unwrap_or_else(|| {
        let probe = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        if probe.enumerate_adapters(wgpu::Backends::VULKAN).is_empty() {
            wgpu::Backends::VULKAN | wgpu::Backends::GL
        } else {
            wgpu::Backends::VULKAN
        }
    });
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(WgpuSetupCreateNew {
            instance_descriptor: wgpu::InstanceDescriptor {
                backends,
                flags: wgpu::InstanceFlags::from_build_config().with_env(),
                backend_options: wgpu::BackendOptions::from_env_or_default(),
            },
            // Mirrors egui-wgpu 0.31's default device descriptor except for
            // memory_hints, so limits and features are unchanged.
            device_descriptor: std::sync::Arc::new(|adapter| {
                let base_limits = if adapter.get_info().backend == wgpu::Backend::Gl {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                };
                wgpu::DeviceDescriptor {
                    label: Some("egui wgpu device"),
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits {
                        max_texture_dimension_2d: 8192,
                        ..base_limits
                    },
                    memory_hints: wgpu::MemoryHints::Manual {
                        suballocated_device_memory_block_size: (2 << 20)..(64 << 20),
                    },
                }
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn app_icon() -> egui::IconData {
    let pixels =
        image::load_from_memory(include_bytes!("../assets/branding/icons/app-icon-256.png"))
            .expect("embedded app icon must be a valid PNG")
            .into_rgba8();
    let (width, height) = pixels.dimensions();
    egui::IconData {
        rgba: pixels.into_raw(),
        width,
        height,
    }
}

#[derive(Debug)]
struct Args {
    fixture: bool,
    scan_image: Option<PathBuf>,
    no_updates: bool,
}

fn parse_args(args: &[String]) -> Result<Args> {
    let mut fixture = false;
    let mut scan_image: Option<PathBuf> = None;
    let mut no_updates = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fixture" => fixture = true,
            "--no-updates" => no_updates = true,
            "--scan" => {
                i += 1;
                let path = args
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("--scan needs an image path"))?;
                scan_image = Some(PathBuf::from(path));
            }
            other if !other.starts_with('-') && scan_image.is_none() => {
                scan_image = Some(PathBuf::from(other));
            }
            other => anyhow::bail!("unknown argument {other:?}; see --help"),
        }
        i += 1;
    }
    Ok(Args {
        fixture,
        scan_image,
        no_updates,
    })
}

fn print_help() {
    eprintln!(
        "PoEMercPricer: Path of Exile 3.29 Mercenary Warrant screener

USAGE:
  poemercpricer                  Start the overlay (Ctrl+Shift+M)
  poemercpricer --fixture        Open with a canned Kineticist jackpot
  poemercpricer --scan FILE      Scan a screenshot of the merc panel
  poemercpricer --version        Print the version
  poemercpricer --no-updates     Start without checking GitHub for a newer release
  poemercpricer dump-scan FILE   Scan a screenshot and print the result to stdout
  poemercpricer dump-trade-query FILE [LEAGUE] [--every-skill]  Print the offline official trade query
  poemercpricer dump-clipboard-scan [FILE]  Diagnose clipboard image; optionally save it
  poemercpricer dump-window-scan [FILE]     Diagnose the current PoE window capture
  poemercpricer clipboard        Parse a pasted Warrant from stdin
  poemercpricer score --family kineticist --skill 'Kinetic Blast of Clustering:returnT3,gmpT3'

PoE should be Windowed Fullscreen. If the game is elevated, run this app elevated too."
    );
}

fn cli_score(args: &[String]) -> Result<()> {
    let mut family = "kineticist".to_string();
    let mut skills: Vec<Skill> = Vec::new();
    let mut proj = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--family" => {
                i += 1;
                family = args.get(i).cloned().unwrap_or(family);
            }
            "--skill" => {
                i += 1;
                if let Some(raw) = args.get(i) {
                    skills.push(parse_skill_flag(raw));
                }
            }
            "--proj-speed" => proj = true,
            _ => {}
        }
        i += 1;
    }
    let merc = Mercenary {
        class_name: title(&family),
        family,
        skills,
        ..Default::default()
    };
    let result = score_mercenary(&merc, proj);
    print_result(&merc, &result);
    Ok(())
}

fn parse_skill_flag(raw: &str) -> Skill {
    if let Some((name, rest)) = raw.split_once(':') {
        let mut skill = Skill::new(name.trim(), name.trim());
        let re = regex::Regex::new(r"(?i)^([a-z_]+?)(?:t([123]))?$").unwrap();
        for token in rest.split(',') {
            let token = token.trim().replace(' ', "");
            if token.is_empty() {
                continue;
            }
            if let Some(c) = re.captures(&token) {
                let canon = c
                    .get(1)
                    .unwrap()
                    .as_str()
                    .trim_end_matches('_')
                    .to_lowercase();
                let tier = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(2);
                skill.supports.push(poemercpricer::SupportGem {
                    name: canon.clone(),
                    canonical: canon,
                    tier: SupportTier::from_u8(tier),
                    confidence: 1.0,
                    raw: token,
                });
            }
        }
        skill
    } else {
        Skill::new(raw.trim(), raw.trim())
    }
}

fn print_result(merc: &Mercenary, result: &poemercpricer::ScoreResult) {
    let market = poemercpricer::pricing::estimate_for(merc, !result.bricks.is_empty());
    print!(
        "{}",
        poemercpricer::summary::summary(merc, result, market.as_ref())
    );
}

fn title(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, parse_skill_flag};
    use poemercpricer::SupportTier;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_args_sets_no_updates() {
        let parsed = parse_args(&args(&["--no-updates"])).unwrap();
        assert!(parsed.no_updates);
        assert!(!parsed.fixture);
        assert_eq!(parsed.scan_image, None);
    }

    #[test]
    fn parse_args_sets_fixture_and_no_updates_together() {
        let parsed = parse_args(&args(&["--fixture", "--no-updates"])).unwrap();
        assert!(parsed.fixture);
        assert!(parsed.no_updates);
    }

    #[test]
    fn parse_args_rejects_unknown_flag() {
        let err = parse_args(&args(&["--bogus"])).unwrap_err();
        assert!(err.to_string().contains("unknown argument"), "{err}");
    }

    #[test]
    fn parse_args_scan_without_path_errors() {
        let err = parse_args(&args(&["--scan"])).unwrap_err();
        assert!(err.to_string().contains("needs an image path"), "{err}");
    }

    #[test]
    fn parse_args_bare_positional_sets_scan_image() {
        let parsed = parse_args(&args(&["x.png"])).unwrap();
        assert_eq!(parsed.scan_image, Some(std::path::PathBuf::from("x.png")));
    }

    #[test]
    fn skill_flag_keeps_a_trailing_t_without_a_tier() {
        let skill = parse_skill_flag("fb:bolt,castT1,gmpT3,chain");
        let gems: Vec<_> = skill
            .supports
            .iter()
            .map(|g| (g.canonical.as_str(), g.tier))
            .collect();
        assert_eq!(
            gems,
            vec![
                ("bolt", SupportTier::T2),
                ("cast", SupportTier::T1),
                ("gmp", SupportTier::T3),
                ("chain", SupportTier::T2),
            ]
        );
    }
}
