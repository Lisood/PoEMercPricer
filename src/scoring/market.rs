//! Per-family 3.29 market-floor screens for the non-audited families.
//!
//! Derived from the Perandus Ledger asking-price snapshot of 2026-09-01
//! (1.07M listings, xddbsns.com /data/allflame exports, skill-bound support
//! decoding) cross-checked against 3.29 tier lists (aoeah, akrpg, odealo).
//! Floors are asks, not sales. Full research: docs/research-3.29.md.
//!
//! ponytail: gates bind to ONE primary money skill per family; a few families
//! have secondary skill-bound packages (e.g. Shattersword's Lancing Steel +
//! Gilded Scattershot) that are folded into skill-presence bonuses or dropped.
//! Upgrade path: multi-skill gate lists when a resale case demands it.

use crate::models::{Mercenary, ScoreBreakdown, ScoreResult};

pub struct Screen {
    /// Primary money/utility skill (any listed name counts).
    money: &'static [&'static str],
    /// Points for having the money skill at all.
    base: f32,
    /// Skill-bound support gates on the money skill: (support ids, points).
    /// Negative points = support brick (e.g. Brutality on Flame Link).
    gates: &'static [(&'static [&'static str], f32, &'static str)],
    /// Presence-scored extra skills (auras, second utility): (names, points).
    skills: &'static [(&'static [&'static str], f32, &'static str)],
    /// Skill bricks: presence kills value: (names, negative points).
    bricks: &'static [(&'static [&'static str], f32, &'static str)],
    /// Money skill + every positive gate + no bricks = jackpot band.
    jackpot: bool,
}

pub const SCREENS: &[(&str, Screen)] = &[
    (
        "swiftblade",
        Screen {
            money: &["Rallying Cry"],
            base: 30.0,
            gates: &[
                (&["cdr"], 30.0, "Cooldown Recovery on Rallying Cry"),
                (&["more_duration"], 25.0, "More Duration on Rallying Cry"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: true, // 10d floor / 38d 5th, n=38 — deepest non-meta package
        },
    ),
    (
        "shattersword",
        Screen {
            money: &["Rallying Cry"],
            base: 25.0,
            gates: &[
                (&["cdr"], 28.0, "Cooldown Recovery on Rallying Cry"),
                (&["more_duration"], 22.0, "More Duration on Rallying Cry"),
            ],
            skills: &[(&["Lancing Steel"], 5.0, "Lancing Steel secondary")],
            bricks: &[],
            jackpot: true, // 6d floor, thin book (n=9)
        },
    ),
    (
        "fallen_reverend",
        Screen {
            money: &[
                "Reinforce: Fallen Bishop",
                "Reinforce: Fallen Emperor",
                "Reinforce: Fallen Osseotitan",
            ],
            base: 20.0,
            gates: &[],
            skills: &[
                (&["Wrath"], 20.0, "Wrath aura"),
                (&["Zealotry"], 20.0, "Zealotry aura"),
            ],
            bricks: &[(
                &["Battlemage's Cry"],
                -35.0,
                "Battlemage's Cry in the aura slot (~97% price loss)",
            )],
            jackpot: false, // 250-355c deep (n≈400-500)
        },
    ),
    (
        "flamehand",
        Screen {
            money: &["Rolling Magma"],
            base: 25.0,
            gates: &[
                (
                    &["gilded_area_per_projectile"],
                    25.0,
                    "Gilded Area per Projectile on Rolling Magma",
                ),
                (&["gmp"], 15.0, "GMP on Rolling Magma"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // 5d 5th but n=16; scarcity family (8.2k listings)
        },
    ),
    (
        "eruptor",
        Screen {
            money: &["Flame Link"],
            base: 25.0,
            gates: &[
                (
                    &["gilded_empowered_link"],
                    25.0,
                    "Gilded Empowered Link on Flame Link",
                ),
                (&["more_duration"], 8.0, "More Duration on Flame Link"),
                (&["cdr"], 8.0, "Cooldown Recovery on Flame Link"),
                (
                    &["brutality"],
                    -30.0,
                    "Brutality bricks the Flame Link setup",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "cruel_mistress",
        Screen {
            money: &["Void Sphere"],
            base: 15.0,
            gates: &[
                (
                    &["gilded_sphere_frequency"],
                    22.0,
                    "Gilded Sphere Frequency on Void Sphere",
                ),
                (
                    &["physical_as_extra_chaos"],
                    18.0,
                    "Phys as Extra Chaos on Void Sphere",
                ),
            ],
            skills: &[
                (&["Envy"], 25.0, "Envy aura (chaos curse-bot demand)"),
                (&["Zealotry"], 20.0, "Zealotry aura"),
            ],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "storming_zealot",
        Screen {
            money: &["Shockwave Totem of Shocking"],
            base: 10.0,
            gates: &[
                (
                    &["gilded_astral_totem"],
                    25.0,
                    "Gilded Astral Totem on Shockwave Totem",
                ),
                (&["more_duration"], 10.0, "More Duration on Shockwave Totem"),
                (&["aoe"], 10.0, "Increased AoE on Shockwave Totem"),
                (
                    &["multiple_totems"],
                    15.0,
                    "Multiple Totems (thin premium asks)",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // 20/21k listings are the 1c Divine Ire dump
        },
    ),
    (
        "bastion",
        Screen {
            money: &["Impenetrable Bastion"],
            base: 20.0,
            gates: &[
                (&["cdr"], 18.0, "Cooldown Recovery on Impenetrable Bastion"),
                (
                    &["more_duration"],
                    15.0,
                    "More Duration on Impenetrable Bastion",
                ),
            ],
            skills: &[(&["Intimidating Cry"], 8.0, "Intimidating Cry utility")],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "cardinal",
        Screen {
            money: &["Consecrated Path"],
            base: 20.0,
            gates: &[
                (
                    &["gilded_consecration"],
                    20.0,
                    "Gilded Consecration on Consecrated Path",
                ),
                (
                    &["faster_attacks"],
                    12.0,
                    "Faster Attacks on Consecrated Path",
                ),
                (&["aoe"], 8.0, "Increased AoE on Consecrated Path"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "earthshaker",
        Screen {
            money: &["Molten Shell"],
            base: 15.0,
            gates: &[
                (&["cdr"], 20.0, "Cooldown Recovery on Molten Shell"),
                (
                    &["gilded_physical_damage_reduction"],
                    20.0,
                    "Gilded Phys Damage Reduction on Molten Shell",
                ),
            ],
            skills: &[(&["Burrow"], 5.0, "Burrow utility")],
            bricks: &[],
            jackpot: false, // value is the defensive utility, not the slams
        },
    ),
    (
        "shock_ambusher",
        Screen {
            money: &["Vaal Lightning Trap"],
            base: 25.0,
            gates: &[
                (
                    &["added_lightning"],
                    15.0,
                    "Added Lightning on Vaal Lightning Trap",
                ),
                (
                    &["more_duration"],
                    12.0,
                    "More Duration on Vaal Lightning Trap",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "withertouch",
        Screen {
            money: &["Scourstorm"],
            base: 20.0,
            gates: &[
                (&["dot_multiplier"], 15.0, "DoT Multiplier on Scourstorm"),
                (&["cdr"], 8.0, "Cooldown Recovery on Scourstorm"),
                (&["swift_affliction"], 8.0, "Swift Affliction on Scourstorm"),
            ],
            skills: &[(&["Malevolence"], 8.0, "Malevolence aura")],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "frosthand",
        Screen {
            money: &["Ice Nova", "Ice Nova of Projection"],
            base: 20.0,
            gates: &[
                (
                    &["gilded_freezer_burn"],
                    18.0,
                    "Gilded Freezer Burn on Ice Nova",
                ),
                (&["cold_penetration"], 12.0, "Cold Penetration on Ice Nova"),
                (&["ailment_effect"], 8.0, "Ailment Effect on Ice Nova"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "winter_deacon",
        Screen {
            money: &["Earthquake of Winter"],
            base: 20.0,
            gates: &[
                (
                    &["concentrated_effect"],
                    15.0,
                    "Concentrated Effect on Earthquake of Winter",
                ),
                (
                    &["hypothermia"],
                    15.0,
                    "Hypothermia on Earthquake of Winter",
                ),
                (
                    &["freeze_chance"],
                    8.0,
                    "Freeze Chance on Earthquake of Winter",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "frost_ambusher",
        Screen {
            money: &["Ice Trap"],
            base: 18.0,
            gates: &[
                (&["cold_penetration"], 15.0, "Cold Penetration on Ice Trap"),
                (&["trigger_radius"], 12.0, "Trigger Radius on Ice Trap"),
                (
                    &["trap_and_mine_damage"],
                    10.0,
                    "Trap & Mine Damage on Ice Trap",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "flaming_charlatan",
        Screen {
            money: &["Wave of Conviction of Trarthus"],
            base: 20.0,
            gates: &[
                (&["added_fire"], 20.0, "Added Fire on Wave of Conviction"),
                (&["cdr"], 15.0, "Cooldown Recovery on Wave of Conviction"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // best campaign merc per akrpg; modest resale
        },
    ),
    (
        "sanguimancer",
        Screen {
            money: &["Vaal Reap"],
            base: 20.0,
            gates: &[
                (
                    &["gilded_searing_agony"],
                    18.0,
                    "Gilded Searing Agony on Vaal Reap",
                ),
                (&["dot_multiplier"], 10.0, "DoT Multiplier on Vaal Reap"),
                (&["cdr"], 10.0, "Cooldown Recovery on Vaal Reap"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "bladecaster",
        Screen {
            money: &["Seismic Crush"],
            base: 18.0,
            gates: &[
                (&["crit_damage"], 12.0, "Crit Damage on Seismic Crush"),
                (&["crit_chance"], 10.0, "Crit Chance on Seismic Crush"),
                (&["brutality"], 8.0, "Brutality on Seismic Crush"),
                (&["aoe"], 6.0, "Increased AoE on Seismic Crush"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "smoulderstrike",
        Screen {
            money: &["Infernal Cry"],
            base: 20.0,
            gates: &[
                (&["aoe"], 18.0, "Increased AoE on Infernal Cry"),
                (&["more_duration"], 15.0, "More Duration on Infernal Cry"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // market prices the warcry, not Molten Strike
        },
    ),
    (
        "bloodletter",
        Screen {
            money: &["Leap Slam", "Leap Slam of Groundbreaking"],
            base: 8.0,
            gates: &[
                (&["gilded_frenzy"], 28.0, "Gilded Frenzy on Leap Slam"),
                (&["faster_attacks"], 18.0, "Faster Attacks on Leap Slam"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // DPS primaries are 1c at any depth
        },
    ),
    (
        "ripper",
        Screen {
            money: &["Leap Slam", "Leap Slam of Groundbreaking"],
            base: 8.0,
            gates: &[
                (&["gilded_frenzy"], 28.0, "Gilded Frenzy on Leap Slam"),
                (&["pulverise"], 10.0, "Pulverise on Leap Slam"),
                (&["brutality"], 10.0, "Brutality on Leap Slam"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "striker",
        Screen {
            money: &["Leap Slam", "Leap Slam of Groundbreaking"],
            base: 8.0,
            gates: &[
                (&["gilded_frenzy"], 25.0, "Gilded Frenzy on Leap Slam"),
                (&["brutality"], 18.0, "Brutality on Leap Slam"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "bladebitter",
        Screen {
            money: &["Pestilent Strike"],
            base: 15.0,
            gates: &[
                (
                    &["faster_attacks"],
                    10.0,
                    "Faster Attacks on Pestilent Strike",
                ),
                (&["chance_to_poison"], 8.0, "Poison on Pestilent Strike"),
                (
                    &["dot_multiplier"],
                    10.0,
                    "DoT Multiplier on Pestilent Strike",
                ),
                (
                    &["ailment_effect"],
                    8.0,
                    "Ailment Effect on Pestilent Strike",
                ),
            ],
            skills: &[(&["Malevolence"], 6.0, "Malevolence aura")],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "flamequiver",
        Screen {
            money: &["Artillery Ballista"],
            base: 20.0,
            gates: &[
                (
                    &["gilded_totemic_onslaught"],
                    18.0,
                    "Gilded Totemic Onslaught on Artillery Ballista",
                ),
                (&["aoe"], 8.0, "Increased AoE on Artillery Ballista"),
                (
                    &["fire_penetration"],
                    8.0,
                    "Fire Penetration on Artillery Ballista",
                ),
                (
                    &["multiple_totems"],
                    12.0,
                    "Multiple Totems (thin premium asks)",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "toxicologist",
        Screen {
            money: &["Scourge Arrow of Menace"],
            base: 15.0,
            gates: &[
                (
                    &["gilded_additional_pods"],
                    18.0,
                    "Gilded Additional Pods on Scourge Arrow",
                ),
                (&["gmp"], 12.0, "GMP on Scourge Arrow"),
                (&["mirage_archer"], 8.0, "Mirage Archer on Scourge Arrow"),
                (
                    &["physical_as_extra_chaos"],
                    8.0,
                    "Phys as Extra Chaos on Scourge Arrow",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "reanimator",
        Screen {
            money: &["Raise Zombie of Falling"],
            base: 18.0,
            gates: &[
                (&["minion_damage"], 15.0, "Minion Damage on Raise Zombie"),
                (&["added_chaos"], 10.0, "Added Chaos on Raise Zombie"),
                (
                    &["melee_physical_damage"],
                    10.0,
                    "Melee Phys on Raise Zombie",
                ),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    (
        "warpriest",
        Screen {
            money: &["Herald of Purity"],
            base: 8.0,
            gates: &[
                (
                    &["minion_damage"],
                    18.0,
                    "Minion Damage on Herald of Purity",
                ),
                (&["pulverise"], 12.0, "Pulverise on Herald of Purity"),
                (&["aoe"], 8.0, "Increased AoE on Herald of Purity"),
                (&["brutality"], 8.0, "Brutality on Herald of Purity"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // Dominating Blow primaries are 1c
        },
    ),
    (
        "mysterious_diver",
        Screen {
            money: &["Frost Blades"],
            base: 10.0,
            gates: &[
                (&["edwa"], 20.0, "Greater EDWA on Frost Blades"),
                (&["return"], 20.0, "Return on Frost Blades"),
                (&["hypothermia"], 15.0, "Hypothermia on Frost Blades"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // budget Combatant package, ~10x cheaper
        },
    ),
    (
        "blade_ambusher",
        Screen {
            money: &["Bear Trap"],
            base: 5.0,
            gates: &[
                (&["trigger_radius"], 10.0, "Trigger Radius on Bear Trap"),
                (&["cdr"], 10.0, "Cooldown Recovery on Bear Trap"),
            ],
            skills: &[],
            bricks: &[],
            jackpot: false, // great to USE, near-worthless to SELL (1c at n=3200+)
        },
    ),
];

pub fn screen_for(family: &str) -> Option<&'static Screen> {
    SCREENS.iter().find(|(f, _)| *f == family).map(|(_, s)| s)
}

pub fn score(merc: &Mercenary, family: &str, screen: &Screen) -> ScoreResult {
    let money = merc.skill(screen.money);
    let has_money = money.is_some();
    let mut score = 0.0;
    let mut parts = Vec::new();
    let mut bricks = Vec::new();
    let mut all_gates = true;
    if has_money {
        score += screen.base;
        parts.push(ScoreBreakdown {
            label: format!(
                "{} (money skill)",
                money.map(|s| s.canonical.as_str()).unwrap_or("")
            ),
            points: screen.base,
            detail: String::new(),
        });
        for (ids, pts, label) in screen.gates {
            let factor = money.map(|s| s.t(ids)).unwrap_or(0.0);
            if factor > 0.0 {
                // A support brick bricks at any tier: charge the full penalty.
                let earned = if *pts < 0.0 { *pts } else { pts * factor };
                score += earned;
                parts.push(ScoreBreakdown {
                    label: (*label).into(),
                    points: earned,
                    detail: String::new(),
                });
                if *pts < 0.0 {
                    bricks.push((*label).to_string());
                    all_gates = false;
                } else if factor < 0.8 {
                    // T1 gates earn points but don't satisfy the jackpot package.
                    all_gates = false;
                }
            } else if *pts > 0.0 {
                all_gates = false;
            }
        }
    } else {
        all_gates = false;
    }
    for (names, pts, label) in screen.skills {
        if merc.has_skill(names) {
            score += pts;
            parts.push(ScoreBreakdown {
                label: (*label).into(),
                points: *pts,
                detail: String::new(),
            });
        }
    }
    for (names, pts, label) in screen.bricks {
        if merc.has_skill(names) {
            score += pts;
            bricks.push((*label).to_string());
            parts.push(ScoreBreakdown {
                label: (*label).into(),
                points: *pts,
                detail: String::new(),
            });
        }
    }
    let jackpot = screen.jackpot && has_money && all_gates && bricks.is_empty();
    if !jackpot {
        // Non-jackpot market screens stay below the very-valuable band.
        score = score.min(79.0);
    }
    let mut result = super::extra::finish(family, score, jackpot, bricks, parts);
    result.notes = vec![
        "3.29 market-floor screen — Perandus Ledger asking-price snapshot (2026-09-01); floors are asks, not sales."
            .into(),
    ];
    if !has_money {
        result.notes.push(format!(
            "Missing the money skill ({}) — this family's other variants are dump-tier.",
            screen.money.join(" / ")
        ));
    }
    result
}
