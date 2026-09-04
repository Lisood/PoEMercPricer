use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum SupportTier {
    #[default]
    Absent = 0,
    T1 = 1,
    T2 = 2,
    T3 = 3,
}

impl SupportTier {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::T1,
            2 => Self::T2,
            3 => Self::T3,
            _ => Self::Absent,
        }
    }

    pub fn factor(self) -> f32 {
        match self {
            Self::Absent => 0.0,
            Self::T1 => 0.6,
            Self::T2 => 0.8,
            Self::T3 => 1.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SupportGem {
    pub name: String,
    pub canonical: String,
    pub tier: SupportTier,
    pub confidence: f32,
    pub raw: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Skill {
    pub name: String,
    pub canonical: String,
    pub supports: Vec<SupportGem>,
    /// Active-skill level, read directly or derived from the mercenary level.
    pub level: Option<u32>,
    pub confidence: f32,
    pub raw: String,
}

/// 3.29 mercenary active/aura skill progression. Supports use tiers instead.
pub fn infer_mercenary_skill_level(mercenary_level: u32) -> Option<u32> {
    let mercenary_level = mercenary_level.min(100);
    (mercenary_level >= 66).then(|| (18 + (mercenary_level - 66) / 2).min(40))
}

impl Skill {
    pub fn new(name: impl Into<String>, canonical: impl Into<String>) -> Self {
        let n = name.into();
        let c = canonical.into();
        Self {
            name: n,
            canonical: c,
            supports: Vec::new(),
            level: None,
            confidence: 1.0,
            raw: String::new(),
        }
    }

    pub fn with_supports(mut self, supports: Vec<(String, SupportTier)>) -> Self {
        self.supports = supports
            .into_iter()
            .map(|(name, tier)| SupportGem {
                canonical: name.clone(),
                name,
                tier,
                confidence: 1.0,
                raw: String::new(),
            })
            .collect();
        self
    }

    pub fn support_tier(&self, ids: &[&str]) -> SupportTier {
        let mut best = SupportTier::Absent;
        for gem in &self.supports {
            if ids.iter().any(|id| gem.canonical.eq_ignore_ascii_case(id)) && gem.tier > best {
                best = gem.tier;
            }
        }
        best
    }

    pub fn t(&self, ids: &[&str]) -> f32 {
        self.support_tier(ids).factor()
    }

    pub fn has_support(&self, ids: &[&str]) -> bool {
        self.support_tier(ids) != SupportTier::Absent
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Mercenary {
    pub name: String,
    pub class_name: String,
    pub family: String,
    pub level: Option<u32>,
    pub attributes: String,
    pub infamous: bool,
    pub skills: Vec<Skill>,
    pub source: String,
    pub notes: Vec<String>,
}

impl Mercenary {
    pub fn has_skill(&self, names: &[&str]) -> bool {
        self.skills
            .iter()
            .any(|s| names.iter().any(|n| s.canonical.eq_ignore_ascii_case(n)))
    }

    pub fn skill(&self, names: &[&str]) -> Option<&Skill> {
        self.skills
            .iter()
            .find(|s| names.iter().any(|n| s.canonical.eq_ignore_ascii_case(n)))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub label: String,
    pub points: f32,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreResult {
    pub family: String,
    pub score: f32,
    pub band: String,
    pub action: String,
    pub jackpot: bool,
    pub bricks: Vec<String>,
    pub highlights: Vec<String>,
    pub breakdown: Vec<ScoreBreakdown>,
    pub notes: Vec<String>,
    pub formula: String,
    #[serde(default)]
    pub estimate: bool,
}

impl Default for ScoreResult {
    fn default() -> Self {
        Self {
            family: "other".into(),
            score: 0.0,
            band: "unsupported".into(),
            action: String::new(),
            jackpot: false,
            bricks: Vec::new(),
            highlights: Vec::new(),
            breakdown: Vec::new(),
            notes: Vec::new(),
            formula: "screening-heuristic".into(),
            estimate: false,
        }
    }
}

pub fn clamp(value: f32) -> f32 {
    value.clamp(0.0, 100.0)
}

pub fn present(flag: bool) -> f32 {
    if flag {
        1.0
    } else {
        0.0
    }
}

pub fn interpret_score(score: f32, family: &str) -> (&'static str, &'static str) {
    let check_at = if family == "combatant" { 60.0 } else { 55.0 };
    if score < 50.0 {
        ("skip", "Usually skip the Warrant for resale")
    } else if score < check_at {
        ("common", "Correct core but common; Chaos to low Divines")
    } else if score < 65.0 {
        ("check", "Worth creating and price-checking the Warrant")
    } else if score < 80.0 {
        ("good", "Good; always create and price-check the Warrant")
    } else if score < 90.0 {
        ("very-valuable", "Very valuable, search supports carefully")
    } else {
        (
            "jackpot-band",
            "Jackpot candidate; search every support exactly",
        )
    }
}

pub fn kineticist_jackpot_fixture() -> (Mercenary, ScoreResult) {
    let merc = Mercenary {
        name: "Example Kineticist".into(),
        class_name: "Infamous Kineticist".into(),
        family: "kineticist".into(),
        level: Some(83),
        infamous: true,
        source: "fixture".into(),
        skills: vec![
            Skill::new("Kinetic Blast of Clustering", "Kinetic Blast of Clustering").with_supports(
                vec![
                    ("return".into(), SupportTier::T3),
                    ("gmp".into(), SupportTier::T3),
                    ("chain".into(), SupportTier::T2),
                    ("edwa".into(), SupportTier::T3),
                ],
            ),
            Skill::new("Greater Kinetic Blast", "Greater Kinetic Blast"),
            Skill::new("Haste", "Haste"),
        ],
        ..Default::default()
    };
    let result = crate::scoring::score_mercenary(&merc, false);
    (merc, result)
}
