//! The bundled price snapshot is replaced by the collector, so these tests
//! check structure and package selection, not the numbers.

use poemercpricer::catalog::{all_known_skills, support_icon};
use poemercpricer::models::{kineticist_jackpot_fixture, Mercenary, Skill};
use poemercpricer::pricing::{estimate, satisfies, snapshot, Row};
use poemercpricer::MarketEstimate;

#[test]
fn snapshot_parses_with_rows_and_a_date() {
    let snap = snapshot();
    assert!(!snap.rows.is_empty(), "snapshot has no rows");
    assert!(snap.chaos_per_divine > 0.0);
    assert!(
        snap.generated_at.len() >= 10 && snap.generated_at.as_bytes()[4] == b'-',
        "generated_at is not ISO-8601: {}",
        snap.generated_at
    );
}

#[test]
fn rows_use_lowercase_families_and_catalog_ids() {
    let skills = all_known_skills();
    for row in &snapshot().rows {
        assert_eq!(
            row.family,
            row.family.to_lowercase(),
            "family not lowercase"
        );
        assert!(
            matches!(row.package.as_str(), "base" | "money" | "money_gates"),
            "unknown package {:?} for {}",
            row.package,
            row.family
        );
        if row.package != "base" {
            let skill = row
                .money_skill
                .as_deref()
                .unwrap_or_else(|| panic!("{} {} row has no money_skill", row.family, row.package));
            assert!(
                skills.iter().any(|known| known.eq_ignore_ascii_case(skill)),
                "money_skill {skill:?} is not a known skill"
            );
        }
        for gate in &row.gates {
            assert!(
                support_icon(&gate.canonical).is_some(),
                "gate {:?} on {} is not a catalog support id",
                gate.canonical,
                row.family
            );
            assert!((1..=3).contains(&gate.tier), "gate tier {}", gate.tier);
        }
    }
}

fn chosen_rows(merc: &Mercenary, package: &str) -> Vec<&'static Row> {
    snapshot()
        .rows
        .iter()
        .filter(|row| {
            row.listings > 0
                && row.package == package
                && row.family == merc.family
                && row.infamous.is_none_or(|i| i == merc.infamous)
                && satisfies(merc, row)
        })
        .collect()
}

#[test]
fn fixture_takes_the_most_specific_package_it_satisfies() {
    let (merc, _) = kineticist_jackpot_fixture();
    let Some(market) = estimate(&merc) else {
        assert!(
            chosen_rows(&merc, "base").is_empty(),
            "a base row exists but estimate() returned None"
        );
        return;
    };
    let expected = ["money_gates", "money", "base"]
        .into_iter()
        .find(|package| !chosen_rows(&merc, package).is_empty())
        .expect("estimate() found a row the filter did not");
    assert_eq!(market.package, expected);
    assert!(market.listings > 0);
    assert!(market.low_chaos <= market.high_chaos);
    assert!(!market.snapshot_date.is_empty());
    assert!(market.range_label().starts_with("≈ "));
}

#[test]
fn kineticist_without_the_money_skill_falls_back_to_base() {
    let merc = Mercenary {
        family: "kineticist".into(),
        infamous: false,
        skills: vec![Skill::new("Haste", "Haste")],
        ..Default::default()
    };
    match estimate(&merc) {
        Some(market) => assert_eq!(market.package, "base"),
        None => assert!(chosen_rows(&merc, "base").is_empty()),
    }
}

#[test]
fn unknown_family_has_no_estimate() {
    let merc = Mercenary {
        family: "not-a-family".into(),
        ..Default::default()
    };
    assert!(estimate(&merc).is_none());
}

#[test]
fn bricked_mercenary_is_priced_off_the_family_floor() {
    let (merc, _) = poemercpricer::models::kineticist_jackpot_fixture();
    if let Some(bricked) = poemercpricer::pricing::estimate_for(&merc, true) {
        assert_eq!(bricked.package, "base");
    }
}

#[test]
fn summary_line_carries_the_confidence_caveat() {
    // A thin, stale book must not copy or print as a clean range: the
    // overlay shows a caveat next to the range, so the copied summary and
    // CLI line need it too.
    let thin_and_stale = MarketEstimate {
        package: "base",
        money_skill: None,
        listings: 2,
        low_chaos: 900.0,
        typical_chaos: 1300.0,
        high_chaos: 1800.0,
        median_age_days: 15.0,
        fresh_share: 0.1,
        chaos_per_divine: 200.0,
        league: "Allflame".into(),
        snapshot_date: "2026-09-02".into(),
        placeholder: false,
        min_level: 0,
        scanned: 0,
        passed: 0,
    };
    assert_eq!(thin_and_stale.confidence(), "thin market");
    assert_eq!(
        thin_and_stale.summary_line(),
        "Market: ≈ 4.5–9.0 div (2 listed, snapshot 2026-09-02, thin market)"
    );

    let mut healthy = thin_and_stale.clone();
    healthy.listings = 12;
    healthy.median_age_days = 4.0;
    assert_eq!(healthy.confidence(), "");
    assert_eq!(
        healthy.summary_line(),
        "Market: ≈ 4.5–9.0 div (12 listed, snapshot 2026-09-02)"
    );
}

#[test]
fn rows_without_listings_are_never_chosen() {
    for row in &snapshot().rows {
        let merc = Mercenary {
            family: row.family.clone(),
            infamous: row.infamous.unwrap_or(false),
            ..Default::default()
        };
        if let Some(market) = estimate(&merc) {
            assert!(market.listings > 0);
        }
    }
}
