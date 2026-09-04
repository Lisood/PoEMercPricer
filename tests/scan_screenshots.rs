//! Live Windows.Media.Ocr against the real inspect-panel screenshots.
//! These tests are the proof that skill names and gem tiers can be read.
#![cfg(windows)]

use poemercpricer::scan::scan_rgba;
use std::time::{Duration, Instant};

fn load(path: &str) -> image::RgbaImage {
    image::open(path)
        .unwrap_or_else(|e| panic!("open {path}: {e}"))
        .to_rgba8()
}

fn rows(merc: &poemercpricer::Mercenary) -> Vec<(&str, Vec<(&str, u8)>)> {
    merc.skills
        .iter()
        .map(|skill| {
            (
                skill.canonical.as_str(),
                skill
                    .supports
                    .iter()
                    .map(|support| (support.canonical.as_str(), support.tier as u8))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn scan_blade_ambusher_sid() {
    let merc = scan_rgba(&load("samples/blade_ambusher_sid.png")).expect("scan");
    assert_eq!(merc.class_name, "Blade Ambusher");
    assert_eq!(merc.family, "blade_ambusher");
    assert_eq!(merc.name, "Sid, the Slack-Jawed");
    assert_eq!(merc.level, Some(83));
    assert!(merc.skills.iter().all(|skill| skill.level == Some(26)));
    assert_eq!(
        rows(&merc),
        vec![
            (
                "Blade Trap",
                vec![
                    ("concentrated_effect", 3),
                    ("aoe", 2),
                    ("crit_damage", 3),
                    ("multiple_traps", 3),
                ]
            ),
            ("Trarthan Agility", vec![("more_duration", 2), ("aoe", 2)]),
            (
                "Spectral Throw of Trarthus",
                vec![
                    ("faster_projectiles", 2),
                    // Throwing Speed and Trigger Radius share one generic
                    // silver art. Two cells with that art in one row means
                    // both are socketed; only their order is unknown.
                    ("throwing_speed", 3),
                    ("charged_traps", 3),
                    ("trigger_radius", 3),
                    ("gmp", 3),
                ]
            ),
            (
                "Spectral Helix of Trarthus",
                vec![
                    ("slower_projectiles", 2),
                    ("trap_and_mine_damage", 3),
                    ("ambiguous", 3),
                    ("ambiguous", 3),
                    ("ambiguous", 3),
                ]
            ),
            ("Summon Skitterbots", vec![]),
            ("Flame Dash", vec![("more_duration", 2), ("cdr", 3)]),
        ]
    );
}

#[test]
fn scan_bladebitter_kryxon_rejects_class_header_false_positive() {
    let merc = scan_rgba(&load("samples/kryxon_bladebitter.png")).expect("scan");
    assert_eq!(merc.family, "bladebitter");
    assert_eq!(merc.name, "Kryxon, the Pale Knife");
    assert!(!merc.has_skill(&["Blade Trap"]));
    assert_eq!(
        merc.skills
            .iter()
            .map(|skill| (skill.canonical.as_str(), skill.supports.len()))
            .collect::<Vec<_>>(),
        [
            ("Viper Strike", 5),
            ("Abyssal Cry", 2),
            ("Pestilent Strike", 5),
            ("Venom Gyre", 5),
            ("Malevolence", 0),
            ("Whirling Blades", 2),
        ]
    );
}

#[test]
fn scan_sanguimancer_grynelle_reads_every_visible_support_cell() {
    let merc = scan_rgba(&load("samples/grynelle_sanguimancer.png"))
        .expect("scan full-screen Grynnelle capture");
    assert_eq!(merc.name, "Grynnelle, the Fifth");
    assert_eq!(merc.class_name, "Sanguimancer");
    assert_eq!(merc.family, "sanguimancer");
    assert_eq!(merc.level, Some(83));

    assert_eq!(
        rows(&merc),
        vec![
            (
                "Boiling Blood",
                vec![
                    ("brutality", 2),
                    // Green ring-and-hook gem: Swift Affliction. Nearest
                    // neighbour resampling scored it below the floor.
                    ("swift_affliction", 2),
                    ("more_duration", 3)
                ]
            ),
            (
                "Storm Call of Trarthus",
                vec![
                    ("aoe", 2),
                    ("concentrated_effect", 3),
                    ("more_duration", 3),
                    ("dot_multiplier", 3),
                    ("faster_casting", 3),
                ]
            ),
            ("Bodyswap", vec![("faster_casting", 2), ("aoe", 2)]),
            (
                "Reap",
                vec![
                    ("faster_casting", 2),
                    ("physical_as_extra_chaos", 3),
                    ("dot_multiplier", 2),
                    ("more_duration", 2),
                ]
            ),
            ("Proximity Shield", vec![("more_duration", 2), ("cdr", 2)]),
            ("Pride", vec![]),
        ],
        "every visible support icon must be preserved in left-to-right order"
    );

    let query = poemercpricer::trade::trade_query(&merc, false)
        .expect("every scanned skill and support maps to an official trade stat");
    let groups = query["query"]["stats"].as_array().unwrap();
    assert_eq!(groups.len(), 1, "anonymous search permits one exact group");
    assert_eq!(groups[0]["filters"].as_array().unwrap().len(), 6);
    assert_eq!(
        groups[0]["filters"][0]["id"], "mercenary.skill_2187",
        "the most-supported skill is selected"
    );
    assert_eq!(query["query"]["type"]["option"], "MiscScionPhysDot");
    assert_eq!(
        query["query"]["filters"]["misc_filters"]["filters"]["ilvl"]["min"],
        83
    );
    let search = poemercpricer::trade::trade_search(&merc, "Allflame", false).unwrap();
    assert_eq!(search.selected_skill, "Storm Call of Trarthus");
    assert_eq!(search.included_skills, 1);
    assert_eq!(search.available_skills, 6);
    assert_eq!(search.included_filters, 6);
    assert_eq!(search.available_filters, 22);
    assert!(search
        .url
        .starts_with("https://www.pathofexile.com/trade/search/Allflame?q="));
}

#[test]
fn scan_storming_zealot_orvan() {
    let merc = scan_rgba(&load("samples/storming_zealot_orvan.jpg")).expect("scan");
    assert_eq!(merc.class_name, "Storming Zealot");
    assert_eq!(merc.family, "storming_zealot");
    assert_eq!(merc.name, "Orvan, the Keitan Convert");
    assert_eq!(merc.level, Some(83));
    assert!(merc.skills.iter().all(|skill| skill.level == Some(26)));
    assert_eq!(
        rows(&merc),
        vec![
            (
                "Unnerving Blast",
                vec![
                    ("faster_casting", 2),
                    ("aoe", 3),
                    ("physical_as_extra", 2),
                    ("crit_chance", 3),
                    ("lightning_penetration", 2),
                ]
            ),
            (
                "Wave of Conviction",
                vec![("second_wind", 3), ("physical_as_extra", 2)]
            ),
            (
                "Divine Retribution",
                vec![
                    ("crit_chance", 3),
                    ("faster_casting", 2),
                    // Shares art with Cooldown Recovery, which the row already
                    // holds exactly in the next cell.
                    ("shock_chance", 2),
                    ("cdr", 3),
                ]
            ),
            (
                "Divine Ire",
                vec![
                    ("physical_as_extra", 2),
                    // Blue ring-and-hook gem: Infused Channelling.
                    ("infused_channelling", 3),
                    ("gilded_beam_width", 3),
                    ("faster_casting", 3),
                ]
            ),
            ("Flame Dash", vec![("more_duration", 2), ("cdr", 3)]),
            ("Wrath", vec![]),
        ]
    );
}

#[test]
fn fullscreen_scan_ignores_world_hud_and_loot_icons() {
    let merc = scan_rgba(&load("samples/fullscreen_danalla.jpg")).expect("scan fullscreen Danalla");
    assert_eq!(merc.family, "sanguimancer");
    // The adversarial fixture has an equipment tooltip reading
    // "Requires Level 67" over a level-83 mercenary panel.
    assert_eq!(merc.name, "Danalla, the Second");
    assert_eq!(
        merc.level,
        Some(83),
        "equipment requirements are not mercenary levels"
    );
    let counts: Vec<_> = merc
        .skills
        .iter()
        .map(|skill| (skill.canonical.as_str(), skill.supports.len()))
        .collect();
    assert_eq!(
        counts,
        vec![
            ("Boiling Blood", 2),
            ("Storm Call of Trarthus", 5),
            ("Bodyswap", 2),
            ("Vaal Reap", 5),
            ("Flame Dash", 2),
            ("Vaal Vitality", 2),
        ],
        "support detection must stop at the mercenary panel edge"
    );
    assert!(merc
        .skills
        .iter()
        .flat_map(|skill| &skill.supports)
        .all(|support| {
            support
                .raw
                .strip_prefix("icon@")
                .and_then(|raw| raw.split(',').next())
                .and_then(|x| x.parse::<u32>().ok())
                .is_some_and(|x| x < 1300)
        }));
}

#[test]
fn adversarial_fullscreen_preserves_unobstructed_support_semantics() {
    // The equipment tooltip makes this unsuitable as a clean whole-panel
    // reference. Its lower skill rows are unobstructed, however, so compare
    // only those semantics with the clean cropped fixture.
    let cropped =
        scan_rgba(&load("samples/sanguimancer_danalla.jpg")).expect("scan cropped Danalla");
    let fullscreen =
        scan_rgba(&load("samples/fullscreen_danalla.jpg")).expect("scan fullscreen Danalla");
    assert_eq!(fullscreen.skills.len(), cropped.skills.len());
    for (full_skill, cropped_skill) in fullscreen.skills.iter().zip(&cropped.skills) {
        assert_eq!(full_skill.canonical, cropped_skill.canonical);
        assert_eq!(full_skill.supports.len(), cropped_skill.supports.len());
        for (full, cropped) in full_skill.supports.iter().zip(&cropped_skill.supports) {
            assert_eq!(
                full.tier, cropped.tier,
                "tier mismatch on {}",
                full_skill.canonical
            );
            let compatible_identity = full.canonical == cropped.canonical
                || (full.canonical == "ambiguous"
                    && full
                        .name
                        .contains(poemercpricer::catalog::support_display(&cropped.canonical)))
                || (cropped.canonical == "ambiguous"
                    && cropped
                        .name
                        .contains(poemercpricer::catalog::support_display(&full.canonical)));
            assert!(
                compatible_identity,
                "identity mismatch on {}: fullscreen={full:?}, cropped={cropped:?}",
                full_skill.canonical
            );
        }
    }
}

#[test]
fn cropped_danalla_fixture_remains_exact_without_fullscreen_adversaries() {
    let merc =
        scan_rgba(&load("samples/sanguimancer_danalla.jpg")).expect("scan cropped Danalla panel");
    assert_eq!(merc.name, "Danalla, the Second");
    assert_eq!(merc.class_name, "Sanguimancer");
    assert_eq!(merc.family, "sanguimancer");
    assert_eq!(merc.level, Some(83));

    let rows: Vec<_> = merc
        .skills
        .iter()
        .map(|skill| {
            (
                skill.canonical.as_str(),
                skill.level,
                skill.supports.len(),
                skill
                    .supports
                    .iter()
                    .map(|support| support.tier as u8)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("Boiling Blood", Some(26), 2, vec![2, 2]),
            ("Storm Call of Trarthus", Some(26), 5, vec![3, 3, 2, 2, 3]),
            ("Bodyswap", Some(26), 2, vec![2, 2]),
            ("Vaal Reap", Some(26), 5, vec![2, 2, 2, 3, 2]),
            ("Flame Dash", Some(26), 2, vec![2, 2]),
            ("Vaal Vitality", Some(26), 2, vec![2, 2]),
        ],
        "cropped/clipboard-style recognition regressed"
    );
    assert!(merc
        .skills
        .iter()
        .flat_map(|skill| &skill.supports)
        .all(
            |support| !matches!(support.canonical.as_str(), "" | "unknown" | "gem")
                && !support.name.starts_with("Unresolved")
        ));
}

#[test]
#[ignore = "wall-clock budget flakes on shared CI runners; run with --release -- --ignored"]
fn warm_fullscreen_scan_stays_within_interactive_latency_budget() {
    let image = load("samples/fullscreen_danalla.jpg");

    // Windows OCR and template caches are initialized on the first call. The
    // second call is the repeat-Scan experience reported by the UI.
    scan_rgba(&image).expect("warm fullscreen scan");
    let started = Instant::now();
    scan_rgba(&image).expect("timed fullscreen scan");
    let elapsed = started.elapsed();

    let budget = if cfg!(debug_assertions) {
        Duration::from_secs(8)
    } else {
        Duration::from_secs(1)
    };
    assert!(
        elapsed <= budget,
        "warm fullscreen scan took {elapsed:?}, exceeding the {budget:?} interaction budget"
    );
    eprintln!("warm fullscreen scan: {elapsed:?} (budget {budget:?})");
}

#[test]
fn scan_bladebitter_kestel_keeps_low_gold_return_cell() {
    let merc = scan_rgba(&load("samples/kestel_bladebitter.png")).expect("scan");
    assert_eq!(merc.name, "Kestel, the Azadin Agent");
    assert_eq!(merc.class_name, "Infamous Bladebitter");
    // Return's boomerang gem carries almost no gold, so a pure gold-area
    // threshold dropped the cell and the row shrank to four supports.
    assert_eq!(
        rows(&merc),
        vec![
            (
                "Viper Strike",
                vec![
                    ("multistrike", 2),
                    ("dot_multiplier", 3),
                    ("chance_to_poison", 3),
                    ("more_duration", 3),
                    ("strike_distance", 2),
                ]
            ),
            ("Abyssal Cry", vec![("cdr", 2), ("aoe", 2)]),
            (
                "Cobra Lash",
                vec![
                    ("chance_to_poison", 3),
                    ("return", 3),
                    ("added_chaos", 2),
                    ("wither_on_hit", 2),
                    ("physical_as_extra_chaos", 3),
                ]
            ),
            (
                "Profane Strike",
                vec![
                    ("dot_multiplier", 2),
                    ("ailment_damage", 2),
                    ("faster_attacks", 3),
                    ("chance_to_poison", 3),
                ]
            ),
            (
                "Whirling Blades",
                vec![("brutality", 2), ("added_chaos", 2)]
            ),
            ("Dash", vec![]),
        ]
    );
}
