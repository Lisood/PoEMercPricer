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
    assert!(result.bricks.iter().any(|b| b == "Icicle Rain"));
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
