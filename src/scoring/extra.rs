//! Lightweight 3.29 money screen for Stormhand (its jackpot needs a second
//! skill plus a subset of the gates, which `market::Screen` cannot express),
//! plus the generic tier+demand estimate used by every other catalog family.
//! Sniper and Thunderquiver live as `market::SCREENS` rows.

use crate::models::{interpret_score, Mercenary, ScoreBreakdown, ScoreResult};

pub fn score_stormhand(merc: &Mercenary) -> ScoreResult {
    let arc = merc.skill(&["Arc"]);
    let has_arc = arc.is_some();
    let chain = arc.map(|s| s.t(&["chain"])).unwrap_or(0.0);
    let chain_distance = arc.map(|s| s.t(&["gilded_chain_distance"])).unwrap_or(0.0);
    let faster_casting = arc.map(|s| s.t(&["faster_casting"])).unwrap_or(0.0);
    let bls = merc.has_skill(&["Ball Lightning of Static"]);
    let mut score = if has_arc { 40.0 } else { 0.0 };
    let mut parts = vec![ScoreBreakdown {
        label: "Arc".into(),
        points: score,
        detail: "money main".into(),
    }];
    if chain > 0.0 {
        parts.push(ScoreBreakdown {
            label: "Chain on Arc".into(),
            points: 30.0 * chain,
            detail: String::new(),
        });
        score += 30.0 * chain;
    }
    if bls {
        parts.push(ScoreBreakdown {
            label: "Ball Lightning of Static".into(),
            points: 15.0,
            detail: String::new(),
        });
        score += 15.0;
    }
    if chain_distance > 0.0 {
        parts.push(ScoreBreakdown {
            label: "Gilded Chain Distance on Arc".into(),
            points: 20.0 * chain_distance,
            detail: "market sleeper gate".into(),
        });
        score += 20.0 * chain_distance;
    }
    if faster_casting > 0.0 {
        parts.push(ScoreBreakdown {
            label: "Faster Casting on Arc".into(),
            points: 5.0 * faster_casting,
            detail: String::new(),
        });
        score += 5.0 * faster_casting;
    }
    let jackpot = has_arc && bls && chain > 0.0 && chain_distance > 0.0;
    let mut result = finish("stormhand", score, jackpot, Vec::new(), parts);
    if jackpot {
        result.highlights.push(
            "Market sleeper: Arc + Ball Lightning of Static + Chain + Gilded Chain Distance".into(),
        );
        result
            .notes
            .push("Market value is evidenced; combat efficacy reports remain mixed.".into());
    }
    result
}

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
    // Estimates cap at 79 so the very-valuable / jackpot bands stay exclusive
    // to the market-audited screens.
    let raw = (base + support_pts + infamous_pts + demand_pts).min(79.0);
    let mut result = finish(family, raw, false, Vec::new(), parts);
    result.estimate = true;
    result.formula = "generic-estimate".into();
    result.notes = vec![
        "Estimated from support tiers + trade demand; this family has no market-audited money screen."
            .into(),
    ];
    result
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
            "3.29 Allflame money screen (community testing). Not a sale-price formula.".into(),
        ],
        formula: "screening-heuristic".into(),
        estimate: false,
    }
}
