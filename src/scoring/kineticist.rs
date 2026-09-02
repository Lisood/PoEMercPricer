use crate::models::{clamp, interpret_score, present, Mercenary, ScoreBreakdown, ScoreResult};

const KBOC: &str = "Kinetic Blast of Clustering";

pub fn score(merc: &Mercenary) -> ScoreResult {
    let kboc = merc.skill(&[KBOC]);
    let mut coverage = 0.0;
    let mut damage = 0.0;
    let mut t_return = 0.0;
    let mut t_gmp = 0.0;
    if let Some(s) = kboc {
        coverage = s.t(&["chain"]).max(s.t(&["pierce"])).max(s.t(&["fork"]));
        damage = (s.t(&["edwa"])
            + s.t(&["faster_attacks"])
            + s.t(&["crit_damage"])
            + s.t(&["sacred_wisps"]))
        .min(2.0);
        t_return = s.t(&["return"]);
        t_gmp = s.t(&["gmp"]);
    }

    let mut bricks = Vec::new();
    if merc.has_skill(&["Kinetic Bolt"]) {
        bricks.push("Kinetic Bolt".into());
    }
    if merc.has_skill(&["Power Siphon"]) {
        bricks.push("Power Siphon".into());
    }
    if merc.has_skill(&["Kinetic Rain of Impact"]) {
        bricks.push("Kinetic Rain of Impact".into());
    }

    let i_kboc = present(kboc.is_some());
    let i_gkb = present(merc.has_skill(&["Greater Kinetic Blast"]));
    let i_barrage = present(merc.has_skill(&["Barrage"]));
    let i_haste = present(merc.has_skill(&["Haste"]));
    let secondary = (8.0 * i_gkb).max(5.0 * i_barrage);

    let parts = vec![
        ScoreBreakdown {
            label: "KBoC".into(),
            points: 25.0 * i_kboc,
            detail: "mandatory main skill".into(),
        },
        ScoreBreakdown {
            label: "Return on KBoC".into(),
            points: 22.0 * t_return,
            detail: format!("t={t_return:.2}"),
        },
        ScoreBreakdown {
            label: "GMP on KBoC".into(),
            points: 14.0 * t_gmp,
            detail: format!("t={t_gmp:.2}"),
        },
        ScoreBreakdown {
            label: "Coverage (Chain/Pierce/Fork)".into(),
            points: 10.0 * coverage,
            detail: format!("t={coverage:.2}"),
        },
        ScoreBreakdown {
            label: "Damage supports".into(),
            points: 7.0 * damage,
            detail: "EDWA / Faster Attacks / Crit Damage / Sacred Wisps".into(),
        },
        ScoreBreakdown {
            label: "Secondary burst".into(),
            points: secondary,
            detail: "Greater Kinetic Blast 8 or Barrage 5".into(),
        },
        ScoreBreakdown {
            label: "Haste".into(),
            points: 4.0 * i_haste,
            detail: String::new(),
        },
        ScoreBreakdown {
            label: "Bricks".into(),
            points: -20.0 * bricks.len() as f32,
            detail: if bricks.is_empty() {
                "none".into()
            } else {
                bricks.join(", ")
            },
        },
    ];
    let score = clamp(parts.iter().map(|p| p.points).sum());
    // The 2026-09-01 Allflame market export validates KBoC + Greater KB +
    // Return + GMP as a premium gate even before another coverage support is
    // required. Coverage and damage supports still improve the quality score.
    let jackpot = i_kboc > 0.0 && i_gkb > 0.0 && t_return > 0.0 && t_gmp > 0.0 && bricks.is_empty();

    let mut highlights = Vec::new();
    if i_kboc > 0.0 && t_return > 0.0 {
        highlights.push("Stop: KBoC + Return".into());
    }
    if jackpot {
        highlights
            .push("Market-proven premium gate: KBoC + Greater KB + Return + GMP, no bricks".into());
    }
    if !bricks.is_empty() {
        highlights.push("Reject premium pricing: competing attack present".into());
    }

    let (mut band, mut action) = interpret_score(score, "kineticist");
    if jackpot {
        band = "jackpot";
        action = "Jackpot price-check immediately; search every support exactly";
    }

    ScoreResult {
        family: "kineticist".into(),
        score,
        band: band.into(),
        action: action.into(),
        jackpot,
        bricks,
        highlights,
        breakdown: parts,
        notes: vec![
            "Screening heuristic, not a Divine/Mirror calculator.".into(),
            "3.29 Allflame listings validate Greater KB + Return, with a major premium for GMP on KBoC."
                .into(),
            "Supports only count when attached to Kinetic Blast of Clustering.".into(),
        ],
        formula: "screening-heuristic".into(),
        estimate: false,
    }
}
