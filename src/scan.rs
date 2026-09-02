use anyhow::Result;
use image::RgbaImage;
use std::sync::OnceLock;

use crate::catalog::{all_known_skills, canonical_support, normalize, support_display_for_tier};
use crate::models::{infer_mercenary_skill_level, Mercenary, Skill};
use crate::parse::parse_ocr_lines;
use crate::vision::{crop, guess_panel, supports_for_skill_row};
use crate::winocr::{recognize_lines, OcrLine};

pub fn scan_rgba(img: &RgbaImage) -> Result<Mercenary> {
    let started = std::time::Instant::now();
    let (ocr_image, offset_x, offset_y) = skill_text_region(img);
    let raw_lines = recognize_lines(&ocr_image)?;
    let after_ocr = std::time::Instant::now();
    let cleaned: Vec<OcrLine> = raw_lines
        .into_iter()
        .map(|mut l| {
            l.x += offset_x;
            l.y += offset_y;
            l.text = cleanup_ocr(&l.text);
            l
        })
        .filter(|l| !l.text.is_empty())
        .collect();
    // Several legitimate skill names wrap onto a second line in the in-game
    // panel (for example `WAVE OF CONVICTION OF` + `TRARTHUS`).  Parsing each
    // OCR line independently silently collapses those to the shorter base
    // skill, which in turn selects the wrong support compatibility pool.
    // Merge only geometrically-adjacent lines that resolve decisively to a
    // bundled catalog entry. Union both line bounds so support-icon scanning
    // covers the complete wrapped row rather than clipping its lower half.
    let cleaned = merge_wrapped_skill_lines(&cleaned);
    let texts: Vec<String> = cleaned.iter().map(|l| l.text.clone()).collect();
    // Equipment tooltips can overlap a live mercenary panel.  Their
    // "Requires Level 67" line must not replace the mercenary's header level.
    // Keep the positioned OCR line for geometry/debugging, but exclude this
    // unambiguously unrelated text from semantic parsing.
    let parse_texts: Vec<String> = texts
        .iter()
        .filter(|text| !normalize(text).contains("requires level"))
        .cloned()
        .collect();
    let mut merc = parse_ocr_lines(&parse_texts, "ocr");
    let inferred_skill_level = merc.level.and_then(infer_mercenary_skill_level);
    let panel = guess_panel(img, &cleaned);
    let skill_band = mercenary_skill_band(&cleaned);

    // Parsing is intentionally tolerant of OCR damage, but a fuzzy catalog
    // match is not enough on its own. When the panel controls are visible,
    // require every active skill to originate between the Wager/Recruit row
    // and the Take Item/Rematch row. This rejects header and flavour-text
    // collisions without imposing fixed screen coordinates or DPI assumptions.
    merc.skills.retain(|skill| {
        best_line_for_skill(&cleaned, skill).is_some_and(|line| match skill_band {
            Some(band) => line_is_in_skill_band(line, band),
            None => true,
        })
    });

    for skill in merc.skills.iter_mut() {
        let skill_started = std::time::Instant::now();
        if let Some(line) = best_line_for_skill(&cleaned, skill) {
            skill.raw = line.text.clone();
            skill.level = parse_nearby_level(&line.text).or(inferred_skill_level);
            skill.supports = supports_for_skill_row(img, line, &skill.canonical, panel);
        }
        if std::env::var_os("POEMERC_PROFILE_OCR").is_some() {
            eprintln!(
                "vision {}: {:.1} ms",
                skill.canonical,
                skill_started.elapsed().as_secs_f64() * 1_000.0
            );
        }
    }
    // Generic mercenary support gems deliberately share artwork. If the user
    // is already hovering one of those gems, reuse its visible tooltip title
    // from the OCR pass above to resolve that otherwise irreducible collision.
    // This adds no OCR/capture work and never guesses when the title could
    // refer to more than one ambiguous cell.
    apply_visible_support_tooltip_hints(&mut merc, &texts);
    if merc.notes.is_empty() {
        merc.notes.push(format!(
            "OCR {} lines; support tiers from icon roman numerals I/II/III",
            texts.len()
        ));
    }
    if std::env::var_os("POEMERC_PROFILE_OCR").is_some() {
        eprintln!(
            "scan total: {:.1} ms (OCR {:.1} ms)",
            started.elapsed().as_secs_f64() * 1_000.0,
            after_ocr.duration_since(started).as_secs_f64() * 1_000.0
        );
    }
    Ok(merc)
}

fn apply_visible_support_tooltip_hints(merc: &mut Mercenary, ocr_texts: &[String]) {
    #[derive(Debug)]
    struct Hint {
        canonical: String,
    }

    let mut hints = Vec::<Hint>::new();
    for skill in &merc.skills {
        for support in &skill.supports {
            if support.canonical != "ambiguous" {
                continue;
            }
            for candidate in support.name.split(" / ").map(str::trim) {
                if candidate.is_empty()
                    || !ocr_texts
                        .iter()
                        .any(|text| is_exact_tooltip_title(text, candidate))
                {
                    continue;
                }
                let (canonical, confidence) = canonical_support(candidate);
                if confidence < 0.99 || canonical.is_empty() {
                    continue;
                }
                if !hints.iter().any(|hint| hint.canonical == canonical) {
                    hints.push(Hint { canonical });
                }
            }
        }
    }

    let mut resolutions = Vec::<(usize, usize, String)>::new();
    for (skill_index, skill) in merc.skills.iter().enumerate() {
        for (support_index, support) in skill.supports.iter().enumerate() {
            if support.canonical != "ambiguous" {
                continue;
            }
            let matching_hints: Vec<_> = hints
                .iter()
                .filter(|hint| {
                    support.name.split(" / ").map(str::trim).any(|candidate| {
                        canonical_support(candidate)
                            .0
                            .eq_ignore_ascii_case(&hint.canonical)
                    })
                })
                .collect();
            if matching_hints.len() != 1 {
                continue;
            }
            let hint = matching_hints[0];
            let eligible_count = merc
                .skills
                .iter()
                .flat_map(|skill| &skill.supports)
                .filter(|candidate_support| {
                    candidate_support.canonical == "ambiguous"
                        && candidate_support
                            .name
                            .split(" / ")
                            .map(str::trim)
                            .any(|candidate| {
                                canonical_support(candidate)
                                    .0
                                    .eq_ignore_ascii_case(&hint.canonical)
                            })
                })
                .count();
            if eligible_count == 1 {
                resolutions.push((skill_index, support_index, hint.canonical.clone()));
            }
        }
    }

    for (skill_index, support_index, canonical) in resolutions {
        let support = &mut merc.skills[skill_index].supports[support_index];
        support.canonical.clone_from(&canonical);
        support.name = support_display_for_tier(&canonical, support.tier as u8);
        support.confidence = support.confidence.max(0.99);
        support.raw.push_str("; exact visible tooltip title");
    }
}

fn is_exact_tooltip_title(ocr_text: &str, candidate: &str) -> bool {
    let normalized = normalize(ocr_text);
    let candidate = normalize(candidate);
    normalized == candidate
        || ["lesser", "greater", "gilded"]
            .into_iter()
            .any(|prefix| normalized == format!("{prefix} {candidate}"))
}

fn skill_text_region(img: &RgbaImage) -> (RgbaImage, u32, u32) {
    let width = img.width();
    if width < 1400 {
        let text_width = (width * 3 / 4).max(360).min(width);
        return (crop(img, 0, 0, text_width, img.height()), 0, 0);
    }

    // Full-screen captures place the inspect panel in the middle. OCR three
    // quarters of that panel: wide enough for centered names without including
    // the surrounding world HUD. Support art is read from the original pixels.
    let panel_width = width * 42 / 100;
    let panel_x = (width - panel_width) / 2;
    let text_width = panel_width * 3 / 4;
    (crop(img, panel_x, 0, text_width, img.height()), panel_x, 0)
}

fn best_line_for_skill<'a>(lines: &'a [OcrLine], skill: &Skill) -> Option<&'a OcrLine> {
    let target = normalize(&skill.canonical);
    let mut best: Option<(&OcrLine, f32)> = None;
    for ln in lines {
        let s = strsim::jaro_winkler(&normalize(&ln.text), &target) as f32;
        if s >= 0.86 && best.as_ref().map(|(_, b)| s > *b).unwrap_or(true) {
            best = Some((ln, s));
        }
    }
    best.map(|(l, _)| l)
}

fn mercenary_skill_band(lines: &[OcrLine]) -> Option<(u32, u32)> {
    let top = lines
        .iter()
        .filter(|line| {
            let normalized = normalize(&line.text);
            normalized.contains("wager") || normalized.starts_with("should recr")
        })
        .map(|line| line.y.saturating_add(line.h))
        .max()?;
    let bottom = lines
        .iter()
        .filter(|line| {
            let normalized = normalize(&line.text);
            normalized.contains("take item") || normalized.contains("rematch")
        })
        .map(|line| line.y)
        .min()?;
    (bottom > top.saturating_add(20)).then_some((top, bottom))
}

fn line_is_in_skill_band(line: &OcrLine, band: (u32, u32)) -> bool {
    let center_y = line.y.saturating_add(line.h / 2);
    center_y > band.0 && center_y < band.1
}

fn merge_wrapped_skill_lines(lines: &[OcrLine]) -> Vec<OcrLine> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut consumed = vec![false; lines.len()];

    for index in 0..lines.len() {
        if consumed[index] {
            continue;
        }
        let current = &lines[index];
        let mut best_pair: Option<(usize, &OcrLine, &OcrLine, String, u32)> = None;

        // Windows OCR normally emits reading order, but may insert a nearby
        // level/tooltip fragment between the two halves of a wrapped label.
        // Match by geometry instead of depending on vector adjacency.
        for (other_index, other) in lines.iter().enumerate() {
            if other_index == index || consumed[other_index] {
                continue;
            }
            let (top, bottom) = if current.y <= other.y {
                (current, other)
            } else {
                (other, current)
            };
            if !wrapped_lines_are_adjacent(top, bottom) {
                continue;
            }
            if let Some(canonical) = catalog_wrapped_skill(&top.text, &bottom.text) {
                let vertical_gap = bottom.y.saturating_sub(top.y.saturating_add(top.h));
                let left_drift = top.x.abs_diff(bottom.x);
                let distance = vertical_gap.saturating_mul(4).saturating_add(left_drift);
                if best_pair
                    .as_ref()
                    .map(|(_, _, _, _, best_distance)| distance < *best_distance)
                    .unwrap_or(true)
                {
                    best_pair = Some((other_index, top, bottom, canonical, distance));
                }
            }
        }

        if let Some((other_index, top, bottom, canonical, _)) = best_pair {
            let mut line = top.clone();
            line.text = canonical;
            let left = top.x.min(bottom.x);
            let upper = top.y.min(bottom.y);
            let right = top
                .x
                .saturating_add(top.w)
                .max(bottom.x.saturating_add(bottom.w));
            let lower = top
                .y
                .saturating_add(top.h)
                .max(bottom.y.saturating_add(bottom.h));
            line.x = left;
            line.y = upper;
            line.w = right.saturating_sub(left);
            line.h = lower.saturating_sub(upper);
            merged.push(line);
            consumed[index] = true;
            consumed[other_index] = true;
            continue;
        }

        merged.push(current.clone());
        consumed[index] = true;
    }
    merged
}

fn wrapped_lines_are_adjacent(top: &OcrLine, bottom: &OcrLine) -> bool {
    if bottom.y <= top.y {
        return false;
    }
    let line_height = top.h.max(bottom.h).max(1);
    let vertical_gap = bottom.y.saturating_sub(top.y.saturating_add(top.h));
    let top_right = top.x.saturating_add(top.w);
    let bottom_right = bottom.x.saturating_add(bottom.w);
    let horizontal_gap = bottom
        .x
        .saturating_sub(top_right)
        .max(top.x.saturating_sub(bottom_right));
    vertical_gap <= line_height.saturating_mul(2) && horizontal_gap <= line_height.max(8)
}

/// Recognize a two-line skill only when their combined text has a decisive
/// match in the bundled 3.29 catalog.  The margin prevents unrelated adjacent
/// skill rows from being joined through a merely fuzzy match.
fn catalog_wrapped_skill(top: &str, bottom: &str) -> Option<String> {
    let top = normalize(top);
    let bottom = normalize(bottom);
    if top.is_empty() || bottom.is_empty() {
        return None;
    }
    let combined = format!("{top} {bottom}");
    let mut best: Option<(&str, f32)> = None;
    let mut runner = 0.0_f32;
    for (candidate, normalized) in normalized_skill_catalog() {
        // A wrapped candidate must add information to both component lines;
        // this excludes ordinary neighbouring complete skills.
        if normalized.len() <= top.len() || normalized.len() <= bottom.len() {
            continue;
        }
        let score = strsim::jaro_winkler(&combined, normalized) as f32;
        if score < 0.94 {
            continue;
        }
        if best.map(|(_, value)| score > value).unwrap_or(true) {
            runner = best.map(|(_, value)| value).unwrap_or(runner);
            best = Some((*candidate, score));
        } else {
            runner = runner.max(score);
        }
    }
    let (candidate, score) = best?;
    (score - runner >= 0.025).then(|| candidate.to_string())
}

fn normalized_skill_catalog() -> &'static [(&'static str, String)] {
    static CATALOG: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        all_known_skills()
            .into_iter()
            .map(|skill| (skill, normalize(skill)))
            .collect()
    })
}

fn parse_nearby_level(text: &str) -> Option<u32> {
    let re = regex::Regex::new(r"(?i)\b(?:lv|lvl|level)\s*([1-9]|1[0-9]|2[0-6])\b").unwrap();
    re.captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

pub fn cleanup_ocr(text: &str) -> String {
    let mut t = text.replace('`', "'");
    t = t.replace("lnfamous", "Infamous");
    t = t.replace("Lnfamous", "Infamous");
    t = t.replace("LVI", "Lvl");
    t = t.replace("lvi", "Lvl");
    t = regex::Regex::new(r"(?i)^z:\s*")
        .unwrap()
        .replace(&t, "")
        .into_owned();
    t.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SupportGem, SupportTier};

    const MERCENARY_SKILL_ROW_Y: [u32; 6] = [640, 704, 768, 832, 896, 960];

    #[derive(Clone, Copy, Debug)]
    enum ContinuationAlignment {
        Left,
        Centre,
        Right,
    }

    fn wrapped_line_pair(
        top: &str,
        bottom: &str,
        row_y: u32,
        alignment: ContinuationAlignment,
    ) -> [OcrLine; 2] {
        const LEFT: u32 = 820;
        const HEIGHT: u32 = 15;
        const LINE_GAP: u32 = 8;

        // Segoe/Fontin OCR boxes in the screenshots average roughly 8 px per
        // character.  Put both fragments within one common text column, then
        // exercise the three alignments produced by Windows OCR for wrapped
        // labels of different widths.
        let top_width = (top.chars().count() as u32 * 8).max(24);
        let bottom_width = (bottom.chars().count() as u32 * 8).max(24);
        let column_width = top_width.max(bottom_width);
        let aligned_x = |width: u32| match alignment {
            ContinuationAlignment::Left => LEFT,
            ContinuationAlignment::Centre => LEFT + (column_width - width) / 2,
            ContinuationAlignment::Right => LEFT + column_width - width,
        };

        [
            OcrLine {
                text: top.into(),
                x: aligned_x(top_width),
                y: row_y,
                w: top_width,
                h: HEIGHT,
            },
            OcrLine {
                text: bottom.into(),
                x: aligned_x(bottom_width),
                y: row_y + HEIGHT + LINE_GAP,
                w: bottom_width,
                h: HEIGHT,
            },
        ]
    }

    fn holy_flame_totem_with_ambiguous_silver_support() -> Mercenary {
        let mut skill = Skill::new("Holy Flame Totem", "Holy Flame Totem");
        skill.supports.push(SupportGem {
            name: "Ironwood / Physical as Extra".into(),
            canonical: "ambiguous".into(),
            tier: SupportTier::T2,
            confidence: 0.82,
            raw: "icon@100,200 T2".into(),
        });
        Mercenary {
            class_name: "Flaming Charlatan".into(),
            family: "flaming_charlatan".into(),
            skills: vec![skill],
            ..Default::default()
        }
    }

    #[test]
    fn exact_visible_tooltip_resolves_holy_flame_totem_shared_art() {
        for title in ["PHYSICAL AS EXTRA", "GREATER PHYSICAL AS EXTRA"] {
            let mut merc = holy_flame_totem_with_ambiguous_silver_support();
            apply_visible_support_tooltip_hints(&mut merc, &[title.into()]);

            let support = &merc.skills[0].supports[0];
            assert_eq!(support.canonical, "physical_as_extra");
            assert_eq!(support.name, "Physical as Extra");
            assert_eq!(support.tier, SupportTier::T2);
            assert!(support.confidence >= 0.99);
            assert!(support.raw.contains("exact visible tooltip title"));
        }
    }

    #[test]
    fn tooltip_hint_never_guesses_from_descriptions_or_longer_support_names() {
        for unrelated in [
            "Gain 20% of Physical Damage as Extra Damage of a random Element",
            "GREATER PHYSICAL AS EXTRA CHAOS",
            "Supported Skills gain Physical Damage as Extra Fire Damage",
        ] {
            let mut merc = holy_flame_totem_with_ambiguous_silver_support();
            apply_visible_support_tooltip_hints(&mut merc, &[unrelated.into()]);
            assert_eq!(merc.skills[0].supports[0].canonical, "ambiguous");
            assert_eq!(
                merc.skills[0].supports[0].name,
                "Ironwood / Physical as Extra"
            );
        }
    }

    #[test]
    fn tooltip_hint_preserves_ambiguity_when_title_or_target_is_not_unique() {
        let mut both_titles = holy_flame_totem_with_ambiguous_silver_support();
        apply_visible_support_tooltip_hints(
            &mut both_titles,
            &["IRONWOOD".into(), "PHYSICAL AS EXTRA".into()],
        );
        assert_eq!(both_titles.skills[0].supports[0].canonical, "ambiguous");

        let mut duplicate_targets = holy_flame_totem_with_ambiguous_silver_support();
        let duplicate = duplicate_targets.skills[0].supports[0].clone();
        duplicate_targets.skills[0].supports.push(duplicate);
        apply_visible_support_tooltip_hints(&mut duplicate_targets, &["PHYSICAL AS EXTRA".into()]);
        assert!(duplicate_targets.skills[0]
            .supports
            .iter()
            .all(|support| support.canonical == "ambiguous"));
    }

    #[test]
    fn infers_endgame_mercenary_skill_levels() {
        assert_eq!(infer_mercenary_skill_level(68), Some(19));
        assert_eq!(infer_mercenary_skill_level(82), Some(26));
        assert_eq!(infer_mercenary_skill_level(83), Some(26));
        assert_eq!(infer_mercenary_skill_level(84), Some(27));
        assert_eq!(infer_mercenary_skill_level(67), Some(18));
        assert_eq!(infer_mercenary_skill_level(65), None);
    }

    #[test]
    fn ocr_region_is_bounded_to_the_skill_text_column() {
        let cropped = RgbaImage::new(850, 1189);
        let (region, x, y) = skill_text_region(&cropped);
        assert_eq!((region.width(), region.height(), x, y), (637, 1189, 0, 0));

        let fullscreen = RgbaImage::new(2000, 1123);
        let (region, x, y) = skill_text_region(&fullscreen);
        assert_eq!((region.width(), region.height()), (630, 1123));
        assert_eq!((x, y), (580, 0));
    }

    #[test]
    fn panel_control_band_rejects_header_text_that_fuzzily_matches_a_skill() {
        let lines = [
            OcrLine {
                text: "Bladebitter".into(),
                x: 1118,
                y: 158,
                w: 79,
                h: 14,
            },
            OcrLine {
                text: "Wager: 5,691".into(),
                x: 944,
                y: 279,
                w: 136,
                h: 25,
            },
            OcrLine {
                text: "VIPER STRIKE".into(),
                x: 989,
                y: 814,
                w: 110,
                h: 15,
            },
            OcrLine {
                text: "TAKE ITEM".into(),
                x: 1123,
                y: 1175,
                w: 105,
                h: 15,
            },
        ];
        let band = mercenary_skill_band(&lines).expect("panel controls define skill band");
        assert!(!line_is_in_skill_band(&lines[0], band));
        assert!(line_is_in_skill_band(&lines[2], band));
    }

    #[test]
    fn merges_every_catalog_prefix_variant_when_ocr_wraps_it() {
        let cases = [
            ("Barrage of", "Volley Fire", "Barrage of Volley Fire"),
            ("Bladefall of", "Trarthus", "Bladefall of Trarthus"),
            ("Fireball of", "Impact", "Fireball of Impact"),
            ("Ice Nova of", "Projection", "Ice Nova of Projection"),
            (
                "Leap Slam of",
                "Groundbreaking",
                "Leap Slam of Groundbreaking",
            ),
            ("Lightning Warp", "Trap", "Lightning Warp Trap"),
            (
                "Spectral Helix of",
                "Trarthus",
                "Spectral Helix of Trarthus",
            ),
            (
                "WAVE OF CONVICTION OF",
                "TRARTHUS",
                "Wave of Conviction of Trarthus",
            ),
        ];

        for (top, bottom, expected) in cases {
            let lines = [
                OcrLine {
                    text: top.into(),
                    x: 989,
                    y: 919,
                    w: 214,
                    h: 15,
                },
                OcrLine {
                    text: bottom.into(),
                    x: 989,
                    y: 942,
                    w: 86,
                    h: 15,
                },
            ];
            let result = merge_wrapped_skill_lines(&lines);
            assert_eq!(result.len(), 1, "{expected}");
            assert_eq!(result[0].text, expected);
            assert_eq!(
                (result[0].x, result[0].y, result[0].w, result[0].h),
                (989, 919, 214, 38)
            );
        }
    }

    #[test]
    fn merges_wrapped_skills_in_every_slot_when_ocr_order_is_interleaved() {
        for row_y in MERCENARY_SKILL_ROW_Y {
            let [top, bottom] = wrapped_line_pair(
                "Wave of Conviction of",
                "Trarthus",
                row_y,
                ContinuationAlignment::Left,
            );
            let unrelated = OcrLine {
                text: "Lvl 26".into(),
                x: 1_240,
                y: row_y + 10,
                w: 48,
                h: 15,
            };

            // Windows OCR can return a right-column fragment between the two
            // visual lines, or even report the continuation before its prefix.
            for lines in [
                vec![top.clone(), unrelated.clone(), bottom.clone()],
                vec![bottom.clone(), unrelated.clone(), top.clone()],
            ] {
                let result = merge_wrapped_skill_lines(&lines);
                let skill = result
                    .iter()
                    .find(|line| line.text == "Wave of Conviction of Trarthus")
                    .unwrap_or_else(|| panic!("missed interleaved wrap at y={row_y}"));
                assert_eq!((skill.y, skill.h), (row_y, 38));
                assert_eq!(result.len(), 2);
                assert!(result.iter().any(|line| line.text == "Lvl 26"));
            }
        }
    }

    #[test]
    fn does_not_merge_distinct_skill_rows_or_distant_text() {
        let separate_skills = [
            OcrLine {
                text: "Holy Flame Totem".into(),
                x: 92,
                y: 700,
                w: 180,
                h: 15,
            },
            OcrLine {
                text: "Purifying Flame".into(),
                x: 92,
                y: 723,
                w: 160,
                h: 15,
            },
        ];
        assert_eq!(merge_wrapped_skill_lines(&separate_skills).len(), 2);

        let distant_wrap = [
            OcrLine {
                text: "Wave of Conviction of".into(),
                x: 92,
                y: 700,
                w: 220,
                h: 15,
            },
            OcrLine {
                text: "Trarthus".into(),
                x: 92,
                y: 780,
                w: 80,
                h: 15,
            },
        ];
        assert_eq!(merge_wrapped_skill_lines(&distant_wrap).len(), 2);
    }

    #[test]
    fn reconstructs_every_multiword_catalog_skill_at_every_word_boundary_and_row() {
        let alignments = [
            ContinuationAlignment::Left,
            ContinuationAlignment::Centre,
            ContinuationAlignment::Right,
        ];
        let mut cases = 0_usize;

        for skill in all_known_skills() {
            let words: Vec<&str> = skill.split_whitespace().collect();
            for boundary in 1..words.len() {
                let top = words[..boundary].join(" ");
                let bottom = words[boundary..].join(" ");
                for row_y in MERCENARY_SKILL_ROW_Y {
                    for alignment in alignments {
                        let lines = wrapped_line_pair(&top, &bottom, row_y, alignment);
                        let result = merge_wrapped_skill_lines(&lines);
                        assert_eq!(
                            result.len(),
                            1,
                            "failed to merge {skill:?} at boundary {boundary}, y={row_y}, {alignment:?}"
                        );
                        assert_eq!(
                            result[0].text, skill,
                            "wrong skill for {skill:?} at boundary {boundary}, y={row_y}, {alignment:?}"
                        );

                        let expected_left = lines[0].x.min(lines[1].x);
                        let expected_right = (lines[0].x + lines[0].w).max(lines[1].x + lines[1].w);
                        assert_eq!(result[0].x, expected_left);
                        assert_eq!(result[0].w, expected_right - expected_left);
                        assert_eq!(result[0].y, row_y);
                        assert_eq!(result[0].h, 38);
                        cases += 1;
                    }
                }
            }
        }

        // This is intentionally a broad generated matrix.  If the catalog is
        // accidentally narrowed, keep this regression from silently becoming
        // a handful of hand-picked examples.
        assert!(cases >= 6_000, "only exercised {cases} wrapped-name cases");
    }

    #[test]
    fn never_merges_wrapped_fragments_across_mercenary_skill_rows() {
        let mut cases = 0_usize;

        for skill in all_known_skills() {
            let words: Vec<&str> = skill.split_whitespace().collect();
            for boundary in 1..words.len() {
                let top = words[..boundary].join(" ");
                let bottom = words[boundary..].join(" ");
                for rows in MERCENARY_SKILL_ROW_Y.windows(2) {
                    let mut lines =
                        wrapped_line_pair(&top, &bottom, rows[0], ContinuationAlignment::Left);
                    // Put the apparent continuation at the next skill-row
                    // origin. Even when its words would complete an exact
                    // catalog name, geometry must keep the two rows separate.
                    lines[1].y = rows[1];
                    let result = merge_wrapped_skill_lines(&lines);
                    assert_eq!(
                        result.len(),
                        2,
                        "cross-row merge for {skill:?} at boundary {boundary}, y={} -> {}",
                        rows[0],
                        rows[1]
                    );
                    assert_eq!(result[0].text, top);
                    assert_eq!(result[1].text, bottom);
                    cases += 1;
                }
            }
        }

        assert!(cases >= 1_600, "only exercised {cases} cross-row cases");
    }
}
