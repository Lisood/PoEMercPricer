use crate::models::{clamp, interpret_score, present, Mercenary, ScoreBreakdown, ScoreResult};

pub fn score(merc: &Mercenary, assume_projectile_speed: bool) -> ScoreResult {
    let fb = merc.skill(&["Frost Blades"]);
    let ws = merc.skill(&["Wild Strike"]);
    let static_s = merc.skill(&["Static Strike"]);
    let helix = merc.has_skill(&["Spectral Helix"]);

    let mut frost_score = 0.0;
    let mut frost_parts = Vec::new();
    if let Some(s) = fb {
        // Return is a proven resale gate even when open-area return geometry is
        // not guaranteed. The setting controls the DPS interpretation, not
        // whether a highly sought-after support disappears from the price screen.
        let return_fb = 8.0 * s.t(&["return"]);
        frost_score = 26.0
            + 14.0 * s.t(&["chain"])
            + 12.0 * s.t(&["edwa"])
            + 10.0 * s.t(&["faster_attacks"])
            + 8.0 * s.t(&["hypothermia"])
            + 6.0 * s.t(&["gmp"])
            + return_fb;
        frost_parts = vec![
            ScoreBreakdown {
                label: "Frost Blades base".into(),
                points: 26.0,
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "FB Chain".into(),
                points: 14.0 * s.t(&["chain"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "FB EDWA".into(),
                points: 12.0 * s.t(&["edwa"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "FB Faster Attacks".into(),
                points: 10.0 * s.t(&["faster_attacks"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "FB Hypothermia".into(),
                points: 8.0 * s.t(&["hypothermia"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "FB GMP".into(),
                points: 6.0 * s.t(&["gmp"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "FB Return".into(),
                points: return_fb,
                detail: if assume_projectile_speed {
                    "DPS enabled: sufficient projectile speed / wall geometry".into()
                } else {
                    "resale signal; open-area returning hits remain conditional".into()
                },
            },
        ];
    }

    let mut wild_score = 0.0;
    let mut wild_parts = Vec::new();
    if let Some(s) = ws {
        wild_score = 26.0
            + 14.0 * s.t(&["edwa"])
            + 10.0 * s.t(&["faster_attacks"])
            + 8.0 * s.t(&["hypothermia"])
            + 5.0 * s.t(&["ele_weakness_on_hit"])
            + 4.0 * s.t(&["chain"]).max(s.t(&["gmp"]));
        wild_parts = vec![
            ScoreBreakdown {
                label: "Wild Strike base".into(),
                points: 26.0,
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "WS EDWA".into(),
                points: 14.0 * s.t(&["edwa"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "WS Faster Attacks".into(),
                points: 10.0 * s.t(&["faster_attacks"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "WS Hypothermia".into(),
                points: 8.0 * s.t(&["hypothermia"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "WS Ele Weakness on Hit".into(),
                points: 5.0 * s.t(&["ele_weakness_on_hit"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "WS coverage".into(),
                points: 4.0 * s.t(&["chain"]).max(s.t(&["gmp"])),
                detail: String::new(),
            },
        ];
    }

    let mut static_score = 0.0;
    let mut static_parts = Vec::new();
    if let Some(s) = static_s {
        static_score = 12.0
            + 7.0 * s.t(&["more_duration"])
            + 4.0 * s.t(&["edwa"])
            + 3.0 * s.t(&["aoe"])
            + 2.0 * s.t(&["chain"]);
        static_parts = vec![
            ScoreBreakdown {
                label: "Static Strike base".into(),
                points: 12.0,
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "SS More Duration".into(),
                points: 7.0 * s.t(&["more_duration"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "SS EDWA".into(),
                points: 4.0 * s.t(&["edwa"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "SS AoE".into(),
                points: 3.0 * s.t(&["aoe"]),
                detail: String::new(),
            },
            ScoreBreakdown {
                label: "SS Chain".into(),
                points: 2.0 * s.t(&["chain"]),
                detail: String::new(),
            },
        ];
    }

    let util = (2.0 * present(merc.has_skill(&["Herald of Ice"]))
        + 2.0 * present(merc.has_skill(&["Dash"]) || merc.has_skill(&["Frostblink"]))
        + present(merc.has_skill(&["Wrath"])))
    .min(4.0);

    let both_mains = fb.is_some() && ws.is_some();
    let helix_with_main = helix && (fb.is_some() || ws.is_some());
    let fb_chain_pierce = fb
        .map(|s| s.has_support(&["chain"]) && s.has_support(&["pierce"]))
        .unwrap_or(false);
    let ws_multistrike = ws.map(|s| s.has_support(&["multistrike"])).unwrap_or(false);

    let penalty = 28.0 * present(both_mains)
        + 20.0 * present(helix_with_main)
        + 12.0 * present(fb_chain_pierce)
        + 8.0 * present(ws_multistrike);

    let mut parts = if frost_score >= wild_score {
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
    parts.push(ScoreBreakdown {
        label: "FB+WS penalty".into(),
        points: -28.0 * present(both_mains),
        detail: String::new(),
    });
    parts.push(ScoreBreakdown {
        label: "Spectral Helix + main".into(),
        points: -20.0 * present(helix_with_main),
        detail: String::new(),
    });
    parts.push(ScoreBreakdown {
        label: "FB Chain+Pierce".into(),
        points: -12.0 * present(fb_chain_pierce),
        detail: String::new(),
    });
    parts.push(ScoreBreakdown {
        label: "WS Multistrike".into(),
        points: -8.0 * present(ws_multistrike),
        detail: String::new(),
    });

    let main_score = frost_score.max(wild_score);
    let score = clamp(main_score + static_score + util - penalty);

    let mut bricks = Vec::new();
    if both_mains {
        bricks.push("Frost Blades + Wild Strike".into());
    }
    if helix_with_main {
        bricks.push("Spectral Helix with a preferred main attack".into());
    }

    let jackpot_fb = fb.is_some()
        && static_s.is_some()
        && ws.is_none()
        && !helix
        && fb.unwrap().has_support(&["return"])
        && fb.unwrap().has_support(&["chain"])
        && fb.unwrap().has_support(&["edwa"]);
    let jackpot_ws = ws.is_some()
        && static_s.is_some()
        && fb.is_none()
        && !helix
        && ws.unwrap().has_support(&["edwa"])
        && ws.unwrap().has_support(&["faster_attacks"])
        && ws.unwrap().has_support(&["hypothermia"]);
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
