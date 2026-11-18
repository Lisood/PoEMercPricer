use poemercpricer::parse::parse_warrant_text;
use poemercpricer::scoring::score_mercenary;

const SAMPLE: &str = r#"
Item Class: Map Fragments
Rarity: Normal
Mercenary Warrant
--------
Build: Infamous Kineticist
Mercenary Level: 83
--------
Kinetic Blast of Clustering
Return (Tier: 3)
Greater Multiple Projectiles (Tier: 3)
Chain (Tier: 2)
Elemental Damage with Attacks (Tier: 3)
--------
Greater Kinetic Blast
--------
Haste
"#;

#[test]
fn manyshot_warrant_with_icicle_rain_is_not_jackpot() {
    let text = r#"
Item Class: Map Fragments
Rarity: Normal
Mercenary Warrant
--------
Build: Infamous Manyshot
Mercenary Level: 83
--------
Ice Shot
Greater Multiple Projectiles (Tier: 3)
Return (Tier: 3)
--------
Vaal Ice Shot
Return (Tier: 3)
--------
Icicle Rain
Pierce (Tier: 2)
"#;
    let merc = parse_warrant_text(text);
    assert_eq!(merc.family, "manyshot");
    assert!(merc.has_skill(&["Ice Shot"]));
    assert!(merc.has_skill(&["Icicle Rain"]));
    let result = score_mercenary(&merc, false);
    // docs/market-3.30.md "VIS + Icicle Rain" (n=14+2, 175d, top-share 0.88): no brick;
    // Ice Shot 18 + VIS 18 + VIS Return 18 + Ice Return 8 + GMP 10, under the premium cap 79.
    assert!(result.bricks.is_empty(), "{:?}", result.bricks);
    assert!((result.score - 72.0).abs() < 0.01, "{}", result.score);
    assert!(result
        .highlights
        .iter()
        .any(|h| h == "Icicle Rain: no Vaal Ice Shot + Mirror Arrow jackpot package"));
    assert!(!result.jackpot);
}

#[test]
fn parse_kineticist_warrant() {
    let merc = parse_warrant_text(SAMPLE);
    assert_eq!(merc.family, "kineticist");
    assert!(merc.infamous);
    assert_eq!(merc.level, Some(83));
    assert!(merc.has_skill(&["Kinetic Blast of Clustering"]));
    let kboc = merc.skill(&["Kinetic Blast of Clustering"]).unwrap();
    assert_eq!(kboc.level, Some(26));
    assert!(merc.skills.iter().all(|skill| skill.level == Some(26)));
    assert!((kboc.t(&["return"]) - 1.0).abs() < f32::EPSILON);
    assert!((kboc.t(&["gmp"]) - 1.0).abs() < f32::EPSILON);
    assert!((kboc.t(&["chain"]) - 0.8).abs() < f32::EPSILON);
    assert!(merc.has_skill(&["Haste"]));
    let result = score_mercenary(&merc, false);
    assert_eq!(result.score.round() as i32, 88);
    assert!(result.jackpot);
}

fn skill_names(text: &str) -> Vec<String> {
    parse_warrant_text(text)
        .skills
        .iter()
        .map(|skill| skill.canonical.clone())
        .collect()
}

const REAL_SKILLS: [&str; 3] = [
    "Kinetic Blast of Clustering",
    "Greater Kinetic Blast",
    "Haste",
];

#[test]
fn trailing_note_and_corrupted_sections_are_not_skills() {
    let text = format!("{SAMPLE}--------\nCorrupted\n--------\nNote: ~b/o 5 divine\n");
    assert_eq!(skill_names(&text), REAL_SKILLS);
}

#[test]
fn separator_with_trailing_space_still_splits_sections() {
    let text = SAMPLE.replace("--------\n", "-------- \n");
    assert_eq!(skill_names(&text), REAL_SKILLS);
}

#[test]
fn doubled_separator_does_not_drop_the_next_skill() {
    let text = SAMPLE.replace("--------\nHaste", "--------\n--------\nHaste");
    assert_eq!(skill_names(&text), REAL_SKILLS);
}

#[test]
fn missing_build_line_still_parses_skills() {
    let text = SAMPLE.replace("Build: Infamous Kineticist\n", "");
    let merc = parse_warrant_text(&text);
    assert_eq!(merc.family, "other");
    assert_eq!(merc.level, Some(83));
    assert_eq!(skill_names(&text), REAL_SKILLS);
}

#[test]
fn looks_like_warrant_needs_more_than_a_build_line() {
    use poemercpricer::parse::looks_like_warrant;
    assert!(looks_like_warrant(SAMPLE));
    assert!(looks_like_warrant("Build: Sniper\nMercenary Level: 80"));
    assert!(!looks_like_warrant("Build: release\ncargo build --release"));
}

#[test]
fn looks_like_warrant_rejects_oversized_text_before_matching() {
    use poemercpricer::parse::looks_like_warrant;
    let huge = "Build: Sniper Mercenary Level: 80 ".repeat(3000);
    assert!(huge.len() > 64 * 1024);
    assert!(!looks_like_warrant(&huge));
}

#[test]
fn level_84_warrant_has_level_27_active_skills() {
    let merc = parse_warrant_text(&SAMPLE.replace("Mercenary Level: 83", "Mercenary Level: 84"));
    assert_eq!(merc.level, Some(84));
    assert!(merc.skills.iter().all(|skill| skill.level == Some(27)));
}

fn skills_and_supports(text: &str) -> Vec<(String, Vec<String>)> {
    parse_warrant_text(text)
        .skills
        .iter()
        .map(|skill| {
            let supports = skill.supports.iter().map(|s| s.canonical.clone()).collect();
            (skill.canonical.clone(), supports)
        })
        .collect()
}

const PROPERTIES_330: &str = "Limit: 1\nPortal: The Proving Grounds\nArea Level: 83\nItem Tier: 3";

#[test]
fn plain_329_warrant_parses_to_the_known_skills_and_supports() {
    let parsed = skills_and_supports(SAMPLE);
    assert_eq!(
        parsed.iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(),
        REAL_SKILLS
    );
    assert_eq!(parsed[0].1, ["return", "gmp", "chain", "edwa"]);
    assert!(parsed[1].1.is_empty() && parsed[2].1.is_empty());
}

#[test]
fn warrant_330_property_lines_before_build_are_ignored() {
    let expected = skills_and_supports(SAMPLE);
    // Inside the header section, and as their own section before Build.
    let in_header = SAMPLE.replace(
        "Mercenary Warrant\n",
        &format!("Mercenary Warrant\n{PROPERTIES_330}\n"),
    );
    let own_section = SAMPLE.replace(
        "--------\nBuild:",
        &format!("--------\n{PROPERTIES_330}\n--------\nBuild:"),
    );
    let header_after_properties = SAMPLE.replace(
        "Item Class: Map Fragments\nRarity: Normal\nMercenary Warrant\n",
        &format!("{PROPERTIES_330}\nItem Class: Map Fragments\nRarity: Normal\n"),
    );
    for text in [in_header, own_section, header_after_properties] {
        assert_eq!(skills_and_supports(&text), expected, "{text}");
        assert_eq!(parse_warrant_text(&text).level, Some(83));
    }
}

#[test]
fn warrant_330_property_section_after_build_is_ignored() {
    let text = SAMPLE.replace(
        "Mercenary Level: 83\n--------\n",
        &format!("Mercenary Level: 83\n--------\n{PROPERTIES_330}\n--------\n"),
    );
    assert_eq!(skills_and_supports(&text), skills_and_supports(SAMPLE));
    let trailing = format!("{SAMPLE}--------\n{PROPERTIES_330}\n");
    assert_eq!(skills_and_supports(&trailing), skills_and_supports(SAMPLE));
}

#[test]
fn warrant_330_property_lines_inside_a_skill_section_are_not_supports() {
    let after_name = SAMPLE.replace(
        "Kinetic Blast of Clustering\n",
        &format!("Kinetic Blast of Clustering\n{PROPERTIES_330}\n"),
    );
    let after_supports = SAMPLE.replace(
        "Chain (Tier: 2)\n",
        &format!("Chain (Tier: 2)\n{PROPERTIES_330}\n"),
    );
    let before_name = SAMPLE.replace(
        "--------\nHaste",
        &format!("--------\n{PROPERTIES_330}\nHaste"),
    );
    for text in [after_name, after_supports, before_name] {
        assert_eq!(
            skills_and_supports(&text),
            skills_and_supports(SAMPLE),
            "{text}"
        );
    }
}

/// A real 3.30 Ctrl+C copy, decoded from a trade listing's `extended.text`,
/// with the mercenary renamed. The source elided Smite's last three supports
/// and the two utility rows; those are filled in from a 3.30 Chaplain of the
/// Well trade listing's structured skills, not from the copied text.
const CHAPLAIN_330: &str = r#"Item Class: Map Fragments
Rarity: Normal
Mercenary Warrant
--------
Orlan, the Unlit
--------
Build: Chaplain of the Well
Mercenary Level: 83
--------
Abyssal Spray
Faster Casting (Tier: 2)
Greater Pierce (Tier: 3)
Physical as Extra Chaos (Tier: 2)
Greater Multiple Projectiles (Tier: 3)
--------
Abyssal Spray Detonation
Critical Chance (Tier: 2)
Greater Area of Effect (Tier: 3)
--------
Power of the Depths
--------
Smite
Faster Attacks (Tier: 2)
Increased Area of Effect (Tier: 2)
Generosity (Tier: 2)
Elemental Damage with Attacks (Tier: 2)
--------
Abyssal Cry
Warcry Speed (Tier: 2)
Cooldown Recovery (Tier: 2)
--------
Flame Dash
Greater Cooldown Recovery (Tier: 3)
Faster Casting (Tier: 2)
--------
Right click this item to view Mercenary details.
Open a portal to the Proving Grounds by using this item in a personal Map Device. This previously fought Mercenary will reappear in the area for a rematch. While viewing your Atlas, Right-click this item to pinpoint its location.
--------
Note: ~b/o 99999999 divine
"#;

#[test]
fn real_330_chaplain_copy_parses_every_skill_support_and_tier() {
    let merc = parse_warrant_text(CHAPLAIN_330);
    assert_eq!(merc.class_name, "Chaplain of the Well");
    assert_eq!(merc.family, "chaplain_of_the_well");
    assert!(!merc.infamous);
    assert_eq!(merc.level, Some(83));
    let parsed: Vec<(String, Vec<(String, u8)>)> = merc
        .skills
        .iter()
        .map(|skill| {
            let supports = skill
                .supports
                .iter()
                .map(|s| (s.name.clone(), s.tier as u8))
                .collect();
            (skill.canonical.clone(), supports)
        })
        .collect();
    let expected: Vec<(&str, Vec<(&str, u8)>)> = vec![
        (
            "Abyssal Spray",
            vec![
                ("Faster Casting", 2),
                ("Greater Pierce", 3),
                ("Physical as Extra Chaos", 2),
                ("Greater Multiple Projectiles", 3),
            ],
        ),
        (
            "Abyssal Spray Detonation",
            vec![("Critical Chance", 2), ("Greater Area of Effect", 3)],
        ),
        ("Power of the Depths", vec![]),
        (
            "Smite",
            vec![
                ("Faster Attacks", 2),
                ("Increased Area of Effect", 2),
                ("Generosity", 2),
                ("Elemental Damage with Attacks", 2),
            ],
        ),
        (
            "Abyssal Cry",
            vec![("Warcry Speed", 2), ("Cooldown Recovery", 2)],
        ),
        (
            "Flame Dash",
            vec![("Greater Cooldown Recovery", 3), ("Faster Casting", 2)],
        ),
    ];
    let expected: Vec<(String, Vec<(String, u8)>)> = expected
        .into_iter()
        .map(|(skill, supports)| {
            let supports = supports
                .into_iter()
                .map(|(name, tier)| (name.to_string(), tier))
                .collect();
            (skill.to_string(), supports)
        })
        .collect();
    assert_eq!(parsed, expected);
    // Every support resolved to a catalog identity, not a low-confidence guess.
    assert!(merc
        .skills
        .iter()
        .flat_map(|skill| &skill.supports)
        .all(|support| support.confidence >= 1.0));
}

#[test]
fn clipboard_skill_levels_are_estimates_and_shown_with_a_tilde() {
    let mut merc = parse_warrant_text(SAMPLE);
    assert!(merc
        .skills
        .iter()
        .all(|skill| skill.level == Some(26) && skill.level_estimated));
    let result = score_mercenary(&merc, false);
    let text = poemercpricer::summary::summary(&merc, &result, None);
    assert!(
        text.contains("Kinetic Blast of Clustering Lv~26:"),
        "{text}"
    );

    // A level read off the panel is printed as is.
    for skill in &mut merc.skills {
        skill.level_estimated = false;
    }
    let text = poemercpricer::summary::summary(&merc, &result, None);
    assert!(text.contains("Kinetic Blast of Clustering Lv26:"), "{text}");
    assert!(!text.contains('~'), "{text}");

    let unlevelled = parse_warrant_text(&SAMPLE.replace("Mercenary Level: 83\n", ""));
    assert!(unlevelled
        .skills
        .iter()
        .all(|skill| skill.level.is_none() && !skill.level_estimated));
}
