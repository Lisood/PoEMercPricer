use poemercpricer::catalog::{canonical_skill, compatible_supports_for_art};
use poemercpricer::parse::parse_ocr_lines;
use poemercpricer::scoring::score_mercenary;

fn lines(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn manyshot_screenshot_lines_are_a_brick() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Alara, the Cyaxan Sister",
            "Infamous Manyshot   Lvl 83   Dex",
            "Wager: 8,833",
            "Should Recruit",
            "Ice Shot",
            "Vaal Grace",
            "Icicle Rain",
            "Mirror Arrow",
            "Grace",
            "Blink Arrow",
            "TAKE ITEM",
            "REMATCH",
        ]),
        "ocr",
    );
    assert_eq!(merc.family, "manyshot");
    assert!(merc.infamous);
    assert_eq!(merc.level, Some(83));
    assert!(merc.has_skill(&["Ice Shot"]));
    assert!(merc.has_skill(&["Icicle Rain"]));
    let result = score_mercenary(&merc, false);
    assert_eq!(result.bricks, vec!["Icicle Rain".to_string()]);
    assert!(!result.jackpot);
    assert!(result.score < 50.0);
}

#[test]
fn concatenated_ocr_tokens() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Alara, the Cyaxan Sister",
            "InfamousManyshot",
            "Lvl83",
            "ICE SHOT",
            "VAAL GRACE",
            "lCICLE RAIN",
            "MIRRORARROW",
            "GRACE",
            "BLINKARROW",
        ]),
        "ocr",
    );
    assert_eq!(merc.family, "manyshot");
    assert!(merc.has_skill(&["Ice Shot"]));
    assert!(merc.has_skill(&["Icicle Rain"]));
    assert!(merc.has_skill(&["Mirror Arrow"]));
    assert!(merc.has_skill(&["Blink Arrow"]));
}

#[test]
fn blade_ambusher_resale_is_dump_tier() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Sid, the Slack-Jawed",
            "Blade Ambusher   Lvl 83   Dex / Int",
            "Blade Trap",
            "Trarthan Agility",
            "Spectral Throw of Trarthus",
            "Spectral Helix of Trarthus",
            "Summon Skitterbots",
            "Flame Dash",
        ]),
        "ocr",
    );
    assert_eq!(merc.family, "blade_ambusher");
    assert!(merc.class_name.to_lowercase().contains("blade ambusher"));
    assert!(merc.has_skill(&["Blade Trap"]));
    assert!(merc.has_skill(&["Flame Dash"]));
    let result = score_mercenary(&merc, false);
    // Blade Ambusher resale is dump-tier: no Bear Trap money package here.
    assert_eq!(result.band, "skip");
    assert_ne!(result.band, "unsupported");
}

#[test]
fn bladebitter_header_and_unrelated_exact_skill_never_enter_its_loadout() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Kryxon, the Pale Knife",
            "Bladebitter",
            "Lvl 83",
            "Blade Trap",
            "Viper Strike",
            "Abyssal Cry",
            "Pestilent Strike",
            "Venom Gyre",
            "Malevolence",
            "Whirling Blades",
        ]),
        "ocr",
    );
    assert_eq!(merc.family, "bladebitter");
    assert!(!merc.has_skill(&["Blade Trap"]));
    assert_eq!(
        merc.skills
            .iter()
            .map(|skill| skill.canonical.as_str())
            .collect::<Vec<_>>(),
        [
            "Viper Strike",
            "Abyssal Cry",
            "Pestilent Strike",
            "Venom Gyre",
            "Malevolence",
            "Whirling Blades",
        ]
    );
}

#[test]
fn combatant_family_from_header() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Infamous Combatant",
            "Lvl 83",
            "Frost Blades",
            "Static Strike",
            "Herald of Ice",
        ]),
        "ocr",
    );
    assert_eq!(merc.family, "combatant");
    assert!(merc.has_skill(&["Frost Blades"]));
    assert!(merc.has_skill(&["Static Strike"]));
}

#[test]
fn misread_striker_header_still_classifies() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Vexar, the Brutal",
            "Strlker Lvl 83 Str",
            "Vigilant Strike",
            "Dual Strike",
            "Physical Aegis",
        ]),
        "ocr",
    );
    assert_eq!(merc.class_name, "Striker");
    assert_eq!(merc.family, "striker");
    assert_eq!(merc.level, Some(83));
    assert!(merc.has_skill(&["Dual Strike"]));
}

#[test]
fn current_329_catalog_reads_new_money_skills() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Infamous Cruel Mistress  Lvl 84  Int",
            "Soulrend of Reaping",
            "Dark Bargain",
            "Envy",
            "Zealotry",
        ]),
        "ocr",
    );
    assert_eq!(merc.class_name, "Infamous Cruel Mistress");
    assert_eq!(merc.family, "cruel_mistress");
    assert_eq!(merc.level, Some(84));
    assert!(merc.has_skill(&["Soulrend of Reaping"]));
    assert!(merc.has_skill(&["Dark Bargain"]));
    assert!(merc.has_skill(&["Envy"]));
}

#[test]
fn item_tooltip_requirement_does_not_replace_mercenary_level() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Danalla, the Second",
            "Sanguimancer   Lvl 83   Int",
            "Requires Level 67",
            "Boiling Blood",
        ]),
        "ocr",
    );

    assert_eq!(merc.level, Some(83));
}

#[test]
fn mercenary_level_with_colon_is_read() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Danalla, the Second",
            "Sanguimancer",
            "Mercenary Level: 83",
            "Boiling Blood",
        ]),
        "ocr",
    );
    assert_eq!(merc.level, Some(83));
}

#[test]
fn header_after_twenty_lines_of_chat_is_still_found() {
    let mut rows: Vec<String> = (0..21).map(|i| format!("chat line {i}")).collect();
    rows.push("Infamous Manyshot   Lvl 83   Dex".into());
    rows.push("Ice Shot".into());
    let merc = parse_ocr_lines(&rows, "ocr");
    assert_eq!(merc.family, "manyshot");
    assert_eq!(merc.level, Some(83));
    assert!(merc.has_skill(&["Ice Shot"]));
}

#[test]
fn skill_containing_a_build_name_stays_a_skill() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Bastion   Lvl 83   Str",
            "Impenetrable Bastion",
            "Molten Shell",
        ]),
        "ocr",
    );
    assert_eq!(merc.family, "bastion");
    assert!(merc.has_skill(&["Impenetrable Bastion"]));
}

#[test]
fn name_containing_an_attribute_substring_is_kept() {
    let merc = parse_ocr_lines(
        &lines(&[
            "Corinth, the Bold",
            "Kineticist   Lvl 83   Int",
            "Kinetic Bolt",
        ]),
        "ocr",
    );
    assert_eq!(merc.name, "Corinth, the Bold");
}

#[test]
fn trarthus_variant_stays_distinct_and_accepts_its_two_verified_support_arts() {
    let variant = canonical_skill("WAVE OF CONVICTION OF TRARTHUS").0;
    assert_eq!(variant, "Wave of Conviction of Trarthus");
    assert_ne!(variant, canonical_skill("WAVE OF CONVICTION").0);

    let added_fire = compatible_supports_for_art("addedfiredamage", &variant);
    assert_eq!(
        added_fire.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec!["added_fire"]
    );
    let area = compatible_supports_for_art("increasedaoe", &variant);
    assert_eq!(
        area.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec!["aoe"]
    );
}
