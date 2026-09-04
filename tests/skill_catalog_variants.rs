//! 3.29.3 mercenary active-skill catalog acceptance checks.
//!
//! Source: https://poedb.tw/us/Mercenaries (the current mercenary build table).
//! The separate Trarthan-gem reward list also contains Dark Bargain of
//! Trarthus, but no current mercenary build displays that player gem in its
//! skill rows, so it is deliberately outside this screen-scanner catalog.

use poemercpricer::catalog::{all_known_skills, canonical_skill, skill_icon};
use std::collections::BTreeSet;
use std::path::Path;

const DISPLAYED_TRARTHUS_SKILLS: [&str; 11] = [
    "Bladefall of Trarthus",
    "Blast Rain of Trarthus",
    "Chain Hook of Trarthus",
    "Heavy Strike of Trarthus",
    "Siege Ballista of Trarthus",
    "Spectral Helix of Trarthus",
    "Spectral Shield Throw of Trarthus",
    "Spectral Throw of Trarthus",
    "Storm Call of Trarthus",
    "Sunder of Trarthus",
    "Wave of Conviction of Trarthus",
];

/// Names most likely to occupy two OCR lines or collide with a shorter base
/// skill.  These cover Trarthan gems, transfigured skills, house-flavoured
/// skills, traps, and the longest bespoke mercenary skills in the 3.29 table.
const WRAP_AND_COLLISION_RISKS: [&str; 35] = [
    "Ball Lightning of Orbiting Trap",
    "Ball Lightning of Static",
    "Barrage of Volley Fire",
    "Bladefall of Trarthus",
    "Blast Rain of Trarthus",
    "Chain Hook of Trarthus",
    "Charged Dash of the Arcane",
    "Corrupted Blade Vortex of the Scythe",
    "Cyclone of the Empire",
    "Earthquake of Amplification",
    "Essence Drain of Wickedness",
    "Fireball of Impact",
    "Heavy Strike of Trarthus",
    "Heavy Strike of Vulnerability",
    "Ice Nova of Projection",
    "Infernal Blow of Immolation",
    "Kinetic Blast of Clustering",
    "Kinetic Rain of Impact",
    "Leap Slam of Groundbreaking",
    "Rain of Arrows of Saturation",
    "Raise Spectre: Holy Flame Elementals",
    "Raise Spectre of Transience",
    "Raise Zombie of Gigantism",
    "Shockwave Totem of Shocking",
    "Siege Ballista of Trarthus",
    "Soulrend of Reaping",
    "Spectral Helix of Trarthus",
    "Spectral Shield Throw of Trarthus",
    "Spectral Throw of Trarthus",
    "Storm Call of Trarthus",
    "Sunder of Trarthus",
    "Tornado of Elemental Turbulence",
    "Volcanic Fissure of Snaking",
    "Wave of Conviction of Trarthus",
    "Reinforce: Fallen Osseotitan",
];

fn catalog_skill_names() -> BTreeSet<String> {
    let catalog_text =
        std::fs::read_to_string("assets/catalog-3.29.json").expect("read bundled 3.29 catalog");
    let catalog: serde_json::Value =
        serde_json::from_str(&catalog_text).expect("valid catalog JSON");
    catalog["skills"]
        .as_array()
        .expect("skills array")
        .iter()
        .map(|entry| entry["name"].as_str().expect("skill name").to_owned())
        .collect()
}

#[test]
fn current_329_mercenary_loadouts_have_the_complete_trarthus_census() {
    let names = catalog_skill_names();
    let actual: BTreeSet<_> = names
        .iter()
        .filter(|name| name.ends_with(" of Trarthus"))
        .map(String::as_str)
        .collect();
    let expected: BTreeSet<_> = DISPLAYED_TRARTHUS_SKILLS.into_iter().collect();

    assert_eq!(
        actual, expected,
        "bundled catalog drifted from the 3.29 mercenary loadout census"
    );
}

#[test]
fn every_trarthus_and_long_variant_is_canonical_and_has_local_art() {
    let known: BTreeSet<_> = all_known_skills().into_iter().collect();
    for name in DISPLAYED_TRARTHUS_SKILLS
        .into_iter()
        .chain(WRAP_AND_COLLISION_RISKS)
    {
        assert!(known.contains(name), "missing active skill {name:?}");
        assert_eq!(
            canonical_skill(name),
            (name.to_owned(), 1.0),
            "{name:?} must not collapse into a shorter base skill"
        );

        let icon = skill_icon(name).unwrap_or_else(|| panic!("missing icon mapping for {name:?}"));
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/icons/skills")
            .join(icon);
        assert!(path.is_file(), "missing local icon for {name:?}: {path:?}");
    }
}

#[test]
fn similarly_named_base_skills_remain_distinct_from_their_variants() {
    const PAIRS: [(&str, &str); 8] = [
        ("Bladefall", "Bladefall of Trarthus"),
        ("Fireball", "Fireball of Impact"),
        ("Ice Nova", "Ice Nova of Projection"),
        ("Leap Slam", "Leap Slam of Groundbreaking"),
        ("Spectral Helix", "Spectral Helix of Trarthus"),
        ("Wave of Conviction", "Wave of Conviction of Trarthus"),
        ("Barrage", "Barrage of Volley Fire"),
        ("Lightning Warp", "Lightning Warp Trap"),
    ];

    for (base, variant) in PAIRS {
        let base_canonical = canonical_skill(base).0;
        let variant_canonical = canonical_skill(variant).0;
        assert_eq!(base_canonical, base);
        assert_eq!(variant_canonical, variant);
        assert_ne!(base_canonical, variant_canonical);
    }
}

#[test]
fn house_names_are_not_invented_as_skill_suffixes() {
    let names = catalog_skill_names();
    for house in ["Azadi", "Bardiya", "Cyaxan", "Keita"] {
        assert!(
            names
                .iter()
                .all(|name| !name.ends_with(&format!(" of {house}"))),
            "3.29 mercenary skills use bespoke house abilities, not an 'of {house}' suffix"
        );
    }
}
