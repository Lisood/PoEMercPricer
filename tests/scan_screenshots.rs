//! Live Windows.Media.Ocr against the real inspect-panel screenshots.
//! These tests are the proof that skill names and gem tiers can be read.

use poemercpricer::scan::scan_rgba;
use poemercpricer::scoring::score_mercenary;
use std::time::{Duration, Instant};

fn load(path: &str) -> image::RgbaImage {
    image::open(path)
        .unwrap_or_else(|e| panic!("open {path}: {e}"))
        .to_rgba8()
}

#[cfg(windows)]
#[test]
fn scan_manyshot_alara_reads_skills_and_gem_tiers() {
    let merc = scan_rgba(&load("samples/manyshot_alara.png")).expect("scan");
    eprintln!(
        "manyshot OCR: level={:?}; skills={:#?}",
        merc.level, merc.skills
    );
    assert_eq!(
        merc.family,
        "manyshot",
        "class={} name={} skills={:?}",
        merc.class_name,
        merc.name,
        merc.skills.iter().map(|s| &s.canonical).collect::<Vec<_>>()
    );
    assert!(merc.infamous);
    assert_eq!(merc.level, Some(83));
    assert_eq!(merc.name, "Alara, the Cyaxan Sister");
    assert!(merc.has_skill(&["Ice Shot"]));
    assert!(merc.has_skill(&["Vaal Grace"]));
    assert!(merc.has_skill(&["Icicle Rain"]));
    assert!(merc.has_skill(&["Mirror Arrow"]));
    assert!(merc.has_skill(&["Blink Arrow"]));
    let ice = merc.skill(&["Ice Shot"]).expect("ice shot");
    assert_eq!(
        merc.skills
            .iter()
            .map(|skill| skill.level)
            .collect::<Vec<_>>(),
        vec![Some(26); 6],
        "level-83 mercenary skills should be level 26"
    );
    assert_eq!(
        ice.supports.len(),
        4,
        "Ice Shot supports: {:?}",
        ice.supports
    );
    assert_eq!(
        ice.supports
            .iter()
            .map(|gem| (gem.canonical.as_str(), gem.tier as u8))
            .collect::<Vec<_>>(),
        vec![
            ("aoe", 2),
            ("cold_penetration", 2),
            ("fork", 3),
            ("hypothermia", 3),
        ]
    );
    assert_eq!(merc.skill(&["Icicle Rain"]).unwrap().supports.len(), 5);
    assert_eq!(merc.skill(&["Mirror Arrow"]).unwrap().supports.len(), 4);
    assert_eq!(merc.skill(&["Blink Arrow"]).unwrap().supports.len(), 3);
    let result = score_mercenary(&merc, false);
    assert!(result.bricks.iter().any(|b| b == "Icicle Rain"));
    assert!(!result.jackpot);
}

#[cfg(windows)]
#[test]
fn scan_blade_ambusher_sid() {
    let merc = scan_rgba(&load("samples/blade_ambusher_sid.png")).expect("scan");
    assert!(
        merc.class_name.to_lowercase().contains("blade ambusher") || merc.family == "other",
        "class={}",
        merc.class_name
    );
    assert_eq!(merc.level, Some(83));
    assert!(merc.has_skill(&["Blade Trap"]) || merc.has_skill(&["Flame Dash"]));
}

#[cfg(windows)]
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

    let rows: Vec<_> = merc
        .skills
        .iter()
        .map(|skill| {
            (
                skill.canonical.as_str(),
                skill
                    .supports
                    .iter()
                    .map(|support| (support.canonical.as_str(), support.tier as u8))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (
                "Boiling Blood",
                vec![
                    ("brutality", 2),
                    ("dot_multiplier", 2),
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

    let query = poemercpricer::trade::trade_query(&merc)
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
    let search = poemercpricer::trade::trade_search(&merc, "Allflame").unwrap();
    assert_eq!(search.selected_skill, "Storm Call of Trarthus");
    assert_eq!(search.included_skills, 1);
    assert_eq!(search.available_skills, 6);
    assert_eq!(search.included_filters, 6);
    assert_eq!(search.available_filters, 22);
    assert!(search
        .url
        .starts_with("https://www.pathofexile.com/trade/search/Allflame?q="));
}

#[cfg(windows)]
#[test]
fn scan_storming_zealot_orvan() {
    let merc = scan_rgba(&load("samples/storming_zealot_orvan.jpg")).expect("scan");
    assert!(
        merc.class_name.to_lowercase().contains("storming"),
        "class={}",
        merc.class_name
    );
    assert!(merc.has_skill(&["Flame Dash"]) || merc.has_skill(&["Wrath"]));
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
fn cleanup_ocr_infamous_and_lvl() {
    assert_eq!(
        poemercpricer::scan::cleanup_ocr("Z:lnfamous Manyshot"),
        "Infamous Manyshot"
    );
    assert_eq!(poemercpricer::scan::cleanup_ocr("LVI 83"), "Lvl 83");
}
