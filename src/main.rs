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

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(|s| s.as_str()) == Some("--help")
        || args.first().map(|s| s.as_str()) == Some("-h")
    {
        print_help();
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
        let query = poemercpricer::trade::trade_query(&merc)?;
        println!("{}", serde_json::to_string_pretty(&query)?);
        println!("{}", poemercpricer::trade::trade_search_url(&merc, league)?);
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
        let cfg = AppConfig::load();
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

    let mut fixture = false;
    let mut scan_image: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fixture" => fixture = true,
            "--scan" => {
                i += 1;
                scan_image = args.get(i).map(PathBuf::from);
            }
            other if !other.starts_with('-') && scan_image.is_none() => {
                scan_image = Some(PathBuf::from(other));
            }
            _ => {}
        }
        i += 1;
    }

    let cfg = AppConfig::load();
    // Topmost is applied via raw Win32 SetWindowPos in PricerApp::new, NOT
    // with_always_on_top(): eframe/winit topmost keeps glow in a continuous
    // ~60Hz redraw+present loop on Windows 11 (visible flicker over the game).
    // wgpu renderer for the same reason: glow presents flickered on mouse move.
    let viewport = egui::ViewportBuilder::default()
        .with_title("PoEMercPricer")
        .with_inner_size([560.0, 780.0])
        .with_min_inner_size([440.0, 560.0]);
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "PoEMercPricer",
        options,
        Box::new(move |cc| {
            if fixture {
                Ok(Box::new(PricerApp::with_fixture(cc, cfg)))
            } else {
                Ok(Box::new(PricerApp::new(cc, cfg, scan_image)))
            }
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn print_help() {
    eprintln!(
        "PoEMercPricer — Path of Exile 3.29 Mercenary Warrant screener

USAGE:
  poemercpricer                  Start the overlay (Ctrl+Shift+M)
  poemercpricer --fixture        Open with a canned Kineticist jackpot
  poemercpricer --scan FILE      Scan a screenshot of the merc panel
  poemercpricer dump-trade-query FILE [LEAGUE]  Print the offline official trade query
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
        let re = regex::Regex::new(r"(?i)^([a-z_]+?)[t]?([123])?$").unwrap();
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
    println!(
        "{}  Lvl {}  {}",
        merc.class_name,
        merc.level
            .map(|n| n.to_string())
            .unwrap_or_else(|| "?".into()),
        merc.name
    );
    println!(
        "Score {:.1}  [{}{}]  jackpot={}",
        result.score,
        result.band,
        if result.estimate { " est" } else { "" },
        result.jackpot
    );
    println!("{}", result.action);
    for s in &merc.skills {
        let gems: Vec<_> = s
            .supports
            .iter()
            .map(|g| {
                let n = if matches!(g.canonical.as_str(), "unknown" | "ambiguous") {
                    g.name.as_str()
                } else {
                    g.canonical.as_str()
                };
                format!("{n} T{}", g.tier as u8)
            })
            .collect();
        let g = if gems.is_empty() {
            "—".into()
        } else {
            gems.join(", ")
        };
        let lv = s.level.map(|n| format!(" Lv{n}")).unwrap_or_default();
        println!("  {}{lv}: {g}", s.canonical);
    }
    if !result.bricks.is_empty() {
        println!("Bricks: {}", result.bricks.join(", "));
    }
    for h in &result.highlights {
        println!("- {h}");
    }
}

fn title(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
