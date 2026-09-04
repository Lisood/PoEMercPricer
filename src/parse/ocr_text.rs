use regex::Regex;
use std::sync::LazyLock;

use crate::catalog::{
    all_known_skills, canonical_skill, classify_class, family_skills, known_build_names, normalize,
};
use crate::models::{Mercenary, Skill};

static LEVEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:lvl|lvi|lv|level)\s*:?\s*(\d{1,3})\b").unwrap());

pub fn parse_ocr_lines(lines: &[String], source: &str) -> Mercenary {
    let cleaned: Vec<String> = lines
        .iter()
        .map(|ln| ln.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|ln| !ln.is_empty())
        .collect();

    let mut name = String::new();
    let mut raw_class = String::new();
    let mut level = None;
    let mut level_priority = 0_u8;
    let mut attributes = String::new();
    // Padded so a build name only matches as whole tokens.
    let normalized_build_names: Vec<String> = known_build_names()
        .map(|build| format!(" {} ", normalize(build)))
        .collect();

    for ln in &cleaned {
        let n = normalize(ln);
        if name.is_empty() && ln.contains(',') && ln.len() < 60 && !n.contains("wager") {
            if ln
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                continue;
            }
            if !n.split(' ').any(|token| {
                matches!(
                    token,
                    "lvl" | "dex" | "str" | "int" | "wager" | "recruit" | "house"
                )
            }) {
                name = ln.clone();
            }
        }
        // Item tooltips can overlap the mercenary panel and contain text such as
        // "Requires Level 67". That is an equipment requirement, never the
        // mercenary's level. Prefer the compact `Lvl` header used by the panel,
        // while retaining `Mercenary Level` as a lower-priority OCR fallback.
        let is_item_requirement = n.contains("requires level") || n.contains("required level");
        let candidate_priority = if n.contains("lvl") {
            2
        } else if n.contains("mercenary level") {
            1
        } else {
            0
        };
        if !is_item_requirement && candidate_priority > level_priority {
            if let Some(c) = LEVEL_RE.captures(ln) {
                level = c.get(1).and_then(|m| m.as_str().parse().ok());
                if level.is_some() {
                    level_priority = candidate_priority;
                }
            }
        }
        if (n.contains("str") || n.contains("dex") || n.contains("int")) && n.contains("lvl")
            || matches!(
                n.as_str(),
                "dex" | "int" | "str" | "str / int" | "dex / int" | "str / dex / int"
            )
        {
            attributes = ln.clone();
        }
        if raw_class.is_empty() {
            let padded = format!(" {n} ");
            if normalized_build_names
                .iter()
                .any(|build| padded.contains(build.as_str()))
            {
                raw_class = ln.clone();
            }
        }
    }

    // No exact build token found: the class shares the `Lvl` header line, so
    // let classify_class fuzz that line (it ignores a header with no class).
    if raw_class.is_empty() && normalize(&attributes).contains("lvl") {
        raw_class = attributes.clone();
    }
    let (display, family, infamous) = classify_class(&raw_class);
    let preferred = family_skills(&family);
    // A known family has a finite, audited 3.29 skill pool. Do not merely rank
    // those skills ahead of the global catalog: that still lets unrelated
    // fuzzy matches through (for example `Bladebitter` -> `Blade Trap`).
    let catalog: Vec<&str> = if preferred.is_empty() {
        all_known_skills()
    } else {
        preferred.to_vec()
    };

    let normalized_catalog: Vec<(&str, String)> = catalog
        .into_iter()
        .map(|skill| (skill, normalize(skill)))
        .collect();
    let mut skills = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let semantic_headers: std::collections::HashSet<String> = [&name, &raw_class, &attributes]
        .into_iter()
        .filter(|header| !header.is_empty())
        .map(|header| normalize(header))
        .collect();
    for ln in &cleaned {
        let n = normalize(ln);
        if n.is_empty()
            || semantic_headers.contains(&n)
            || n.contains("wager")
            || matches!(n.as_str(), "take item" | "rematch" | "should recruit")
        {
            continue;
        }
        let mut best: Option<(&str, f32)> = None;
        for (cand, normalized_candidate) in &normalized_catalog {
            let s = strsim::jaro_winkler(&n, normalized_candidate) as f32;
            if s >= 0.86 && best.as_ref().map(|(_, b)| s > *b).unwrap_or(true) {
                best = Some((cand, s));
            }
        }
        let Some((matched, score)) = best else {
            continue;
        };
        let (canon, _) = canonical_skill(matched);
        if canon.is_empty() || seen.contains(&canon.to_lowercase()) {
            continue;
        }
        // A lone fragment ("Projection") earns a prefix bonus against an
        // unrelated short skill ("Precision"); demand a near-exact read.
        if score < 0.90 && (n.len() > matched.len() + 10 || !n.contains(' ')) {
            continue;
        }
        seen.insert(canon.to_lowercase());
        skills.push(Skill {
            name: ln.clone(),
            canonical: canon,
            supports: Vec::new(),
            level: None,
            confidence: score,
            raw: ln.clone(),
        });
    }

    Mercenary {
        name,
        class_name: if display.is_empty() {
            raw_class
        } else {
            display
        },
        family,
        level,
        attributes,
        infamous,
        skills,
        source: source.into(),
        notes: Vec::new(),
    }
}
