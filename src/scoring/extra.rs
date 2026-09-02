//! Lightweight 3.29 money screens for Sniper / Thunderquiver / Stormhand,
//! plus the generic tier+demand estimate used by every other catalog family.
//! Full Q-scores exist only for Kineticist / Manyshot / Combatant; the
//! screens are stop/skip filters from current Allflame carry testing.

use crate::models::{interpret_score, Mercenary, ScoreBreakdown, ScoreResult};

pub fn score_sniper(merc: &Mercenary) -> ScoreResult {
    let ts = merc.skill(&["Tornado Shot"]);
    let has_ts = ts.is_some();
    let gmp = ts.map(|s| s.t(&["gmp"])).unwrap_or(0.0);
    let brutality = ts.map(|s| s.has_support(&["brutality"])).unwrap_or(false)
        || merc
            .skills
            .iter()
            .any(|s| s.canonical == "Tornado Shot" && s.has_support(&["brutality"]));
    let arrow_nova = merc
        .skills
        .iter()
        .any(|s| s.has_support(&["arrow_nova"]) || s.canonical.eq_ignore_ascii_case("Arrow Nova"));
    let mut score = 0.0;
    let mut parts = vec![ScoreBreakdown {
        label: "Tornado Shot".into(),
        points: if has_ts { 40.0 } else { 0.0 },
        detail: "money main".into(),
    }];
    score += if has_ts { 40.0 } else { 0.0 };
    if gmp > 0.0 {
        parts.push(ScoreBreakdown {
            label: "GMP on Tornado Shot".into(),
            points: 25.0 * gmp,
            detail: String::new(),
        });
        score += 25.0 * gmp;
    }
    let mut bricks = Vec::new();
    if brutality {
        bricks.push("Brutality on Tornado Shot".into());
        score -= 40.0;
        parts.push(ScoreBreakdown {
            label: "Brutality brick".into(),
            points: -40.0,
            detail: String::new(),
        });
    }
    if arrow_nova {
        bricks.push("Arrow Nova".into());
        score -= 20.0;
    }
    let jackpot = has_ts && gmp > 0.0 && !brutality && !arrow_nova;
    finish("sniper", score, jackpot, bricks, parts)
}

pub fn score_thunderquiver(merc: &Mercenary) -> ScoreResult {
    let la = merc.skill(&["Lightning Arrow"]);
    let has_la = la.is_some();
    let ret = la.map(|s| s.t(&["return"])).unwrap_or(0.0);
    let gmp = la.map(|s| s.t(&["gmp"])).unwrap_or(0.0);
    let galvanic = merc.has_skill(&["Galvanic Arrow"]);
    let mut score = if has_la { 35.0 } else { 0.0 };
    let mut parts = vec![ScoreBreakdown {
        label: "Lightning Arrow".into(),
        points: score,
        detail: "money main".into(),
    }];
    if ret > 0.0 {
        parts.push(ScoreBreakdown {
            label: "Return on LA".into(),
            points: 25.0 * ret,
            detail: String::new(),
        });
        score += 25.0 * ret;
    }
    if gmp > 0.0 {
        parts.push(ScoreBreakdown {
            label: "GMP on LA".into(),
            points: 15.0 * gmp,
            detail: String::new(),
        });
        score += 15.0 * gmp;
    }
    let mut bricks = Vec::new();
    if galvanic {
        bricks.push("Galvanic Arrow".into());
        score -= 25.0;
        parts.push(ScoreBreakdown {
            label: "Galvanic brick".into(),
            points: -25.0,
            detail: String::new(),
        });
    }
    let jackpot = has_la && ret > 0.0 && gmp > 0.0 && !galvanic;
    finish("thunderquiver", score, jackpot, bricks, parts)
}

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
/// ponytail: no generic brick knowledge exists, so bricks stay empty and low
/// support tiers simply score low. Upgrade path: add a dedicated screen when
/// a family gets market-audited.
pub fn score_generic(merc: &Mercenary, listings: u64) -> ScoreResult {
    let quality: f32 = merc
        .skills
        .iter()
        .flat_map(|s| &s.supports)
        .map(|g| g.tier.factor())
        .sum();
    let support_count: usize = merc.skills.iter().map(|s| s.supports.len()).sum();
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
            detail: format!("{support_count} support gems"),
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
    let mut result = finish(&merc.family.to_lowercase(), raw, false, Vec::new(), parts);
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
