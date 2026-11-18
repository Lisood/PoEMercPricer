//! Generic tier+demand estimate for catalog families without a 3.30 market
//! screen, plus the shared result builder. Stormhand moved to
//! `market::SCREENS` in 3.30: its jackpot no longer holds (docs/market-3.30.md).

use crate::models::{interpret_score, Mercenary, ScoreBreakdown, ScoreResult};

/// Data-driven estimate for families without an audited money screen.
/// There's no known brick list for these yet, so bricks stay empty and low
/// support tiers just score low. Add a dedicated screen once a family gets
/// market-audited. `family` is already lowercased by the caller.
pub fn score_generic(merc: &Mercenary, family: &str, listings: u64) -> ScoreResult {
    let resolved: Vec<_> = merc
        .skills
        .iter()
        .flat_map(|s| &s.supports)
        .filter(|g| crate::catalog::support_icon(&g.canonical).is_some())
        .collect();
    let quality: f32 = resolved.iter().map(|g| g.tier.factor()).sum();
    let support_count = resolved.len();
    let base = if merc.skills.is_empty() { 0.0 } else { 20.0 };
    let support_pts = (10.0 * quality).min(40.0);
    let infamous_pts = if merc.infamous { 10.0 } else { 0.0 };
    let demand_pts = 4.0 * (listings.max(1) as f32).log10();
    let parts = vec![
        ScoreBreakdown {
            label: "Recognised build".into(),
            points: base,
            detail: String::new(),
        },
        ScoreBreakdown {
            label: "Support tiers".into(),
            points: support_pts,
            detail: format!("{support_count} resolved support gems"),
        },
        ScoreBreakdown {
            label: "Infamous".into(),
            points: infamous_pts,
            detail: String::new(),
        },
        ScoreBreakdown {
            label: "Market demand".into(),
            points: demand_pts,
            detail: format!("{listings} trade listings"),
        },
    ];
    // Estimates cap at the family's 3.30 tier target (docs/market-3.30.md,
    // rule 6) so a de-screened family never outranks a screened one; 79 keeps
    // the very-valuable / jackpot bands exclusive to the market screens.
    let raw = (base + support_pts + infamous_pts + demand_pts).min(tier_cap(family));
    let mut result = finish(family, raw, false, Vec::new(), parts);
    result.estimate = true;
    result.formula = "generic-estimate".into();
    result.notes = vec![
        "Estimated from support tiers + trade demand; the 3.30 trade survey found no priced money skill for this family."
            .into(),
    ];
    result
}

/// Full-package score target of the family's 3.30 tier (top-50 median ask,
/// docs/market-3.30.md rule 6). Premium and unmeasured families keep 79.
pub(super) fn tier_cap(family: &str) -> f32 {
    match family {
        "withertouch" | "swiftblade" | "smoulderstrike" | "blade_ambusher" | "bladebitter"
        | "warpriest" => 72.0,
        "toxicologist" | "striker" | "shattersword" | "bastion" | "stormhand"
        | "fallen_reverend" | "flamequiver" | "cardinal" | "bladecaster" | "bloodletter"
        | "shock_ambusher" | "flamehand" | "winter_deacon" | "frosthand" | "storming_zealot"
        | "frost_ambusher" | "ripper" | "sanguimancer" => 62.0,
        "earthshaker" | "eruptor" | "reanimator" | "flaming_charlatan" => 52.0,
        _ => 79.0,
    }
}

pub(super) fn finish(
    family: &str,
    raw: f32,
    jackpot: bool,
    bricks: Vec<String>,
    parts: Vec<ScoreBreakdown>,
) -> ScoreResult {
    let score = crate::models::clamp(raw);
    let (mut band, mut action) = interpret_score(score, family);
    if jackpot {
        band = "jackpot";
        action = "Jackpot price-check immediately; search every support exactly";
    }
    ScoreResult {
        family: family.into(),
        score,
        band: band.into(),
        action: action.into(),
        jackpot,
        bricks,
        highlights: Vec::new(),
        breakdown: parts,
        notes: vec![
            "3.30 Covens trade-survey screen (2026-11-18 asks). Not a sale-price formula.".into(),
        ],
        formula: "screening-heuristic".into(),
        estimate: false,
    }
}
