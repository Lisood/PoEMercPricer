use crate::models::{
    clamp, interpret_score, present, Mercenary, ScoreBreakdown, ScoreResult, Skill, SupportTier,
};

fn part(label: &str, points: f32) -> ScoreBreakdown {
    ScoreBreakdown {
        label: label.into(),
        points,
        detail: String::new(),
    }
}

fn t3(skill: Option<&Skill>, id: &str) -> bool {
    skill.is_some_and(|s| s.support_tier(&[id]) == SupportTier::T3)
}

pub fn score(merc: &Mercenary, assume_projectile_speed: bool) -> ScoreResult {
    let fb = merc.skill(&["Frost Blades"]);
    let ws = merc.skill(&["Wild Strike"]);
    let static_s = merc.skill(&["Static Strike"]);
    let helix = merc.has_skill(&["Spectral Helix"]);
    let sum = |parts: &[ScoreBreakdown]| parts.iter().map(|p| p.points).sum::<f32>();

    let frost_parts = fb.map_or_else(Vec::new, |s| {
        vec![
            part("Frost Blades base", 26.0),
            part("FB Chain", 14.0 * s.t(&["chain"])),
            part("FB EDWA", 12.0 * s.t(&["edwa"])),
            part("FB Faster Attacks", 10.0 * s.t(&["faster_attacks"])),
            part("FB Hypothermia", 8.0 * s.t(&["hypothermia"])),
            part("FB GMP", 6.0 * s.t(&["gmp"])),
            // Return is a proven resale gate even when open-area return geometry is
            // not guaranteed. The setting controls the DPS interpretation, not
            // whether a highly sought-after support disappears from the price screen.
            ScoreBreakdown {
                label: "FB Return".into(),
                points: 8.0 * s.t(&["return"]),
                detail: if assume_projectile_speed {
                    "DPS enabled: sufficient projectile speed / wall geometry".into()
                } else {
                    "resale signal; open-area returning hits remain conditional".into()
                },
            },
        ]
    });
    let wild_parts = ws.map_or_else(Vec::new, |s| {
        vec![
            part("Wild Strike base", 26.0),
            part("WS EDWA", 14.0 * s.t(&["edwa"])),
            part("WS Faster Attacks", 10.0 * s.t(&["faster_attacks"])),
            part("WS Hypothermia", 8.0 * s.t(&["hypothermia"])),
            part(
                "WS Ele Weakness on Hit",
                5.0 * s.t(&["gilded_elemental_weakness_on_hit"]),
            ),
            part("WS coverage", 4.0 * s.t(&["chain"]).max(s.t(&["gmp"]))),
        ]
    });
    let static_parts = static_s.map_or_else(Vec::new, |s| {
        vec![
            part("Static Strike base", 12.0),
            part("SS More Duration", 7.0 * s.t(&["more_duration"])),
            part("SS EDWA", 4.0 * s.t(&["edwa"])),
            part("SS AoE", 3.0 * s.t(&["aoe"])),
            part("SS Chain", 2.0 * s.t(&["chain"])),
        ]
    });

    let util = (2.0 * present(merc.has_skill(&["Herald of Ice"]))
        + 2.0 * present(merc.has_skill(&["Dash"]) || merc.has_skill(&["Frostblink"]))
        + present(merc.has_skill(&["Wrath"])))
    .min(4.0);

    let both_mains = fb.is_some() && ws.is_some();
    let helix_with_main = helix && (fb.is_some() || ws.is_some());
    let fb_chain_pierce =
        fb.is_some_and(|s| s.has_support(&["chain"]) && s.has_support(&["pierce"]));
    let ws_multistrike = ws.is_some_and(|s| s.has_support(&["multistrike"]));

    let mut parts = if sum(&frost_parts) >= sum(&wild_parts) {
        frost_parts
    } else {
        wild_parts
    };
    parts.extend(static_parts);
    parts.push(ScoreBreakdown {
        label: "Utility (HoI / Dash|Frostblink / Wrath)".into(),
        points: util,
        detail: "capped at 4".into(),
    });
    parts.push(part("FB+WS penalty", -28.0 * present(both_mains)));
    parts.push(part(
        "Spectral Helix + main",
        -20.0 * present(helix_with_main),
    ));
    parts.push(part("FB Chain+Pierce", -12.0 * present(fb_chain_pierce)));
    parts.push(part("WS Multistrike", -8.0 * present(ws_multistrike)));
    let score = clamp(sum(&parts));

    let mut bricks = Vec::new();
    if both_mains {
        bricks.push("Frost Blades + Wild Strike".into());
    }
    if helix_with_main {
        bricks.push("Spectral Helix with a preferred main attack".into());
    }

    // Market gates are Greater (T3) supports; lower tiers still score but never jackpot.
    // Chain has no Tier 3 on Frost Blades (T3 is Gilded Chain Distance, a
    // different gem), so Chain T2 is the top attainable tier and the gate.
    let jackpot_fb = static_s.is_some()
        && ws.is_none()
        && !helix
        && t3(fb, "return")
        && fb.is_some_and(|s| s.support_tier(&["chain"]) >= SupportTier::T2)
        && t3(fb, "edwa");
    let jackpot_ws = static_s.is_some()
        && fb.is_none()
        && !helix
        && t3(ws, "edwa")
        && t3(ws, "faster_attacks")
        && t3(ws, "hypothermia");
    let jackpot = jackpot_fb || jackpot_ws;

    let mut highlights = Vec::new();
    if jackpot_fb {
        highlights.push(
            "Market-proven Frost Blades gate: Return + Chain + EDWA + Static, no WS/Helix".into(),
        );
    }
    if jackpot_ws {
        highlights.push(
            "Jackpot Wild Strike: EDWA + Faster Attacks + Hypothermia + Static, no FB/Helix".into(),
        );
    }
    if let Some(s) = fb {
        if !s.has_support(&["chain"]) {
            highlights.push("Frost Blades wants Chain".into());
        }
        if !assume_projectile_speed && s.has_support(&["return"]) {
            highlights.push(
                "Return is valued for resale; its direct DPS is conditional on projectile speed or walls"
                    .into(),
            );
        }
    }

    let (mut band, mut action) = interpret_score(score, "combatant");
    if jackpot {
        band = "jackpot";
        action = "Jackpot price-check immediately; search every support exactly";
    }

    ScoreResult {
        family: "combatant".into(),
        score,
        band: band.into(),
        action: action.into(),
        jackpot,
        bricks,
        highlights,
        breakdown: parts,
        notes: vec![
            "Screening heuristic, not a Divine/Mirror calculator.".into(),
            "3.29 Allflame listings validate Return + EDWA + Chain on Frost Blades as a high-value gate."
                .into(),
            "Dash and Frostblink are gap closers, not bricks, for melee Combatants.".into(),
            "Multistrike is a resale penalty on Wild Strike; disputed on Frost Blades.".into(),
        ],
        formula: "screening-heuristic".into(),
        estimate: false,
    }
}
