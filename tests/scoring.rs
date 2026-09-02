use poemercpricer::models::{Mercenary, Skill, SupportTier};
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
                ("cdr".into(), SupportTier::T3),
            ]),
            Skill::new("Mirror Arrow", "Mirror Arrow"),
            Skill::new("Hatred", "Hatred"),
        ],
    );
    let r = score_mercenary(&m, false);
    assert!(r.jackpot);
    assert!(r.score >= 80.0);
}

#[test]
fn combatant_frost_jackpot() {
    let m = merc(
        "combatant",
        vec![
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
fn sniper_tornado_gmp_is_jackpot() {
    let m = merc(
        "sniper",
        vec![Skill::new("Tornado Shot", "Tornado Shot")
            .with_supports(vec![("gmp".into(), SupportTier::T3)])],
    );
    let r = score_mercenary(&m, false);
    assert!(r.jackpot);
    assert_eq!(r.band, "jackpot");
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
fn thunderquiver_return_gmp_package_is_jackpot() {
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
    assert!(r.jackpot);
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
