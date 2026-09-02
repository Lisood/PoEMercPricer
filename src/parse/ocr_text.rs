use regex::Regex;

use crate::catalog::{
    all_known_skills, canonical_skill, classify_class, family_skills, known_build_names, normalize,
};
use crate::models::{Mercenary, Skill};

const CLASS_HINTS: &[&str] = &[
    "kineticist",
    "manyshot",
    "combatant",
    "sniper",
    "thunderquiver",
    "stormhand",
    "blade ambusher",
    "frosthand",
    "storming zealot",
    "sanguimancer",
];

pub fn parse_ocr_lines(lines: &[String], source: &str) -> Mercenary {
    let cleaned: Vec<String> = lines
        .iter()
        .map(|ln| {
            Regex::new(r"\s+")
                .unwrap()
                .replace_all(ln, " ")
                .trim()
                .to_string()
        })
        .filter(|ln| !ln.is_empty())
        .collect();

    let mut name = String::new();
    let mut raw_class = String::new();
    let mut level = None;
    let mut level_priority = 0_u8;
    let mut attributes = String::new();
    let level_re = Regex::new(r"(?i)\b(?:lvl|lvi|lv|level)\s*(\d{1,3})\b").unwrap();
    let normalized_build_names: Vec<String> = known_build_names().map(normalize).collect();

    for ln in cleaned.iter().take(20) {
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
            if !["lvl", "dex", "str", "int", "wager", "recruit", "house"]
                .iter()
                .any(|h| n.contains(h))
            {
                name = ln.clone();
            }
        }
        // Item tooltips can overlap the mercenary panel and contain text such as
        // "Requires Level 67". That is an equipment requirement, never the
        // mercenary's level. Prefer the compact `Lvl` header used by the panel,
        // while retaining `Mercenary Level` as a lower-priority OCR fallback.
        let is_item_requirement = n.contains("requires level") || n.contains("required level");
        let candidate_priority = if n.contains("lvl") || n.contains("lvi") {
            2
        } else if n.contains("mercenary level") {
            1
        } else {
            0
        };
        if !is_item_requirement && candidate_priority > level_priority {
            if let Some(c) = level_re.captures(ln) {
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
        if CLASS_HINTS.iter().any(|hint| n.contains(hint))
            || normalized_build_names.iter().any(|build| n.contains(build))
        {
            raw_class = ln.clone();
        }
    }

    let (display, family, infamous) = classify_class(&raw_class);
    let preferred = family_skills(&family);
    // A known family has a finite, audited 3.29 skill pool. Do not merely rank
    // those skills ahead of the global catalog: that still lets unrelated
    // fuzzy matches through (for example `Bladebitter` -> `Blade Trap`).
    let catalog: Vec<String> = if preferred.is_empty() {
        all_known_skills().into_iter().map(str::to_owned).collect()
    } else {
        preferred.iter().map(|skill| (*skill).to_owned()).collect()
    };

    let normalized_catalog: Vec<(String, String)> = catalog
        .into_iter()
        .map(|skill| {
            let normalized = normalize(&skill);
            (skill, normalized)
        })
        .collect();
    let mut skills = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let semantic_headers: std::collections::HashSet<String> = [&name, &raw_class, &attributes]
        .into_iter()
        .filter(|header| !header.is_empty())
        .map(|header| normalize(header))
        .collect();
    let wager_re = Regex::new(r"(?i)wager").unwrap();
    for ln in &cleaned {
        let n = normalize(ln);
        if n.is_empty()
            || semantic_headers.contains(&n)
            || wager_re.is_match(ln)
            || matches!(n.as_str(), "take item" | "rematch" | "should recruit")
        {
            continue;
        }
        let mut best: Option<(String, f32)> = None;
        for (cand, normalized_candidate) in &normalized_catalog {
            let s = strsim::jaro_winkler(&n, normalized_candidate) as f32;
            if s >= 0.86 && best.as_ref().map(|(_, b)| s > *b).unwrap_or(true) {
                best = Some((cand.clone(), s));
            }
        }
        let Some((matched, score)) = best else {
            continue;
        };
        let (canon, _) = canonical_skill(&matched);
        if canon.is_empty() || seen.contains(&canon.to_lowercase()) {
            continue;
        }
        if score < 0.90 && n.len() > matched.len() + 10 {
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
