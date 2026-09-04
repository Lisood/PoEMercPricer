use crate::models::{
    clamp, interpret_score, present, Mercenary, ScoreBreakdown, ScoreResult, SupportTier,
};

pub fn score(merc: &Mercenary) -> ScoreResult {
    let ice = merc.skill(&["Ice Shot"]);
    let vaal = merc.skill(&["Vaal Ice Shot"]);
    let mirror = merc.skill(&["Mirror Arrow"]);

    let t_ice_return = ice.map(|s| s.t(&["return"])).unwrap_or(0.0);
    let t_ice_gmp = ice.map(|s| s.t(&["gmp"])).unwrap_or(0.0);
    let coverage_ice = ice
        .map(|s| s.t(&["chain"]).max(s.t(&["pierce"])).max(s.t(&["fork"])))
        .unwrap_or(0.0);
    let damage_ice = ice
        .map(|s| (s.t(&["edwa"]) + s.t(&["hypothermia"])).min(1.5))
        .unwrap_or(0.0);

    let t_vaal_return = vaal.map(|s| s.t(&["return"])).unwrap_or(0.0);
    let t_vaal_cdr = vaal.map(|s| s.t(&["cdr"])).unwrap_or(0.0);
    let vaal_t3_edwa = vaal.is_some_and(|s| s.support_tier(&["edwa"]) == SupportTier::T3);
    let vaal_t3_hypothermia =
        vaal.is_some_and(|s| s.support_tier(&["hypothermia"]) == SupportTier::T3);
    let coverage_vaal = vaal
        .map(|s| s.t(&["chain"]).max(s.t(&["pierce"])).max(s.t(&["fork"])))
        .unwrap_or(0.0);
    let damage_vaal = vaal
        .map(|s| (s.t(&["edwa"]) + s.t(&["hypothermia"])).min(2.0))
        .unwrap_or(0.0);
    let t_mirror_cdr = mirror.map(|s| s.t(&["cdr"])).unwrap_or(0.0);

    let i_ice = present(ice.is_some());
    let i_vaal = present(vaal.is_some());
    let icicle = merc.has_skill(&["Icicle Rain"]);
    let i_icicle = present(icicle);
    let i_forkshot = present(merc.has_skill(&["Frigid Forkshot"]));
    let i_aura = present(merc.has_skill(&["Hatred"]) || merc.has_skill(&["Grace"]));

    let parts = vec![
        ScoreBreakdown {
            label: "Ice Shot".into(),
            points: 18.0 * i_ice,
            detail: "mandatory clear".into(),
        },
        ScoreBreakdown {
            label: "Vaal Ice Shot".into(),
            points: 18.0 * i_vaal,
            detail: "mandatory bossing".into(),
        },
        ScoreBreakdown {
            label: "Return on Vaal Ice Shot".into(),
            points: 18.0 * t_vaal_return,
            detail: format!("t={t_vaal_return:.2}"),
        },
        ScoreBreakdown {
            label: "Return on Ice Shot".into(),
            points: 8.0 * t_ice_return,
            detail: format!("t={t_ice_return:.2}"),
        },
        ScoreBreakdown {
            label: "GMP on Ice Shot".into(),
            points: 10.0 * t_ice_gmp,
            detail: format!("t={t_ice_gmp:.2}"),
        },
        ScoreBreakdown {
            label: "Ice Shot coverage".into(),
            points: 6.0 * coverage_ice,
            detail: "Chain / Pierce / Fork".into(),
        },
        ScoreBreakdown {
            label: "Ice Shot damage".into(),
            points: 4.0 * damage_ice,
            detail: "EDWA + Hypothermia, cap 1.5".into(),
        },
        ScoreBreakdown {
            label: "Vaal Ice Shot damage".into(),
            points: 7.0 * damage_vaal,
            detail: "EDWA + Hypothermia, cap 2".into(),
        },
        ScoreBreakdown {
            label: "CDR on Vaal Ice Shot".into(),
            points: 6.0 * t_vaal_cdr,
            detail: format!("t={t_vaal_cdr:.2}"),
        },
        ScoreBreakdown {
            label: "Vaal Ice Shot coverage".into(),
            points: 5.0 * coverage_vaal,
            detail: String::new(),
        },
        ScoreBreakdown {
            label: "CDR on Mirror Arrow".into(),
            points: 3.0 * t_mirror_cdr,
            detail: String::new(),
        },
        ScoreBreakdown {
            label: "Hatred or Grace".into(),
            points: 2.0 * i_aura,
            detail: String::new(),
        },
        ScoreBreakdown {
            label: "Icicle Rain brick".into(),
            points: -40.0 * i_icicle,
            detail: "hard premium brick".into(),
        },
        ScoreBreakdown {
            label: "Frigid Forkshot".into(),
            points: -4.0 * i_forkshot,
            detail: "soft desirability penalty".into(),
        },
    ];
    let score = clamp(parts.iter().map(|p| p.points).sum());
    let bricks = if icicle {
        vec!["Icicle Rain".into()]
    } else {
        Vec::new()
    };
    let premium_vaal_row =
        mirror.is_some() && t_vaal_return > 0.0 && vaal_t3_edwa && vaal_t3_hypothermia;
    // Ice Shot::Return+GMP beside Vaal Return fell from a 50d floor (09-01)
    // to 10c/80c (n=852) by 2026-09-02, so it does not gate jackpot.
    let jackpot = ice.is_some() && vaal.is_some() && !icicle && premium_vaal_row;

    let mut highlights = Vec::new();
    if ice.is_some() && vaal.is_some() && mirror.is_some() && t_vaal_return > 0.0 && !icicle {
        highlights
            .push("Market core: Vaal Ice Shot + Mirror Arrow + Return on Vaal Ice Shot".into());
    }
    if jackpot {
        highlights.push(
            "Market-proven premium: Vaal Return with T3 EDWA and T3 Hypothermia on Vaal Ice Shot"
                .into(),
        );
    }
    if icicle {
        highlights.push("Icicle Rain present, premium resale largely destroyed".into());
    }

    let (mut band, mut action) = interpret_score(score, "manyshot");
    if jackpot {
        band = "jackpot";
        action = "Jackpot price-check immediately; search every support exactly";
    }

    ScoreResult {
        family: "manyshot".into(),
        score,
        band: band.into(),
        action: action.into(),
        jackpot,
        bricks,
        highlights,
        breakdown: parts,
        notes: vec![
            "Screening heuristic, not a Divine/Mirror calculator.".into(),
            "3.29 Allflame listings validate Mirror Arrow + Vaal Ice Shot with Return as the resale core."
                .into(),
            "GMP does not multiply Vaal Ice Shot Mirage Sharpshooter attacks.".into(),
            "Supports only count on the skill they are attached to.".into(),
        ],
        formula: "screening-heuristic".into(),
        estimate: false,
    }
}
