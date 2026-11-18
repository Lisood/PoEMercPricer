//! Per-family 3.30 market screens for the families without bespoke code.
//!
//! Recalibrated from the 2026-11-18 Covens trade-site survey (4,340 level-83
//! securable buyouts: the 50 dearest and 20 cheapest asks per warrant type).
//! Every row cites its numbers; the rules (sample size, top-share thresholds,
//! tier targets, jackpot test) are in docs/market-3.30.md. Asks, not sales.
//! Families whose money skill the survey does not price, with no proven
//! replacement, have no row and fall back to `extra::score_generic`.
//!
//! Gates bind to one primary money skill per family. Packages on a second
//! skill fold into skill-presence bonuses or are dropped.

use crate::models::{Mercenary, ScoreBreakdown, ScoreResult, SupportTier};

pub struct Screen {
    /// Primary money/utility skill (any listed name counts).
    pub money: &'static [&'static str],
    /// Points for having the money skill at all.
    pub base: f32,
    /// Skill-bound support gates on the money skill: (support ids, points).
    /// Negative points = support brick (e.g. Brutality on Flame Link).
    pub gates: &'static [(&'static [&'static str], f32, &'static str)],
    /// Presence-scored extra skills (auras, second utility): (names, points).
    pub skills: &'static [(&'static [&'static str], f32, &'static str)],
    /// Skill bricks: presence kills value: (names, negative points).
    pub bricks: &'static [(&'static [&'static str], f32, &'static str)],
    /// Money skill + every positive gate (T2+) + no bricks = jackpot band.
    pub jackpot: bool,
}

pub const SCREENS: &[(&str, Screen)] = &[
    // 3.30: premium (top-50 median 50d, cheapest-20 median 3c); full package 79. docs/market-3.30.md.
    (
        "sniper",
        Screen {
            money: &["Tornado Shot"],
            base: 45.0,
            gates: &[
                (&["gmp"], 10.0, "GMP on Tornado Shot"), // confirmed: n=22 top=0.86 med=50d p75=65d
                (&["brutality"], -40.0, "Brutality on Tornado Shot"), // keep (undecided): n=34+1mir top=0.74 med=38d p75=95d
                (
                    &["gilded_secondary_shots"],
                    12.0,
                    "Gilded Secondary Shots on Tornado Shot",
                ), // added: n=8+1mir top=1.00 med=38d p75=62d
                (&["arrow_nova"], 12.0, "Arrow Nova on Tornado Shot"), // added: n=28+1mir top=0.97 med=50d p75=100d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: premium (top-50 median 100d, cheapest-20 median 3c); full package 79. docs/market-3.30.md.
    (
        "thunderquiver",
        Screen {
            money: &["Lightning Arrow"],
            base: 47.0,
            gates: &[
                (&["return"], 15.0, "Return on LA"), // confirmed: n=25+4mir top=0.86 med=100d p75=100d
                (&["gmp"], 10.0, "GMP on LA"), // keep (undecided): n=22+3mir top=0.84 med=90d p75=100d
            ],
            skills: &[
                (&["Wrath"], 7.0, "Wrath"), // added: n=22+2mir top=0.92 med=100d p75=108d
            ],
            bricks: &[
                (&["Galvanic Arrow"], -25.0, "Galvanic Arrow"), // keep (undecided): n=34+5mir top=0.64 med=75d p75=100d
            ],
            jackpot: false,
        },
    ),
    // 3.30: high (top-50 median 30d, cheapest-20 median 9c); full package 71. docs/market-3.30.md.
    (
        "swiftblade",
        Screen {
            money: &["Rallying Cry"],
            base: 25.0,
            gates: &[
                (&["cdr"], 25.0, "Cooldown Recovery on Rallying Cry"), // confirmed: n=60 top=0.88 med=25d p75=40d
                (&["more_duration"], 21.0, "More Duration on Rallying Cry"), // confirmed: n=62 top=0.94 med=30d p75=50d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 11d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "shattersword",
        Screen {
            money: &["Rallying Cry"],
            base: 40.0,
            gates: &[
                (&["more_duration"], 22.0, "More Duration on Rallying Cry"), // keep (undecided): n=23 top=0.78 med=10d p75=18d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 10d, cheapest-20 median 15c); full package 62. docs/market-3.30.md.
    (
        "fallen_reverend",
        Screen {
            money: &[
                "Reinforce: Fallen Bishop",
                "Reinforce: Fallen Emperor",
                "Reinforce: Fallen Osseotitan",
            ],
            base: 22.0,
            gates: &[],
            skills: &[
                (&["Wrath"], 20.0, "Wrath aura"), // confirmed: n=73 top=0.89 med=10d p75=15d
                (&["Zealotry"], 20.0, "Zealotry aura"), // confirmed: n=71 top=0.85 med=10d p75=15d
            ],
            bricks: &[
                (
                    &["Battlemage's Cry"],
                    -35.0,
                    "Battlemage's Cry in the aura slot (~97% price loss)",
                ), // keep (undecided): n=56 top=0.62 med=5d p75=20d
                (&["Frostblink"], -35.0, "Frostblink"), // added (brick): n=40 top=0.47 med=40c p75=10d
            ],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 10d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "flamehand",
        Screen {
            money: &["Rolling Magma"],
            base: 20.0,
            gates: &[
                (
                    &["gilded_area_per_projectile"],
                    20.0,
                    "Gilded Area per Projectile on Rolling Magma",
                ), // keep (n<8): n=1 top=1.00 med=10d p75=10d
                (&["gmp"], 12.0, "GMP on Rolling Magma"), // confirmed: n=8 top=1.00 med=10d p75=20d
            ],
            skills: &[
                (&["Anger"], 5.0, "Anger"), // added: n=21 top=0.95 med=8d p75=10d
                (&["Vaal Flameblast"], 5.0, "Vaal Flameblast"), // added: n=25 top=0.92 med=6d p75=10d
            ],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: floor (top-50 median 3d, cheapest-20 median 10c); full package 53. docs/market-3.30.md.
    (
        "eruptor",
        Screen {
            money: &["Flame Link"],
            base: 18.0,
            gates: &[
                (
                    &["gilded_empowered_link"],
                    18.0,
                    "Gilded Empowered Link on Flame Link",
                ), // keep (n<8): n=3 top=1.00 med=5d p75=18d
                (&["more_duration"], 6.0, "More Duration on Flame Link"), // keep (undecided): n=14 top=0.79 med=4d p75=7d
                (&["cdr"], 6.0, "Cooldown Recovery on Flame Link"), // keep (undecided): n=19 top=0.68 med=2d p75=5d
                                                                    // Brutality (3.29 -30) removed: Flame Link cannot roll it in 3.30.
            ],
            skills: &[
                (&["Herald of Ash"], 5.0, "Herald of Ash"), // added: n=22 top=0.95 med=4d p75=6d
            ],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: premium (top-50 median 50d, cheapest-20 median 10c); full package 85, jackpot. docs/market-3.30.md.
    (
        "cruel_mistress",
        Screen {
            money: &["Soulrend of Reaping"],
            base: 40.0,
            gates: &[
                (&["gmp"], 22.0, "GMP on Soulrend of Reaping"), // confirmed: n=24+5mir top=1.00 med=50d p75=70d
                (&["return"], 18.0, "Return on Soulrend of Reaping"), // confirmed: n=39+5mir top=1.00 med=50d p75=75d
            ],
            skills: &[
                (&["Envy"], 5.0, "Envy aura"), // keep (undecided): n=87+2mir top=0.80 med=40d p75=55d
            ],
            bricks: &[],
            jackpot: true,
        },
    ),
    // 3.30: mid (top-50 median 5d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "storming_zealot",
        Screen {
            money: &["Shockwave Totem of Shocking"],
            base: 9.0,
            gates: &[
                (
                    &["gilded_astral_totem"],
                    22.0,
                    "Gilded Astral Totem on Shockwave Totem",
                ), // keep (n<8): n=6 top=1.00 med=8d p75=10d
                (&["more_duration"], 9.0, "More Duration on Shockwave Totem"), // keep (undecided): n=20 top=0.70 med=4d p75=6d
                (&["aoe"], 9.0, "Increased AoE on Shockwave Totem"), // keep (undecided): n=16 top=0.69 med=4d p75=6d
                (
                    &["multiple_totems"],
                    13.0,
                    "Multiple Totems (thin premium asks)",
                ), // keep (undecided): n=15 top=0.67 med=5d p75=10d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 10d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "bastion",
        Screen {
            money: &["Prismplate"],
            base: 54.0,
            gates: &[],
            skills: &[
                (&["Intimidating Cry"], 8.0, "Intimidating Cry utility"), // keep (undecided): n=67 top=0.79 med=10d p75=20d
            ],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: floor (top-50 median 4d, cheapest-20 median 8c); full package 52. docs/market-3.30.md.
    (
        "earthshaker",
        Screen {
            money: &["Ground Slam"],
            base: 40.0,
            gates: &[
                (&["fortify"], 12.0, "Fortify on Ground Slam"), // added: n=13 top=1.00 med=5d p75=20d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 10d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "shock_ambusher",
        Screen {
            money: &["Vaal Lightning Trap"],
            base: 47.0,
            gates: &[
                (
                    &["added_lightning"],
                    15.0,
                    "Added Lightning on Vaal Lightning Trap",
                ), // keep (undecided): n=37 top=0.73 med=5d p75=10d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 6d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "winter_deacon",
        Screen {
            money: &["Earthquake of Winter"],
            base: 39.0,
            gates: &[
                (
                    &["concentrated_effect"],
                    15.0,
                    "Concentrated Effect on Earthquake of Winter",
                ), // confirmed: n=15+1mir top=0.88 med=10d p75=10d
                (
                    &["freeze_chance"],
                    8.0,
                    "Freeze Chance on Earthquake of Winter",
                ), // keep (undecided): n=19+1mir top=0.75 med=5d p75=10d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 5d, cheapest-20 median 4c); full package 62. docs/market-3.30.md.
    (
        "frost_ambusher",
        Screen {
            money: &["Ice Trap"],
            base: 47.0,
            gates: &[
                (&["cold_penetration"], 15.0, "Cold Penetration on Ice Trap"), // keep (undecided): n=14+1mir top=0.67 med=5d p75=9d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: floor (top-50 median 2d, cheapest-20 median 5c); full package 53. docs/market-3.30.md.
    (
        "flaming_charlatan",
        Screen {
            money: &["Wave of Conviction of Trarthus"],
            base: 17.0,
            gates: &[
                (&["added_fire"], 17.0, "Added Fire on Wave of Conviction"), // keep (undecided): n=11 top=0.55 med=2d p75=8d
                (&["cdr"], 13.0, "Cooldown Recovery on Wave of Conviction"), // keep (n<8): n=6 top=0.67 med=1d p75=2d
            ],
            skills: &[
                (&["Meteor"], 6.0, "Meteor"), // added: n=22 top=0.91 med=1d p75=3d
            ],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 5d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "sanguimancer",
        Screen {
            money: &["Vaal Reap"],
            base: 24.0,
            gates: &[
                (
                    &["gilded_searing_agony"],
                    18.0,
                    "Gilded Searing Agony on Vaal Reap",
                ), // keep (n<8): n=3 top=0.67 med=3d p75=6d
                (&["dot_multiplier"], 10.0, "DoT Multiplier on Vaal Reap"), // confirmed: n=21 top=0.86 med=5d p75=10d
                (&["cdr"], 10.0, "Cooldown Recovery on Vaal Reap"), // keep (undecided): n=23 top=0.78 med=5d p75=10d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 10d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "bladecaster",
        Screen {
            money: &["Seismic Crush"],
            base: 26.0,
            gates: &[
                (&["crit_damage"], 12.0, "Crit Damage on Seismic Crush"), // keep (undecided): n=9 top=0.67 med=10d p75=20d
                (&["crit_chance"], 10.0, "Crit Chance on Seismic Crush"), // confirmed: n=10 top=1.00 med=15d p75=27d
                (&["brutality"], 8.0, "Brutality on Seismic Crush"), // keep (undecided): n=12 top=0.67 med=6d p75=32d
                (&["aoe"], 6.0, "Increased AoE on Seismic Crush"), // keep (undecided): n=12 top=0.67 med=6d p75=10d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: high (top-50 median 23d, cheapest-20 median 4c); full package 72. docs/market-3.30.md.
    (
        "smoulderstrike",
        Screen {
            money: &["Infernal Cry"],
            base: 39.0,
            gates: &[
                (&["aoe"], 18.0, "Increased AoE on Infernal Cry"), // keep (undecided): n=11+1mir top=0.75 med=24d p75=50d
                (&["more_duration"], 15.0, "More Duration on Infernal Cry"), // keep (undecided): n=10 top=0.80 med=18d p75=24d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 12d, cheapest-20 median 4c); full package 62. docs/market-3.30.md.
    (
        "striker",
        Screen {
            money: &["Leap Slam", "Leap Slam of Groundbreaking"],
            base: 19.0,
            gates: &[
                (&["gilded_frenzy"], 25.0, "Gilded Frenzy on Leap Slam"), // keep (n<8): n=5 top=0.40 med=5c p75=25d
                (&["brutality"], 18.0, "Brutality on Leap Slam"), // confirmed: n=23+1mir top=0.88 med=10d p75=18d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: high (top-50 median 20d, cheapest-20 median 5c); full package 72. docs/market-3.30.md.
    (
        "bladebitter",
        Screen {
            money: &["Pestilent Strike"],
            // Ailment Effect (3.29 +8) removed: the 3.30 catalog has no Pestilent
            // Strike tier for it; base 40 -> 48 keeps the high target of 72.
            base: 48.0,
            gates: &[
                (
                    &["faster_attacks"],
                    10.0,
                    "Faster Attacks on Pestilent Strike",
                ), // keep (undecided): n=34 top=0.74 med=10d p75=32d
                (&["chance_to_poison"], 8.0, "Poison on Pestilent Strike"), // keep (undecided): n=30 top=0.83 med=10d p75=20d
            ],
            skills: &[
                (&["Malevolence"], 6.0, "Malevolence aura"), // keep (undecided): n=57+1mir top=0.79 med=10d p75=25d
            ],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 10d, cheapest-20 median 4c); full package 63. docs/market-3.30.md.
    (
        "flamequiver",
        Screen {
            money: &["Artillery Ballista"],
            base: 19.0,
            gates: &[
                (
                    &["gilded_totemic_onslaught"],
                    17.0,
                    "Gilded Totemic Onslaught on Artillery Ballista",
                ), // keep (n<8): n=2 top=0.50 med=8d p75=11d
                (&["aoe"], 8.0, "Increased AoE on Artillery Ballista"), // confirmed: n=10 top=1.00 med=15d p75=22d
                (
                    &["fire_penetration"],
                    8.0,
                    "Fire Penetration on Artillery Ballista",
                ), // keep (undecided): n=12 top=0.75 med=8d p75=44d
                (
                    &["multiple_totems"],
                    11.0,
                    "Multiple Totems (thin premium asks)",
                ), // keep (undecided): n=8 top=0.75 med=32d p75=68d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 12d, cheapest-20 median 4c); full package 62. docs/market-3.30.md.
    (
        "toxicologist",
        Screen {
            money: &["Scourge Arrow of Menace"],
            base: 16.0,
            gates: &[
                (
                    &["gilded_additional_pods"],
                    18.0,
                    "Gilded Additional Pods on Scourge Arrow",
                ), // confirmed: n=9 top=1.00 med=10d p75=10d
                (&["gmp"], 12.0, "GMP on Scourge Arrow"), // confirmed: n=16 top=0.88 med=12d p75=32d
                (&["mirage_archer"], 8.0, "Mirage Archer on Scourge Arrow"), // confirmed: n=18 top=0.89 med=10d p75=14d
                (
                    &["physical_as_extra_chaos"],
                    8.0,
                    "Phys as Extra Chaos on Scourge Arrow",
                ), // keep (undecided): n=27 top=0.74 med=10d p75=22d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: floor (top-50 median 2d, cheapest-20 median 5c); full package 52. docs/market-3.30.md.
    (
        "reanimator",
        Screen {
            money: &["Summon Raging Spirit"],
            base: 40.0,
            gates: &[
                (
                    &["minion_damage"],
                    12.0,
                    "Minion Damage on Summon Raging Spirit",
                ), // added: n=42 top=0.93 med=2d p75=5d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // No 3.30 listings (survey total 0): the 3.29 screen is kept unchanged.
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
            jackpot: false,
        },
    ),
    // 3.30: high (top-50 median 20d, cheapest-20 median 3c); full package 72. docs/market-3.30.md.
    (
        "blade_ambusher",
        Screen {
            money: &["Bear Trap"],
            base: 62.0,
            gates: &[
                (&["trigger_radius"], 10.0, "Trigger Radius on Bear Trap"), // keep (undecided): n=22+1mir top=0.83 med=15d p75=30d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: mid (top-50 median 10d, cheapest-20 median 5c); full package 62. docs/market-3.30.md.
    (
        "stormhand",
        Screen {
            money: &["Arc"],
            base: 20.0,
            gates: &[
                (&["chain"], 15.0, "Chain on Arc"), // confirmed: n=28 top=0.89 med=10d p75=20d
                (
                    &["gilded_chain_distance"],
                    10.0,
                    "Gilded Chain Distance on Arc",
                ), // confirmed: n=10 top=0.90 med=15d p75=43d
                (&["faster_casting"], 3.0, "Faster Casting on Arc"), // confirmed: n=37+1mir top=0.87 med=10d p75=20d
                (
                    &["lucky_lightning_damage"],
                    6.0,
                    "Lucky Lightning Damage on Arc",
                ), // added: n=14+1mir top=0.93 med=10d p75=19d
            ],
            skills: &[
                (
                    &["Ball Lightning of Static"],
                    8.0,
                    "Ball Lightning of Static",
                ), // keep (undecided): n=54+1mir top=0.73 med=10d p75=16d
            ],
            bricks: &[],
            jackpot: false,
        },
    ),
    // 3.30: premium (top-50 median 300d, cheapest-20 median 5c); full package 79. docs/market-3.30.md.
    (
        "chaplain_of_the_well",
        Screen {
            money: &["Abyssal Spray"],
            base: 55.0,
            gates: &[
                (&["return"], 12.0, "Return on Abyssal Spray"), // new screen gate: n=11+12mir top=0.83 med=300d p75=325d
                (&["faster_casting"], 12.0, "Faster Casting on Abyssal Spray"), // new screen gate: n=20+28mir top=0.84 med=300d p75=300d
            ],
            skills: &[],
            bricks: &[],
            jackpot: false,
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
    let mut all_gates = has_money;
    if let Some(money) = money {
        score += screen.base;
        parts.push(ScoreBreakdown {
            label: format!("{} (money skill)", money.canonical),
            points: screen.base,
            detail: String::new(),
        });
        for (ids, pts, label) in screen.gates {
            let tier = money.support_tier(ids);
            if tier == SupportTier::Absent {
                all_gates &= *pts < 0.0;
                continue;
            }
            // A support brick bricks at any tier: charge the full penalty.
            let earned = if *pts < 0.0 {
                *pts
            } else {
                pts * tier.factor()
            };
            score += earned;
            parts.push(ScoreBreakdown {
                label: (*label).into(),
                points: earned,
                detail: String::new(),
            });
            if *pts < 0.0 {
                bricks.push((*label).to_string());
            }
            // T1 gates earn points but don't satisfy the jackpot package.
            all_gates &= *pts > 0.0 && tier >= SupportTier::T2;
        }
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
    let jackpot = screen.jackpot && all_gates && bricks.is_empty();
    if !jackpot {
        // Non-jackpot market screens stay below the very-valuable band. A
        // `jackpot: true` screen can therefore sit in the "jackpot" band well
        // under 80 (Cruel Mistress with T2 gates is 40 + 0.8 * (22 + 18) = 72) while a
        // `jackpot: false` family with the same points caps at 79 / "good".
        score = score.min(79.0);
    }
    let mut result = super::extra::finish(family, score, jackpot, bricks, parts);
    result.notes = vec![
        "3.30 trade-survey screen: Covens trade-site asks (2026-11-18); asks, not sales.".into(),
    ];
    if !has_money {
        result.notes.push(format!(
            "Missing the money skill ({}); the 3.30 survey prices this family's other variants low.",
            screen.money.join(" / ")
        ));
    }
    result
}
