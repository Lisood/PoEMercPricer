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
fn manyshot_icicle_rain_is_not_a_score_brick() {
    // docs/market-3.30.md, bespoke table, "VIS + Icicle Rain": 175d median
    // (n=14+2 mirror, top-share 0.88), so the 3.29 -40 brick is gone. Icicle
    // Rain never sits beside the VIS + Mirror Arrow jackpot pair (n=0), so it still cannot jackpot.
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
    assert!(r.bricks.is_empty());
    assert!(!r.jackpot);
    assert_ne!(r.band, "jackpot");
    assert!(r.score >= 80.0);
    assert!(r
        .highlights
        .iter()
        .any(|h| h.contains("no Vaal Ice Shot + Mirror Arrow jackpot package")));
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
fn combatant_both_mains_is_not_a_brick_but_never_jackpots() {
    // docs/market-3.30.md, bespoke table, "Frost Blades + Wild Strike": 100d
    // median (n=20+3 mirror, top-share 0.87), level with the family, so the
    // 3.29 -28 penalty is gone. Both mains rule out the Frost Blades jackpot.
    let m = merc(
        "combatant",
        vec![
            Skill::new("Frost Blades", "Frost Blades").with_supports(vec![
                ("return".into(), SupportTier::T3),
                ("chain".into(), SupportTier::T2),
                ("edwa".into(), SupportTier::T3),
            ]),
            Skill::new("Wild Strike", "Wild Strike")
                .with_supports(vec![("edwa".into(), SupportTier::T3)]),
            Skill::new("Static Strike", "Static Strike"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.bricks.is_empty());
    assert!(!r.breakdown.iter().any(|p| p.label == "FB+WS penalty"));
    assert!(!r.jackpot);
    assert_ne!(r.band, "jackpot");
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
fn stormhand_arc_package_is_a_mid_screen_not_a_jackpot() {
    // docs/market-3.30.md, stormhand row: a mid family (10d top-50 median);
    // Arc + BLS + Chain + Gilded Chain Distance is n=1, so the 3.29 jackpot is
    // gone and the screen moved into market::SCREENS (base 20, Chain 15,
    // GCD 10, Faster Casting 3, BLS 8).
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
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert!((r.score - 53.0).abs() < 0.01);
    assert_eq!(r.band, "common");
    assert!(!r.estimate);
}

#[test]
fn sniper_tornado_gmp_is_a_check_not_a_jackpot() {
    // docs/market-3.30.md, sniper row: premium family (50d top-50 median);
    // TS::GMP is confirmed (n=22, top-share 0.86, 50d), TS::GMP + Arrow Nova
    // is n=6, too thin for a jackpot. Base 45 + GMP 10 = 55.
    let m = merc(
        "sniper",
        vec![Skill::new("Tornado Shot", "Tornado Shot")
            .with_supports(vec![("gmp".into(), SupportTier::T3)])],
    );
    let r = score_mercenary(&m, false);
    assert!(!r.jackpot);
    assert_eq!(r.score, 55.0);
    assert_eq!(r.band, "check");
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
fn sniper_arrow_nova_on_tornado_shot_is_a_gate() {
    // docs/market-3.30.md, sniper row: Arrow Nova on Tornado Shot is n=28+1
    // mirror, top-share 0.97, 50d median, so the 3.29 -20 brick became a +12
    // gate. It only counts on Tornado Shot.
    let elsewhere = merc(
        "sniper",
        vec![
            Skill::new("Tornado Shot", "Tornado Shot")
                .with_supports(vec![("gmp".into(), SupportTier::T3)]),
            Skill::new("Puncture", "Puncture")
                .with_supports(vec![("arrow_nova".into(), SupportTier::T3)]),
        ],
    );
    let r = score_mercenary(&elsewhere, false);
    assert!(r.bricks.is_empty());
    assert_eq!(r.score, 55.0);

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
    assert!(r.bricks.is_empty());
    let sum: f32 = r.breakdown.iter().map(|p| p.points).sum();
    assert!((sum - r.score).abs() < 0.01);
    // T1 Arrow Nova earns 12 * 0.6.
    assert!(r
        .breakdown
        .iter()
        .any(|p| p.label.contains("Arrow Nova") && (p.points - 7.2).abs() < 0.01));
    assert!(r.score > 55.0);
}

#[test]
fn thunderquiver_return_gmp_package_is_good_not_a_jackpot() {
    // docs/market-3.30.md, thunderquiver row: premium family (100d top-50
    // median), LA::Return confirmed (n=25+4 mirror, top-share 0.86), but
    // Return + GMP is n=5, so no jackpot. Base 47 + 15 + 10 = 72.
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
    assert_eq!(r.score, 72.0);
    assert_eq!(r.band, "good");
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
    // warpriest_of_the_ruckus has no 3.30 listings and no market screen.
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
    // docs/market-3.30.md, cruel_mistress row: SoR::GMP + Return is n=15+5
    // mirror, 50d median, top-share 1.00, so jackpot. T2 gates still satisfy
    // the package: 40 + 0.8 * (22 + 18) = 72.
    let m = merc(
        "cruel_mistress",
        vec![
            Skill::new("Soulrend of Reaping", "Soulrend of Reaping").with_supports(vec![
                ("gmp".into(), SupportTier::T2),
                ("return".into(), SupportTier::T2),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!((r.score - 72.0).abs() < 0.01);
    assert!(r.jackpot);
    assert_eq!(r.band, "jackpot");
}

#[test]
fn shattersword_lost_its_jackpot() {
    // docs/market-3.30.md, shattersword row: mid family (11d top-50 median),
    // CDR on Rallying Cry not priced (n=20, top-share 0.70).
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
    assert!(!r.jackpot);
    assert_eq!(r.score, 62.0);
    assert_eq!(r.band, "check");
}

#[test]
fn chaplain_of_the_well_return_faster_casting_package() {
    // docs/market-3.30.md, chaplain_of_the_well row: Return + Faster Casting
    // on Abyssal Spray is n=8+10 mirror, top-share 0.94, 300d median; under
    // the 0.95 jackpot bar, so the full package is 55 + 12 + 12 = 79, "good".
    let full = merc(
        "chaplain_of_the_well",
        vec![
            Skill::new("Abyssal Spray", "Abyssal Spray").with_supports(vec![
                ("return".into(), SupportTier::T3),
                ("faster_casting".into(), SupportTier::T3),
                ("gmp".into(), SupportTier::T3),
            ]),
            Skill::new("Abyssal Spray Detonation", "Abyssal Spray Detonation"),
            Skill::new("Power of the Depths", "Power of the Depths"),
        ],
    );
    let r = score_mercenary(&full, false);
    assert!(!r.estimate);
    assert!(!r.jackpot);
    assert_eq!(r.score, 79.0);
    assert_eq!(r.band, "good");

    // GMP on Abyssal Spray is not priced (top-share 0.68): it earns nothing.
    let bare = merc(
        "chaplain_of_the_well",
        vec![Skill::new("Abyssal Spray", "Abyssal Spray")
            .with_supports(vec![("gmp".into(), SupportTier::T3)])],
    );
    assert_eq!(score_mercenary(&bare, false).score, 55.0);
}

#[test]
fn unpriced_money_skill_families_fall_back_to_the_generic_estimate() {
    // docs/market-3.30.md, "Generic fallback": the 3.30 survey prices none of
    // these money skills and finds no >= 0.90 replacement. The estimate is
    // capped at the family's rule-6 tier target (high 72, mid 62), so a maxed
    // warrant lands exactly on the cap and never outranks a screened family.
    let maxed = |family: &str| Mercenary {
        infamous: true,
        ..merc(
            family,
            vec![Skill::new("Smite", "Smite").with_supports(vec![
                ("aoe".into(), SupportTier::T3),
                ("cdr".into(), SupportTier::T3),
                ("more_duration".into(), SupportTier::T3),
                ("faster_attacks".into(), SupportTier::T3),
                ("edwa".into(), SupportTier::T3),
            ])],
        )
    };
    for (family, cap) in [
        ("cardinal", 62.0),
        ("withertouch", 72.0),
        ("frosthand", 62.0),
        ("bloodletter", 62.0),
        ("ripper", 62.0),
        ("warpriest", 72.0),
    ] {
        assert!(
            SCREENS.iter().all(|(f, _)| *f != family),
            "{family} still has a screen"
        );
        let r = score_mercenary(&maxed(family), false);
        assert!(r.estimate, "{family}");
        assert_eq!(r.score, cap, "{family}");
        assert_eq!(r.band, interpret_score(cap, family).0, "{family}");
    }
    // No measured tier (0 listings): only the 79 cap applies, and the same
    // warrant estimates 20 + 40 + 10 + 0 demand = 70, above the mid cap.
    let r = score_mercenary(&maxed("warpriest_of_the_ruckus"), false);
    assert_eq!(r.score, 70.0);
}

#[test]
fn ripper_generic_estimate_stays_in_its_mid_tier() {
    // docs/market-3.30.md, ripper row (mid, 5d top-50 median) and "Generic
    // estimate cap": six T2 supports used to estimate ~72 "good"; the mid
    // cap holds it at 62, level with a screened mid family's full package.
    let m = merc(
        "ripper",
        vec![
            Skill::new("Cleave", "Cleave").with_supports(vec![
                ("brutality".into(), SupportTier::T2),
                ("faster_attacks".into(), SupportTier::T2),
            ]),
            Skill::new("Leap Slam", "Leap Slam").with_supports(vec![
                ("pulverise".into(), SupportTier::T2),
                ("brutality".into(), SupportTier::T2),
            ]),
            Skill::new("Molten Shell", "Molten Shell").with_supports(vec![
                ("cdr".into(), SupportTier::T2),
                ("more_duration".into(), SupportTier::T2),
            ]),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.estimate);
    assert_eq!(r.score, 62.0);
    assert_eq!(r.band, "check");
}

#[test]
fn bastion_money_skill_moved_to_prismplate() {
    // docs/market-3.30.md, bastion row: Impenetrable Bastion is a brick
    // (n=25, top-share 0.32, 10c); Prismplate is n=72, top-share 0.96.
    let ib = merc(
        "bastion",
        vec![
            Skill::new("Impenetrable Bastion", "Impenetrable Bastion").with_supports(vec![
                ("cdr".into(), SupportTier::T3),
                ("more_duration".into(), SupportTier::T3),
            ]),
        ],
    );
    assert_eq!(score_mercenary(&ib, false).band, "skip");
    let pp = merc("bastion", vec![Skill::new("Prismplate", "Prismplate")]);
    assert_eq!(score_mercenary(&pp, false).score, 54.0);
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
fn swiftblade_rallying_cry_cdr_duration_is_good_not_jackpot() {
    // docs/market-3.30.md, swiftblade row: RC::CDR + More Duration is a 30d
    // median (n=33, top-share 1.00), under the 50d jackpot bar; high tier, so
    // the full package scales to 25 + 25 + 21 = 71.
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
    assert!(!r.jackpot);
    assert_eq!(r.score, 71.0);
    assert_eq!(r.band, "good");
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
