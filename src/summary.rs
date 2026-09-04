//! Plain-text scan summary shared by the CLI output and the overlay's
//! "Copy summary" button.

use crate::catalog::support_display_for_tier;
use crate::models::{Mercenary, ScoreResult};
use crate::pricing::{self, MarketEstimate};

pub fn summary(merc: &Mercenary, result: &ScoreResult, market: Option<&MarketEstimate>) -> String {
    let mut s = format!(
        "{}  Lvl {}  {}\nScore {:.1}  [{}{}]  jackpot={}\n{}\n{}\n",
        merc.class_name,
        merc.level
            .map(|n| n.to_string())
            .unwrap_or_else(|| "?".into()),
        merc.name,
        result.score,
        result.band,
        if result.estimate { " est" } else { "" },
        result.jackpot,
        pricing::summary_line(market),
        result.action
    );
    for sk in &merc.skills {
        let gems: Vec<_> = sk
            .supports
            .iter()
            .map(|g| {
                let n = if matches!(g.canonical.as_str(), "unknown" | "ambiguous")
                    || g.canonical.is_empty()
                {
                    g.name.clone()
                } else {
                    support_display_for_tier(&g.canonical, g.tier as u8)
                };
                format!("{n} T{}", g.tier as u8)
            })
            .collect();
        let gems = if gems.is_empty() {
            "-".into()
        } else {
            gems.join(", ")
        };
        let level = sk.level.map(|n| format!(" Lv{n}")).unwrap_or_default();
        s.push_str(&format!("  {}{level}: {gems}\n", sk.canonical));
    }
    if !result.bricks.is_empty() {
        s.push_str(&format!("Bricks: {}\n", result.bricks.join(", ")));
    }
    for h in &result.highlights {
        s.push_str(&format!("- {h}\n"));
    }
    s
}
