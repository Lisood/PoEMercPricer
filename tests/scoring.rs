use poemercpricer::models::{interpret_score, Mercenary, Skill, SupportTier};
use poemercpricer::scoring::market::SCREENS;
use poemercpricer::scoring::score_mercenary;

fn merc(family: &str, skills: Vec<Skill>) -> Mercenary {
    Mercenary {
        class_name: family.to_string(),
        family: family.into(),
        level: Some(83),
        skills,
        ..Default::default()
    }
}

#[test]
fn kineticist_example_qk_84() {
    let m = merc(
        "kineticist",
        vec![
            Skill::new("Kinetic Blast of Clustering", "Kinetic Blast of Clustering").with_supports(
                vec![
                    ("return".into(), SupportTier::T3),
                    ("gmp".into(), SupportTier::T3),
                    ("chain".into(), SupportTier::T2),
                    ("edwa".into(), SupportTier::T3),
                ],
            ),
            Skill::new("Greater Kinetic Blast", "Greater Kinetic Blast"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert_eq!(r.score.round() as i32, 84);
    assert!(r.jackpot);
    assert_eq!(r.band, "jackpot");
}

#[test]
fn kineticist_bricks_kill_premium() {
    let m = merc(
        "kineticist",
        vec![
            Skill::new("Kinetic Blast of Clustering", "Kinetic Blast of Clustering").with_supports(
                vec![
                    ("return".into(), SupportTier::T3),
                    ("gmp".into(), SupportTier::T3),
                ],
            ),
            Skill::new("Kinetic Bolt", "Kinetic Bolt"),
            Skill::new("Power Siphon", "Power Siphon"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.bricks.iter().any(|b| b == "Kinetic Bolt"));
    assert!(!r.jackpot);
    assert!(r.score <= 50.0);
}

#[test]
fn manyshot_icicle_rain_penalty() {
    let m = merc(
        "manyshot",
        vec![
            Skill::new("Ice Shot", "Ice Shot").with_supports(vec![
                ("gmp".into(), SupportTier::T3),
                ("return".into(), SupportTier::T3),
                ("edwa".into(), SupportTier::T3),
            ]),
            Skill::new("Vaal Ice Shot", "Vaal Ice Shot").with_supports(vec![
                ("return".into(), SupportTier::T3),
                ("edwa".into(), SupportTier::T3),
                ("cdr".into(), SupportTier::T3),
            ]),
            Skill::new("Icicle Rain", "Icicle Rain"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert_eq!(r.bricks, vec!["Icicle Rain".to_string()]);
    assert!(!r.jackpot);
    assert!(r.score < 60.0);
}

#[test]
fn manyshot_premium_no_icicle() {
    let m = merc(
        "manyshot",
        vec![
            Skill::new("Ice Shot", "Ice Shot").with_supports(vec![
                ("gmp".into(), SupportTier::T3),
                ("return".into(), SupportTier::T3),
                ("edwa".into(), SupportTier::T3),
            ]),
            Skill::new("Vaal Ice Shot", "Vaal Ice Shot").with_supports(vec![
                ("return".into(), SupportTier::T3),
                ("edwa".into(), SupportTier::T3),
                ("hypothermia".into(), SupportTier::T3),
            ]),
            Skill::new("Mirror Arrow", "Mirror Arrow"),
            Skill::new("Hatred", "Hatred"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.jackpot);
    assert!(r.score >= 80.0);

    // The 2026-09-02 ledger floors Ice Shot::Return+GMP beside a Vaal Return
    // without T3 Hypothermia at 10c, so it is not a jackpot path.
    let mut two_row = m.clone();
    two_row.skills[1]
        .supports
        .retain(|s| s.canonical != "hypothermia");
    let r = score_mercenary(&two_row, false);
    assert!(!r.jackpot);
    assert_ne!(r.band, "jackpot");
}

#[test]
fn combatant_frost_jackpot() {
    let m = merc(
        "combatant",
        vec![
            // Chain tops out at Tier 2 on Frost Blades (T3 is Gilded Chain
            // Distance, a different gem), so a real jackpot carries Chain T2.
            Skill::new("Frost Blades", "Frost Blades").with_supports(vec![
                ("return".into(), SupportTier::T3),
                ("chain".into(), SupportTier::T2),
                ("edwa".into(), SupportTier::T3),
                ("faster_attacks".into(), SupportTier::T3),
                ("hypothermia".into(), SupportTier::T3),
            ]),
            Skill::new("Static Strike", "Static Strike"),
            Skill::new("Herald of Ice", "Herald of Ice"),
            Skill::new("Dash", "Dash"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.jackpot);
    assert!(r.score >= 80.0);
    let sum: f32 = r.breakdown.iter().map(|p| p.points).sum();
    assert!((sum - r.score).abs() < 0.01);
}

#[test]
fn combatant_t1_frost_package_is_not_jackpot() {
    let m = merc(
        "combatant",
        vec![
            Skill::new("Frost Blades", "Frost Blades").with_supports(vec![
                ("return".into(), SupportTier::T1),
                ("chain".into(), SupportTier::T1),
                ("edwa".into(), SupportTier::T1),
            ]),
            Skill::new("Static Strike", "Static Strike"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert_ne!(r.band, "jackpot");
}

#[test]
fn kineticist_t1_return_gmp_is_not_jackpot() {
    let m = merc(
        "kineticist",
        vec![
            Skill::new("Kinetic Blast of Clustering", "Kinetic Blast of Clustering").with_supports(
                vec![
                    ("return".into(), SupportTier::T1),
                    ("gmp".into(), SupportTier::T1),
                ],
            ),
            Skill::new("Greater Kinetic Blast", "Greater Kinetic Blast"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert_ne!(r.band, "jackpot");
}

#[test]
fn combatant_check_band_is_reachable() {
    assert_eq!(interpret_score(60.0, "combatant").0, "check");
    assert_eq!(interpret_score(59.0, "combatant").0, "common");
    assert_eq!(interpret_score(55.0, "kineticist").0, "check");
}

#[test]
fn combatant_both_mains_penalized() {
    let m = merc(
        "combatant",
        vec![
            Skill::new("Frost Blades", "Frost Blades")
                .with_supports(vec![("chain".into(), SupportTier::T3)]),
            Skill::new("Wild Strike", "Wild Strike")
                .with_supports(vec![("edwa".into(), SupportTier::T3)]),
            Skill::new("Static Strike", "Static Strike"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.bricks.iter().any(|b| b == "Frost Blades + Wild Strike"));
    assert!(!r.jackpot);
}

#[test]
fn combatant_return_keeps_resale_value_without_proj_speed() {
    let m = merc(
        "combatant",
        vec![
            Skill::new("Frost Blades", "Frost Blades").with_supports(vec![
                ("chain".into(), SupportTier::T3),
                ("return".into(), SupportTier::T3),
            ]),
            Skill::new("Static Strike", "Static Strike"),
        ],
    );
    let off = score_mercenary(&m, false);
    let on = score_mercenary(&m, true);
    assert!((on.score - off.score).abs() < 0.01);
    assert!(off.score >= 50.0);
    assert!(off
        .highlights
        .iter()
        .any(|line| line.contains("valued for resale")));
}

#[test]
fn stormhand_market_sleeper_requires_skill_bound_chain_package() {
    let m = merc(
        "stormhand",
        vec![
            Skill::new("Arc", "Arc").with_supports(vec![
                ("chain".into(), SupportTier::T2),
                ("gilded_chain_distance".into(), SupportTier::T3),
                ("faster_casting".into(), SupportTier::T3),
            ]),
            Skill::new("Ball Lightning of Static", "Ball Lightning of Static"),
        ],
    );
    let result = score_mercenary(&m, false);
    assert!(result.jackpot);
    assert!(result
        .highlights
        .iter()
        .any(|line| line.contains("sleeper")));
}

#[test]
fn sniper_tornado_gmp_is_a_floor_package_not_a_jackpot() {
    // 2026-09-02 ledger: TS::GMP floors at 1c (n=8,383); see docs/market-3.29.md.
    let m = merc(
        "sniper",
        vec![Skill::new("Tornado Shot", "Tornado Shot")
            .with_supports(vec![("gmp".into(), SupportTier::T3)])],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert_eq!(r.band, "skip");
    assert!(!r.estimate);
}

#[test]
fn sniper_brutality_bricks_tornado_shot() {
    let m = merc(
        "sniper",
        vec![Skill::new("Tornado Shot", "Tornado Shot")
            .with_supports(vec![("brutality".into(), SupportTier::T3)])],
    );
    let r = score_mercenary(&m, false);
    assert!(r.bricks.iter().any(|b| b.contains("Brutality")));
    assert!(!r.jackpot);
    assert_eq!(r.band, "skip");
}

#[test]
fn sniper_arrow_nova_only_bricks_on_tornado_shot() {
    let elsewhere = merc(
        "sniper",
        vec![
            Skill::new("Tornado Shot", "Tornado Shot")
                .with_supports(vec![("gmp".into(), SupportTier::T3)]),
            Skill::new("Puncture", "Puncture")
                .with_supports(vec![("arrow_nova".into(), SupportTier::T3)]),
        ],
    );
    assert!(score_mercenary(&elsewhere, false).bricks.is_empty());

    let on_ts = merc(
        "sniper",
        vec![
            Skill::new("Tornado Shot", "Tornado Shot").with_supports(vec![
                ("gmp".into(), SupportTier::T3),
                ("arrow_nova".into(), SupportTier::T1),
            ]),
        ],
    );
    let r = score_mercenary(&on_ts, false);
    assert!(!r.jackpot);
    assert!(r.bricks.iter().any(|b| b.contains("Arrow Nova")));
    let sum: f32 = r.breakdown.iter().map(|p| p.points).sum();
    assert!((sum - r.score).abs() < 0.01);
    assert!(r.breakdown.iter().any(|p| p.points == -20.0));
}

#[test]
fn thunderquiver_return_gmp_package_is_not_a_jackpot() {
    // 2026-09-02 ledger: LA::Return+GMP floors at 1c (n=3,404).
    let m = merc(
        "thunderquiver",
        vec![
            Skill::new("Lightning Arrow", "Lightning Arrow").with_supports(vec![
                ("return".into(), SupportTier::T3),
                ("gmp".into(), SupportTier::T3),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert_eq!(r.band, "skip");
    assert!(!r.estimate);
}

#[test]
fn thunderquiver_galvanic_arrow_is_a_brick() {
    let m = merc(
        "thunderquiver",
        vec![
            Skill::new("Lightning Arrow", "Lightning Arrow")
                .with_supports(vec![("return".into(), SupportTier::T3)]),
            Skill::new("Galvanic Arrow", "Galvanic Arrow"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.bricks.iter().any(|b| b == "Galvanic Arrow"));
    assert!(!r.jackpot);
}

#[test]
fn generic_fallback_family_scores_an_estimate() {
    // warpriest_of_the_ruckus is the only family without a market screen (1 listing).
    let m = Mercenary {
        infamous: true,
        ..merc(
            "warpriest_of_the_ruckus",
            vec![
                Skill::new("Herald of Purity", "Herald of Purity").with_supports(vec![
                    ("minion_damage".into(), SupportTier::T3),
                    ("pulverise".into(), SupportTier::T3),
                    ("aoe".into(), SupportTier::T3),
                    ("brutality".into(), SupportTier::T3),
                ]),
            ],
        )
    };
    let r = score_mercenary(&m, false);
    assert!(r.estimate);
    assert!(!r.jackpot);
    assert!(r.score <= 79.0);
    assert_ne!(r.band, "very-valuable");
    assert_ne!(r.band, "jackpot-band");
    assert_ne!(r.band, "unsupported");

    // Unresolved gems earn nothing.
    let mut noisy = m.clone();
    noisy.skills[0].supports.extend(
        [
            ("unknown".to_string(), SupportTier::T3),
            ("ambiguous".to_string(), SupportTier::T3),
            (String::new(), SupportTier::T3),
        ]
        .map(|(id, tier)| poemercpricer::SupportGem {
            name: id.clone(),
            canonical: id,
            tier,
            ..Default::default()
        }),
    );
    assert_eq!(score_mercenary(&noisy, false).score, r.score);
}

#[test]
fn generic_fallback_ignores_non_catalog_junk_canonicals() {
    // canonical_support's fuzzy fallback returns (normalized_junk, 0.4) for
    // unrecognised text, so a stray clipboard line like "Str 155" inside a
    // skill section must not be scored as if it were a real support gem.
    let m = merc(
        "warpriest_of_the_ruckus",
        vec![
            Skill::new("Herald of Purity", "Herald of Purity").with_supports(vec![
                ("str 155".into(), SupportTier::T2),
                ("dexterity 90".into(), SupportTier::T2),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert_eq!(r.band, "skip");
    assert!(r
        .breakdown
        .iter()
        .find(|p| p.label == "Support tiers")
        .is_some_and(|p| p.points == 0.0));
}

#[test]
fn jackpot_screen_enters_the_jackpot_band_below_80() {
    let m = merc(
        "shattersword",
        vec![
            Skill::new("Rallying Cry", "Rallying Cry").with_supports(vec![
                ("cdr".into(), SupportTier::T3),
                ("more_duration".into(), SupportTier::T3),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert_eq!(r.score, 75.0);
    assert!(r.jackpot);
    assert_eq!(r.band, "jackpot");
}

#[test]
fn every_market_screen_full_package_and_missing_money_skill() {
    for (family, screen) in SCREENS {
        let positive: Vec<_> = screen
            .gates
            .iter()
            .filter(|(_, pts, _)| *pts > 0.0)
            .collect();
        let supports = positive
            .iter()
            .map(|(ids, _, _)| (ids[0].to_string(), SupportTier::T3))
            .collect();
        let expected = screen.base + positive.iter().map(|(_, pts, _)| pts).sum::<f32>();
        let full = merc(
            family,
            vec![Skill::new(screen.money[0], screen.money[0]).with_supports(supports)],
        );
        let r = score_mercenary(&full, false);
        assert!(r.bricks.is_empty(), "{family}");
        if screen.jackpot {
            assert!(r.jackpot, "{family}");
            assert_eq!(r.band, "jackpot", "{family}");
            assert!((r.score - expected.min(100.0)).abs() < 0.01, "{family}");
        } else {
            assert!(!r.jackpot, "{family}");
            let expected = expected.min(79.0);
            assert!((r.score - expected).abs() < 0.01, "{family}");
            assert_eq!(r.band, interpret_score(expected, family).0, "{family}");
        }

        let r = score_mercenary(&merc(family, vec![]), false);
        assert_eq!(r.band, "skip", "{family}");
        assert!(!r.jackpot, "{family}");
        assert!(
            r.notes.iter().any(|n| n.contains("money skill")),
            "{family}"
        );
    }
}

#[test]
fn scorer_support_ids_are_catalog_canonicals() {
    // Hand-collected from combatant.rs, kineticist.rs, manyshot.rs, extra.rs;
    // market gates come from the table itself.
    let mut ids = vec![
        "return",
        "chain",
        "edwa",
        "faster_attacks",
        "hypothermia",
        "gmp",
        "gilded_elemental_weakness_on_hit",
        "more_duration",
        "aoe",
        "pierce",
        "multistrike",
        "fork",
        "crit_damage",
        "sacred_wisps",
        "cdr",
        "gilded_chain_distance",
        "faster_casting",
    ];
    for (_, screen) in SCREENS {
        for (gate_ids, _, _) in screen.gates {
            ids.extend(*gate_ids);
        }
    }
    for id in ids {
        assert!(
            poemercpricer::catalog::support_icon(id).is_some(),
            "{id} is not a catalog support canonical"
        );
    }
}

#[test]
fn market_screen_skill_names_are_catalog_canonicals() {
    for (family, screen) in SCREENS {
        for name in screen.money {
            assert!(
                poemercpricer::catalog::skill_icon(name).is_some(),
                "{family}: money skill {name:?} is not a catalog canonical"
            );
        }
        for (names, _, label) in screen.skills {
            for name in *names {
                assert!(
                    poemercpricer::catalog::skill_icon(name).is_some(),
                    "{family}: skill {name:?} ({label}) is not a catalog canonical"
                );
            }
        }
        for (names, _, label) in screen.bricks {
            for name in *names {
                assert!(
                    poemercpricer::catalog::skill_icon(name).is_some(),
                    "{family}: brick skill {name:?} ({label}) is not a catalog canonical"
                );
            }
        }
    }
}

#[test]
fn swiftblade_rallying_cry_cdr_duration_is_jackpot() {
    let m = merc(
        "swiftblade",
        vec![
            Skill::new("Rallying Cry", "Rallying Cry").with_supports(vec![
                ("cdr".into(), SupportTier::T3),
                ("more_duration".into(), SupportTier::T3),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.jackpot);
    assert_eq!(r.band, "jackpot");
    assert!(!r.estimate);
}

#[test]
fn swiftblade_t1_gates_do_not_jackpot() {
    let m = merc(
        "swiftblade",
        vec![
            Skill::new("Rallying Cry", "Rallying Cry").with_supports(vec![
                ("cdr".into(), SupportTier::T1),
                ("more_duration".into(), SupportTier::T1),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert_ne!(r.band, "jackpot");
}

#[test]
fn eruptor_brutality_on_flame_link_is_a_reported_brick() {
    let m = merc(
        "eruptor",
        vec![Skill::new("Flame Link", "Flame Link").with_supports(vec![
            ("gilded_empowered_link".into(), SupportTier::T3),
            ("brutality".into(), SupportTier::T1),
        ])],
    );
    let r = score_mercenary(&m, false);
    assert!(r.bricks.iter().any(|b| b.contains("Brutality")));
    assert!(r.score < 50.0);
}

#[test]
fn swiftblade_missing_duration_is_not_jackpot() {
    let m = merc(
        "swiftblade",
        vec![Skill::new("Rallying Cry", "Rallying Cry")
            .with_supports(vec![("cdr".into(), SupportTier::T3)])],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert!(r.score < 65.0);
}

#[test]
fn storming_zealot_divine_ire_dump_is_skip() {
    let m = merc(
        "storming_zealot",
        vec![
            Skill::new("Divine Ire", "Divine Ire")
                .with_supports(vec![("faster_casting".into(), SupportTier::T3)]),
            Skill::new("Divine Retribution", "Divine Retribution"),
            Skill::new("Wrath", "Wrath"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert_eq!(r.band, "skip");
    assert!(r.notes.iter().any(|n| n.contains("money skill")));
}

#[test]
fn fallen_reverend_battlemage_cry_bricks_the_aura_bot() {
    let m = merc(
        "fallen_reverend",
        vec![
            Skill::new("Reinforce: Fallen Bishop", "Reinforce: Fallen Bishop"),
            Skill::new("Wrath", "Wrath"),
            Skill::new("Zealotry", "Zealotry"),
            Skill::new("Battlemage's Cry", "Battlemage's Cry"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.bricks.is_empty());
    assert!(r.score < 50.0);
}

#[test]
fn mysterious_diver_budget_combatant_package_scores_good() {
    let m = merc(
        "mysterious_diver",
        vec![
            Skill::new("Frost Blades", "Frost Blades").with_supports(vec![
                ("edwa".into(), SupportTier::T3),
                ("return".into(), SupportTier::T3),
                ("hypothermia".into(), SupportTier::T3),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert_eq!(r.band, "good");
    assert!(!r.jackpot);
}

#[test]
fn market_screen_keys_are_real_catalog_families() {
    let known: Vec<&str> = poemercpricer::catalog::known_families().collect();
    for (family, _) in poemercpricer::scoring::market::SCREENS {
        assert!(known.contains(family), "unknown screen family {family}");
    }
}

#[test]
fn every_catalog_family_gets_a_score() {
    for family in poemercpricer::catalog::known_families() {
        let r = score_mercenary(&merc(family, vec![]), false);
        assert_ne!(r.band, "unsupported", "family {family} left unscored");
    }
}

#[test]
fn unsupported_class() {
    let m = Mercenary {
        class_name: "Frosthand".into(),
        family: "other".into(),
        ..Default::default()
    };
    let r = score_mercenary(&m, false);
    assert_eq!(r.band, "unsupported");
    assert_eq!(r.score, 0.0);
}
