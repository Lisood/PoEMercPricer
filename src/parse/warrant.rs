use regex::Regex;
use std::sync::LazyLock;

use crate::catalog::{canonical_skill, canonical_support, classify_class, parse_tier};
use crate::models::{infer_mercenary_skill_level, Mercenary, Skill, SupportGem, SupportTier};

static SECTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*-{4,}\s*$").unwrap());
static BUILD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?im)^Build:\s*(.+)$").unwrap());
static LEVEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?im)^Mercenary Level:\s*(\d+)").unwrap());
static SKIP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:note:|item level:|(?:corrupted|unidentified|mirrored)$)").unwrap()
});
static TIER_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?P<name>.+?)\s*(?:\((?:Tier:?\s*)?(?P<tier>[123]|I{1,3})\)|\bT(?P<t>[123])\b)?\s*$",
    )
    .unwrap()
});

pub fn looks_like_warrant(text: &str) -> bool {
    // A real warrant is under 2 KB; a 1 MB single line took 14 s of fuzzy matching.
    if text.len() > 64 * 1024 {
        return false;
    }
    let t = text.trim().to_lowercase();
    t.contains("mercenary warrant") || (t.contains("build:") && t.contains("mercenary level:"))
}

pub fn parse_warrant_text(text: &str) -> Mercenary {
    let text = text.replace("\r\n", "\n");
    let text = text.trim();
    let sections: Vec<&str> = SECTION_RE
        .split(text)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let raw_class = BUILD_RE
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();
    let (display, family, infamous) = classify_class(&raw_class);
    let level = LEVEL_RE
        .captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok());

    let mut skills = Vec::new();
    let mut started = false;
    for section in sections {
        if BUILD_RE.is_match(section) || LEVEL_RE.is_match(section) {
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
            .filter(|l| !l.is_empty() && !l.starts_with("----"))
            .collect();
        if lines.is_empty() || SKIP_RE.is_match(lines[0]) {
            continue;
        }
        let (skill_name, conf) = canonical_skill(lines[0]);
        if skill_name.is_empty() {
            continue;
        }
        let mut gems = Vec::new();
        for line in &lines[1..] {
            let (raw_name, extra_tier) = if let Some(c) = TIER_LINE.captures(line) {
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
