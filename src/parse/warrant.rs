use regex::Regex;

use crate::catalog::{canonical_skill, canonical_support, classify_class, parse_tier};
use crate::models::{infer_mercenary_skill_level, Mercenary, Skill, SupportGem, SupportTier};

pub fn looks_like_warrant(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    t.contains("mercenary warrant") || t.contains("build:") || t.contains("mercenary level:")
}

pub fn parse_warrant_text(text: &str) -> Mercenary {
    let text = text.replace("\r\n", "\n");
    let text = text.trim();
    let section_re = Regex::new(r"\n-{4,}\n").unwrap();
    let sections: Vec<&str> = section_re
        .split(text)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let build_re = Regex::new(r"(?im)^Build:\s*(.+)$").unwrap();
    let level_re = Regex::new(r"(?im)^Mercenary Level:\s*(\d+)").unwrap();
    let name_re = Regex::new(r"(?im)^Name:\s*(.+)$").unwrap();

    let raw_class = build_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();
    let (display, family, infamous) = classify_class(&raw_class);
    let level = level_re
        .captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok());
    let name = name_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();

    let mut skills = Vec::new();
    let mut started = false;
    let tier_line = Regex::new(
        r"(?i)^(?P<name>.+?)\s*(?:\((?:Tier:?\s*)?(?P<tier>[123]|I{1,3})\)|\bT(?P<t>[123])\b)?\s*$",
    )
    .unwrap();
    for section in sections {
        if build_re.is_match(section) || level_re.is_match(section) {
            started = true;
            continue;
        }
        if !started && section.to_lowercase().contains("mercenary warrant") {
            continue;
        }
        if !started && section.to_lowercase().starts_with("item class") {
            continue;
        }
        let lines: Vec<&str> = section
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }
        let (skill_name, conf) = canonical_skill(lines[0]);
        if skill_name.is_empty() {
            continue;
        }
        let mut gems = Vec::new();
        for line in &lines[1..] {
            if line.starts_with("--------") {
                continue;
            }
            let (raw_name, extra_tier) = if let Some(c) = tier_line.captures(line) {
                let n = c.name("name").map(|m| m.as_str().trim()).unwrap_or(line);
                let t = c
                    .name("tier")
                    .or_else(|| c.name("t"))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                (n.to_string(), t.to_string())
            } else {
                ((*line).to_string(), String::new())
            };
            let (canon, sconf) = canonical_support(&raw_name);
            if canon.is_empty() {
                continue;
            }
            let tier = parse_tier(&format!("{raw_name} {extra_tier}"));
            gems.push(SupportGem {
                name: raw_name,
                canonical: canon,
                tier: SupportTier::from_u8(tier),
                confidence: sconf,
                raw: (*line).to_string(),
            });
        }
        skills.push(Skill {
            name: lines[0].to_string(),
            canonical: skill_name,
            supports: gems,
            level: level.and_then(infer_mercenary_skill_level),
            confidence: conf,
            raw: section.to_string(),
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
        infamous,
        skills,
        source: "clipboard".into(),
        ..Default::default()
    }
}
