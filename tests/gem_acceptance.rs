//! End-to-end acceptance checks for support-gem recognition.
//!
//! Detecting the gold-framed cells is not enough: every detected support must
//! have a real 3.29 catalog identity and tier.  This fixture previously found
//! all 16 cells while silently returning eight of them as `unknown`.

#[cfg(windows)]
mod windows {
    use std::collections::{HashMap, HashSet};
    use std::time::{Duration, Instant};

    use poemercpricer::scan::scan_rgba;

    fn alara() -> image::RgbaImage {
        image::open("samples/manyshot_alara.png")
            .expect("decode Alara acceptance fixture")
            .to_rgba8()
    }

    fn catalog_supports() -> (HashSet<String>, HashMap<String, String>) {
        let text =
            std::fs::read_to_string("assets/catalog-3.29.json").expect("read bundled 3.29 catalog");
        let catalog: serde_json::Value = serde_json::from_str(&text).expect("valid catalog JSON");
        let entries = catalog["supports"].as_array().expect("supports array");
        let ids = entries
            .iter()
            .map(|entry| entry["canonical"].as_str().expect("support id").to_owned())
            .collect();
        let names = entries
            .iter()
            .map(|entry| {
                (
                    entry["name"]
                        .as_str()
                        .expect("support name")
                        .to_ascii_lowercase(),
                    entry["canonical"].as_str().expect("support id").to_owned(),
                )
            })
            .collect();
        (ids, names)
    }

    fn assert_row(merc: &poemercpricer::Mercenary, skill: &str, expected: &[(&str, u8)]) {
        let actual: Vec<_> = merc
            .skill(&[skill])
            .unwrap_or_else(|| panic!("missing skill {skill}"))
            .supports
            .iter()
            .map(|support| (support.canonical.as_str(), support.tier as u8))
            .collect();
        assert_eq!(actual, expected, "wrong support identity/tier on {skill}");
    }

    #[test]
    fn alara_has_no_missing_or_placeholder_support_gems() {
        let merc = scan_rgba(&alara()).expect("scan Alara fixture");
        let (catalog_ids, catalog_names) = catalog_supports();
        let supports: Vec<_> = merc
            .skills
            .iter()
            .flat_map(|skill| {
                skill
                    .supports
                    .iter()
                    .map(move |support| (skill.canonical.as_str(), support))
            })
            .collect();

        assert_eq!(
            supports.len(),
            16,
            "the fixture contains exactly 16 visible support cells"
        );
        for (skill, support) in supports {
            let id = support.canonical.trim();
            assert!(
                !id.is_empty() && id != "unknown" && id != "gem",
                "{skill} still has a missing support at {}",
                support.raw
            );
            assert!(
                (1..=3).contains(&(support.tier as u8)),
                "{skill}/{id} has invalid tier {:?}",
                support.tier
            );

            if id == "ambiguous" {
                // Minion Damage and Minion Life deliberately use byte-identical
                // art.  A static screenshot cannot honestly pick one.  Preserve
                // both deterministic, catalog-backed candidates instead of
                // inventing a single identity or emitting an unhelpful `gem`.
                assert!(
                    matches!(skill, "Mirror Arrow" | "Blink Arrow"),
                    "unexpected ambiguity on {skill}: {}",
                    support.name
                );
                let candidate_ids: HashSet<_> = support
                    .name
                    .split(" / ")
                    .map(|name| {
                        catalog_names
                            .get(&name.to_ascii_lowercase())
                            .unwrap_or_else(|| panic!("non-catalog ambiguity candidate {name:?}"))
                            .as_str()
                    })
                    .collect();
                assert_eq!(
                    candidate_ids,
                    HashSet::from(["minion_damage", "minion_life"]),
                    "unsupported ambiguity on {skill}: {}",
                    support.name
                );
                continue;
            }
            assert!(
                catalog_ids.contains(id),
                "{skill} produced non-catalog support {id:?} at {}",
                support.raw
            );
        }

        // These identities are visually distinct and therefore must never be
        // weakened to an ambiguity merely to satisfy the no-placeholder gate.
        assert_row(
            &merc,
            "Ice Shot",
            &[
                ("aoe", 2),
                ("cold_penetration", 2),
                ("fork", 3),
                ("hypothermia", 3),
            ],
        );
        assert_row(
            &merc,
            "Icicle Rain",
            &[
                ("aoe", 2),
                ("gmp", 3),
                ("edwa", 2),
                ("fork", 3),
                ("hypothermia", 3),
            ],
        );
        assert_row(
            &merc,
            "Mirror Arrow",
            &[("cdr", 2), ("gmp", 3), ("second_wind", 3), ("ambiguous", 2)],
        );
        assert_row(
            &merc,
            "Blink Arrow",
            &[
                ("more_duration", 2),
                ("faster_attacks", 2),
                ("faster_projectiles", 2),
            ],
        );
    }

    #[test]
    fn alara_scan_stays_within_release_latency_budget() {
        let image = alara();

        // Warm lazy OCR/template initialization so this measures repeat-click
        // latency, which is the path users experience after the first scan.
        scan_rgba(&image).expect("warm Alara scan");
        let started = Instant::now();
        scan_rgba(&image).expect("timed Alara scan");
        let elapsed = started.elapsed();

        let budget = if cfg!(debug_assertions) {
            Duration::from_secs(5)
        } else {
            Duration::from_secs(2)
        };
        assert!(
            elapsed <= budget,
            "warm Alara scan took {elapsed:?}, exceeding {budget:?}"
        );
    }

    #[test]
    fn alara_scaled_dpi_variants_have_no_placeholders() {
        let source = alara();
        for percent in [125_u32, 150] {
            let scaled = image::imageops::resize(
                &source,
                source.width() * percent / 100,
                source.height() * percent / 100,
                image::imageops::FilterType::Nearest,
            );
            let merc = scan_rgba(&scaled)
                .unwrap_or_else(|error| panic!("scan {percent}% DPI fixture: {error:#}"));
            let supports: Vec<_> = merc
                .skills
                .iter()
                .flat_map(|skill| &skill.supports)
                .collect();
            assert_eq!(supports.len(), 16, "{percent}% DPI lost support cells");
            assert!(
                supports.iter().all(|support| {
                    !matches!(support.canonical.as_str(), "" | "unknown" | "gem")
                        && !support.name.starts_with("Unresolved")
                }),
                "{percent}% DPI produced a placeholder: {supports:#?}"
            );
        }
    }
}
