use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

const INFAMOUS_PREFIX: &str = "infamous ";

#[derive(Debug, Deserialize)]
struct Catalog329 {
    patch: String,
    builds: Vec<CatalogBuild>,
    skills: Vec<CatalogSkill>,
    supports: Vec<CatalogSupport>,
}

#[derive(Debug, Deserialize)]
struct CatalogBuild {
    name: String,
    family: String,
    #[serde(default)]
    listings: u64,
}

#[derive(Debug, Deserialize)]
struct CatalogSkill {
    name: String,
    icon: String,
}

#[derive(Debug, Deserialize)]
struct CatalogSupport {
    name: String,
    canonical: String,
    icon: String,
    /// Compact `active skill|tiers` entries in which PoEDB lists this support.
    /// For example, `Holy Flame Totem|123` permits all three visible tiers.
    /// Tier is visible in the panel, so this removes impossible identities
    /// from shared artwork without adding OCR or storing redundant build data.
    #[serde(default)]
    skill_tiers: Vec<String>,
}

fn catalog_329() -> &'static Catalog329 {
    static CATALOG: OnceLock<Catalog329> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/catalog-3.29.json"))
            .expect("bundled 3.29 mercenary catalog must be valid")
    })
}

pub fn catalog_patch() -> &'static str {
    &catalog_329().patch
}

pub fn known_build_names() -> impl Iterator<Item = &'static str> {
    catalog_329().builds.iter().map(|b| b.name.as_str())
}

pub fn known_families() -> impl Iterator<Item = &'static str> {
    catalog_329().builds.iter().map(|b| b.family.as_str())
}

pub(crate) fn catalog_support_names(canonical: &str) -> Vec<&'static str> {
    catalog_329()
        .supports
        .iter()
        .filter(|support| support.canonical == canonical)
        .map(|support| support.name.as_str())
        .collect()
}

#[cfg(test)]
pub(crate) fn catalog_skill_names() -> impl Iterator<Item = &'static str> {
    catalog_329().skills.iter().map(|skill| skill.name.as_str())
}

#[cfg(test)]
pub(crate) fn catalog_skill_support_tiers() -> Vec<(&'static str, &'static str, u8)> {
    let mut pairs = std::collections::BTreeSet::new();
    for support in &catalog_329().supports {
        for entry in &support.skill_tiers {
            let Some((skill, tiers)) = entry.rsplit_once('|') else {
                continue;
            };
            for tier in tiers.bytes().filter_map(|tier| tier.checked_sub(b'0')) {
                if (1..=3).contains(&tier) {
                    pairs.insert((skill, support.canonical.as_str(), tier));
                }
            }
        }
    }
    pairs.into_iter().collect()
}

pub fn family_listings(family: &str) -> Option<u64> {
    catalog_329()
        .builds
        .iter()
        .find(|b| b.family == family)
        .map(|b| b.listings)
}

const CLASS_TO_FAMILY: &[(&str, &str)] = &[
    ("kineticist", "kineticist"),
    ("infamous kineticist", "kineticist"),
    ("manyshot", "manyshot"),
    ("infamous manyshot", "manyshot"),
    ("combatant", "combatant"),
    ("infamous combatant", "combatant"),
    ("sniper", "sniper"),
    ("infamous sniper", "sniper"),
    ("thunderquiver", "thunderquiver"),
    ("infamous thunderquiver", "thunderquiver"),
    ("stormhand", "stormhand"),
    ("infamous stormhand", "stormhand"),
];

const OTHER_CLASSES: &[&str] = &[
    "blade ambusher",
    "infamous blade ambusher",
    "frosthand",
    "infamous frosthand",
    "storming zealot",
    "infamous storming zealot",
    "sanguimancer",
    "infamous sanguimancer",
    "stormhand",
    "flamehand",
    "withertouch",
    "reanimator",
    "cruel mistress",
    "shock ambusher",
    "frost ambusher",
    "bladecaster",
    "bladebitter",
    "eruptor",
    "ripper",
    "earthshaker",
    "smoulderstrike",
    "striker",
    "warpriest",
    "cardinal",
    "fallen reverend",
    "winter deacon",
    "sniper",
    "bastion",
];

const SKILL_ALIASES: &[(&str, &str)] = &[
    ("kinetic blast of clustering", "Kinetic Blast of Clustering"),
    ("kboc", "Kinetic Blast of Clustering"),
    ("kinetic blast clustering", "Kinetic Blast of Clustering"),
    ("greater kinetic blast", "Greater Kinetic Blast"),
    ("greater kb", "Greater Kinetic Blast"),
    ("kinetic bolt", "Kinetic Bolt"),
    ("power siphon", "Power Siphon"),
    ("kinetic rain of impact", "Kinetic Rain of Impact"),
    ("kinetic rain", "Kinetic Rain of Impact"),
    ("barrage", "Barrage"),
    ("elemental weakness", "Elemental Weakness"),
    ("flame wall", "Flame Wall"),
    ("haste", "Haste"),
    ("flame dash", "Flame Dash"),
    ("frostblink", "Frostblink"),
    ("inspiring cry", "Inspiring Cry"),
    ("ice shot", "Ice Shot"),
    ("vaal ice shot", "Vaal Ice Shot"),
    ("vaal grace", "Vaal Grace"),
    ("frigid forkshot", "Frigid Forkshot"),
    ("icicle rain", "Icicle Rain"),
    ("lcicle rain", "Icicle Rain"),
    ("mirror arrow", "Mirror Arrow"),
    ("hatred", "Hatred"),
    ("frost bomb", "Frost Bomb"),
    ("grace", "Grace"),
    ("dash", "Dash"),
    ("blink arrow", "Blink Arrow"),
    ("frost blades", "Frost Blades"),
    ("wild strike", "Wild Strike"),
    ("spectral helix", "Spectral Helix"),
    ("static strike", "Static Strike"),
    ("herald of ice", "Herald of Ice"),
    ("wrath", "Wrath"),
    ("purity of ice", "Purity of Ice"),
    ("blade trap", "Blade Trap"),
    ("trarthan agility", "Trarthan Agility"),
    ("spectral throw of trarthus", "Spectral Throw of Trarthus"),
    ("spectral helix of trarthus", "Spectral Helix of Trarthus"),
    ("summon skitterbots", "Summon Skitterbots"),
    ("frostbite", "Frostbite"),
    ("ice nova of projection", "Ice Nova of Projection"),
    ("vortex", "Vortex"),
    ("discipline", "Discipline"),
    ("unnerving blast", "Unnerving Blast"),
    ("wave of conviction", "Wave of Conviction"),
    ("divine retribution", "Divine Retribution"),
    ("divine ire", "Divine Ire"),
    ("boiling blood", "Boiling Blood"),
    ("storm call of trarthus", "Storm Call of Trarthus"),
    ("bodyswap", "Bodyswap"),
    ("vaal reap", "Vaal Reap"),
    ("vaal vitality", "Vaal Vitality"),
    ("tornado shot", "Tornado Shot"),
    ("ensnaring arrow", "Ensnaring Arrow"),
    ("poacher's mark", "Poacher's Mark"),
    ("poachers mark", "Poacher's Mark"),
    ("greater split arrow", "Greater Split Arrow"),
    (
        "rain of arrows of saturation",
        "Rain of Arrows of Saturation",
    ),
    ("shrapnel ballista", "Shrapnel Ballista"),
    ("barrage of volley fire", "Barrage of Volley Fire"),
    ("siege ballista of trarthus", "Siege Ballista of Trarthus"),
    ("split arrow", "Split Arrow"),
    ("lightning arrow", "Lightning Arrow"),
    ("assassin's mark", "Assassin's Mark"),
    ("assassins mark", "Assassin's Mark"),
    ("galvanic arrow", "Galvanic Arrow"),
    ("greater lightning arrow", "Greater Lightning Arrow"),
    ("vaal lightning arrow", "Vaal Lightning Arrow"),
    ("storm rain", "Storm Rain"),
    ("precision", "Precision"),
    ("conductivity", "Conductivity"),
    ("sigil of power", "Sigil of Power"),
    ("spark", "Spark"),
    ("arc", "Arc"),
    ("ball lightning of static", "Ball Lightning of Static"),
    ("greater stormcall", "Greater Stormcall"),
    ("greater shock nova", "Greater Shock Nova"),
    ("orb of storms", "Orb of Storms"),
    ("stormcall", "Stormcall"),
    ("lightning warp", "Lightning Warp"),
    ("purity of lightning", "Purity of Lightning"),
    ("gilded secondary shot", "Gilded Secondary Shot"),
    ("gilded secondary shots", "Gilded Secondary Shot"),
    ("mirage archer", "Mirage Archer"),
    ("second wind", "Second Wind"),
    ("arrow nova", "Arrow Nova"),
    ("brutality", "Brutality"),
];

const SUPPORT_ALIASES: &[(&str, &str)] = &[
    ("return", "return"),
    ("returning projectiles", "return"),
    ("return projectiles", "return"),
    ("greater multiple projectiles", "gmp"),
    ("multiple projectiles", "gmp"),
    ("gmp", "gmp"),
    ("chain", "chain"),
    ("gilded chain", "chain"),
    ("pierce", "pierce"),
    ("greater pierce", "pierce"),
    ("fork", "fork"),
    ("greater fork", "fork"),
    ("elemental damage with attacks", "edwa"),
    ("greater elemental damage with attacks", "edwa"),
    ("weapon elemental damage", "edwa"),
    ("edwa", "edwa"),
    ("wed", "edwa"),
    ("faster attacks", "faster_attacks"),
    ("greater faster attacks", "faster_attacks"),
    ("critical damage", "crit_damage"),
    ("greater critical damage", "crit_damage"),
    ("increased critical damage", "crit_damage"),
    ("critical chance", "crit_chance"),
    ("increased critical strikes", "crit_chance"),
    ("critical strike chance", "crit_chance"),
    ("sacred wisps", "sacred_wisps"),
    ("greater sacred wisps", "sacred_wisps"),
    ("hypothermia", "hypothermia"),
    ("greater hypothermia", "hypothermia"),
    ("cooldown recovery", "cdr"),
    ("increased cooldown recovery", "cdr"),
    ("cdr", "cdr"),
    ("increased area of effect", "aoe"),
    ("greater area of effect", "aoe"),
    ("area of effect", "aoe"),
    ("aoe", "aoe"),
    ("more duration", "more_duration"),
    ("increased duration", "more_duration"),
    ("multistrike", "multistrike"),
    ("greater multistrike", "multistrike"),
    ("gilded elemental weakness on hit", "ele_weakness_on_hit"),
    ("elemental weakness on hit", "ele_weakness_on_hit"),
    ("lightning penetration", "lightning_pen"),
    ("greater lightning penetration", "lightning_pen"),
    ("cold penetration", "cold_pen"),
    ("greater cold penetration", "cold_pen"),
    ("added cold", "added_cold"),
    ("added cold damage", "added_cold"),
    ("greater added cold", "added_cold"),
    ("added lightning", "added_lightning"),
    ("added lightning damage", "added_lightning"),
    ("brutality", "brutality"),
    ("arrow nova", "arrow_nova"),
    ("mirage archer", "mirage_archer"),
    ("second wind", "second_wind"),
    ("gilded secondary shot", "gilded_secondary_shot"),
    ("gilded secondary shots", "gilded_secondary_shot"),
];

pub fn support_display(id: &str) -> &str {
    match id {
        "return" => "Return",
        "gmp" => "Greater Multiple Projectiles",
        "chain" => "Chain",
        "pierce" => "Pierce",
        "fork" => "Fork",
        "edwa" => "Elemental Damage with Attacks",
        "faster_attacks" => "Faster Attacks",
        "crit_damage" => "Critical Damage",
        "crit_chance" => "Critical Chance",
        "sacred_wisps" => "Sacred Wisps",
        "hypothermia" => "Hypothermia",
        "cdr" => "Cooldown Recovery",
        "aoe" => "Increased Area of Effect",
        "more_duration" => "More Duration",
        "multistrike" => "Multistrike",
        "ele_weakness_on_hit" => "Gilded Elemental Weakness on Hit",
        "lightning_pen" => "Lightning Penetration",
        "cold_pen" => "Cold Penetration",
        "added_cold" => "Added Cold Damage",
        "added_lightning" => "Added Lightning Damage",
        other => catalog_329()
            .supports
            .iter()
            .find(|support| support.canonical == other)
            .map(|support| support.name.as_str())
            .unwrap_or(other),
    }
}

/// The 3.29 mercenary UI gives a few support families tier-specific names.
/// Keep the canonical id stable for scoring, but show the exact in-game label
/// where the tier changes the wording rather than merely appending I/II/III.
pub fn support_display_for_tier(id: &str, tier: u8) -> String {
    match (id, tier) {
        ("added_fire", 1) => "Lesser Added Fire".into(),
        ("added_fire", 3) => "Greater Added Fire".into(),
        ("aoe", 1) => "Lesser Increased Area of Effect".into(),
        ("aoe", 3) => "Greater Area of Effect".into(),
        _ => support_display(id).into(),
    }
}

pub fn skill_icon(name: &str) -> Option<&'static str> {
    let normalized = normalize(name);
    catalog_329()
        .skills
        .iter()
        .find(|skill| normalize(&skill.name) == normalized)
        .map(|skill| skill.icon.as_str())
}

pub fn support_icon(id: &str) -> Option<&'static str> {
    catalog_329()
        .supports
        .iter()
        .find(|support| support.canonical == id)
        .map(|support| support.icon.as_str())
}

/// Supports whose PoEDB art identity and active-skill compatibility both match.
/// Generic Mercenary art is intentionally shared, so callers must retain every
/// returned candidate unless this filter leaves exactly one.
pub fn compatible_supports_for_art(
    art_key: &str,
    skill_name: &str,
) -> Vec<(&'static str, &'static str)> {
    catalog_329()
        .supports
        .iter()
        .filter(|support| {
            let stem = support.icon.strip_suffix(".webp").unwrap_or(&support.icon);
            let support_art = stem.split_once("__").map(|(_, art)| art).unwrap_or(stem);
            support_art.eq_ignore_ascii_case(art_key)
                && support.skill_tiers.iter().any(|pair| {
                    pair.split_once('|')
                        .is_some_and(|(skill, _)| skill.eq_ignore_ascii_case(skill_name))
                })
        })
        .map(|support| (support.canonical.as_str(), support.name.as_str()))
        .collect()
}

/// Supports compatible with an artwork, active skill, and visible tier.
///
/// PoE deliberately reuses a few generic Mercenary support sprites. The
/// active skill removes many impossible identities, and the visible tier
/// removes supports which cannot occur at that tier for the skill. If the
/// catalog has no exact pair, return the broader skill-compatible set so
/// incomplete upstream data never becomes a confident but incorrect answer.
pub fn compatible_supports_for_skill_tier(
    art_key: &str,
    skill_name: &str,
    tier: u8,
) -> Vec<(&'static str, &'static str)> {
    if !(1..=3).contains(&tier) {
        return compatible_supports_for_art(art_key, skill_name);
    }

    let mut broad = Vec::new();
    let mut narrow = Vec::new();
    for support in &catalog_329().supports {
        let stem = support.icon.strip_suffix(".webp").unwrap_or(&support.icon);
        let support_art = stem.split_once("__").map(|(_, art)| art).unwrap_or(stem);
        if !support_art.eq_ignore_ascii_case(art_key) {
            continue;
        }
        let skill_tiers = support.skill_tiers.iter().find_map(|pair| {
            pair.split_once('|')
                .filter(|(skill, _)| skill.eq_ignore_ascii_case(skill_name))
                .map(|(_, tiers)| tiers)
        });
        let Some(tiers) = skill_tiers else {
            continue;
        };
        let candidate = (support.canonical.as_str(), support.name.as_str());
        broad.push(candidate);
        if tiers.as_bytes().contains(&(b'0' + tier)) {
            narrow.push(candidate);
        }
    }

    if narrow.is_empty() {
        broad
    } else {
        narrow
    }
}

pub fn family_skills(family: &str) -> &'static [&'static str] {
    match family {
        "sniper" => &[
            "Tornado Shot",
            "Ensnaring Arrow",
            "Poacher's Mark",
            "Greater Split Arrow",
            "Rain of Arrows of Saturation",
            "Shrapnel Ballista",
            "Barrage of Volley Fire",
            "Siege Ballista of Trarthus",
            "Split Arrow",
            "Grace",
            "Dash",
            "Blink Arrow",
        ],
        "thunderquiver" => &[
            "Lightning Arrow",
            "Assassin's Mark",
            "Galvanic Arrow",
            "Greater Lightning Arrow",
            "Vaal Lightning Arrow",
            "Storm Rain",
            "Wrath",
            "Blink Arrow",
            "Trarthan Agility",
            "Precision",
            "Dash",
        ],
        "stormhand" => &[
            "Conductivity",
            "Sigil of Power",
            "Spark",
            "Arc",
            "Ball Lightning of Static",
            "Greater Stormcall",
            "Greater Shock Nova",
            "Orb of Storms",
            "Stormcall",
            "Lightning Warp",
            "Frostblink",
            "Flame Dash",
            "Wrath",
            "Purity of Lightning",
        ],
        "kineticist" => &[
            "Elemental Weakness",
            "Flame Wall",
            "Kinetic Blast of Clustering",
            "Greater Kinetic Blast",
            "Barrage",
            "Kinetic Bolt",
            "Power Siphon",
            "Kinetic Rain of Impact",
            "Haste",
            "Flame Dash",
            "Frostblink",
            "Inspiring Cry",
        ],
        "manyshot" => &[
            "Ice Shot",
            "Vaal Grace",
            "Vaal Ice Shot",
            "Frigid Forkshot",
            "Icicle Rain",
            "Mirror Arrow",
            "Hatred",
            "Frost Bomb",
            "Grace",
            "Dash",
            "Blink Arrow",
        ],
        "combatant" => &[
            "Inspiring Cry",
            "Herald of Ice",
            "Frost Blades",
            "Wild Strike",
            "Spectral Helix",
            "Static Strike",
            "Wrath",
            "Dash",
            "Frostblink",
            "Purity of Ice",
        ],
        // PoEDB 3.29 Bladebitter primary/secondary/utility pool. Keeping this
        // family bounded prevents its class header from fuzzily becoming the
        // unrelated Blade Ambusher skill `Blade Trap`.
        "bladebitter" => &[
            "Viper Strike",
            "Abyssal Cry",
            "Venom Gyre",
            "Profane Strike",
            "Cobra Lash",
            "Pestilent Strike",
            "Whirling Blades",
            "Dash",
            "Malevolence",
            "Withering Step",
            "Grace",
        ],
        _ => &[],
    }
}

pub fn all_known_skills() -> Vec<&'static str> {
    let mut v: Vec<&str> = SKILL_ALIASES.iter().map(|(_, c)| *c).collect();
    v.extend(catalog_329().skills.iter().map(|skill| skill.name.as_str()));
    v.sort();
    v.dedup();
    v
}

fn re_space() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").unwrap())
}

fn re_punct() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^a-z0-9+ ]+").unwrap())
}

pub fn loosen_concatenated(text: &str) -> String {
    let mut t = text.to_string();
    // Require 2+ lowercase letters so OCR "lCICLE" is not split into "l CICLE".
    static CAMEL: OnceLock<Regex> = OnceLock::new();
    static ACRONYM: OnceLock<Regex> = OnceLock::new();
    static LETTER_DIGIT: OnceLock<Regex> = OnceLock::new();
    static DIGIT_LETTER: OnceLock<Regex> = OnceLock::new();
    t = CAMEL
        .get_or_init(|| Regex::new(r"([a-z]{2,})([A-Z])").unwrap())
        .replace_all(&t, "$1 $2")
        .into_owned();
    t = ACRONYM
        .get_or_init(|| Regex::new(r"([A-Z])([A-Z][a-z])").unwrap())
        .replace_all(&t, "$1 $2")
        .into_owned();
    t = LETTER_DIGIT
        .get_or_init(|| Regex::new(r"([A-Za-z])(\d)").unwrap())
        .replace_all(&t, "$1 $2")
        .into_owned();
    t = DIGIT_LETTER
        .get_or_init(|| Regex::new(r"(\d)([A-Za-z])").unwrap())
        .replace_all(&t, "$1 $2")
        .into_owned();
    let pairs = [
        ("manyshot", "many shot"),
        ("mirrorarrow", "mirror arrow"),
        ("blinkarrow", "blink arrow"),
        ("iceshot", "ice shot"),
        ("kineticblast", "kinetic blast"),
        ("frostblades", "frost blades"),
        ("wildstrike", "wild strike"),
        ("staticstrike", "static strike"),
        ("iciclerain", "icicle rain"),
        ("vaalgrace", "vaal grace"),
        ("vaalice", "vaal ice"),
        ("flamedash", "flame dash"),
        ("frostbomb", "frost bomb"),
    ];
    let lower = t.to_lowercase();
    let mut out = t.clone();
    for (needle, spaced) in pairs {
        if lower.contains(needle) {
            let re = Regex::new(&format!("(?i){needle}")).unwrap();
            out = re.replace_all(&out, spaced).into_owned();
        }
    }
    out
}

pub fn normalize(text: &str) -> String {
    let t = loosen_concatenated(text);
    let t = t.to_lowercase().replace(['’', '`'], "'");
    let t = t.replace("lnfamous", "infamous");
    let t = t.replace("lvi ", "lvl ");
    let t = re_punct().replace_all(&t, " ");
    let t = t.replace("many shot", "manyshot");
    re_space().replace_all(t.trim(), " ").into_owned()
}

fn similarity(a: &str, b: &str) -> f32 {
    let na = normalize(a);
    let nb = normalize(b);
    if na.is_empty() || nb.is_empty() {
        return 0.0;
    }
    if na == nb {
        return 1.0;
    }
    let j = strsim::jaro_winkler(&na, &nb) as f32;
    let l = strsim::normalized_levenshtein(&na, &nb) as f32;
    j.max(l)
}

fn best<'a>(
    query: &str,
    candidates: impl Iterator<Item = &'a str>,
    cutoff: f32,
) -> Option<(&'a str, f32)> {
    let mut best: Option<(&str, f32)> = None;
    for c in candidates {
        let s = similarity(query, c);
        if s >= cutoff && best.map(|(_, b)| s > b).unwrap_or(true) {
            best = Some((c, s));
        }
    }
    best
}

/// Returns (display_class, family, infamous).
pub fn classify_class(raw: &str) -> (String, String, bool) {
    let mut n = normalize(raw);
    n = n.replace("infamous", "infamous ").trim().to_string();
    n = re_space().replace_all(&n, " ").into_owned();
    let infamous = n.starts_with(INFAMOUS_PREFIX);
    if let Some((_, family)) = CLASS_TO_FAMILY.iter().find(|(k, _)| *k == n) {
        let display = if infamous {
            format!("Infamous {}", title(family))
        } else {
            title(family)
        };
        return (display, (*family).to_string(), infamous);
    }

    let base = n.strip_prefix(INFAMOUS_PREFIX).unwrap_or(&n).trim();
    if let Some(build) = catalog_329()
        .builds
        .iter()
        .find(|build| normalize(&build.name) == base)
    {
        return (
            if infamous {
                format!("Infamous {}", build.name)
            } else {
                build.name.clone()
            },
            build.family.clone(),
            infamous,
        );
    }
    if let Some(build) = catalog_329()
        .builds
        .iter()
        .filter(|build| base.contains(&normalize(&build.name)))
        .max_by_key(|build| normalize(&build.name).len())
    {
        return (
            if infamous {
                format!("Infamous {}", build.name)
            } else {
                build.name.clone()
            },
            build.family.clone(),
            infamous,
        );
    }
    let mut cands: Vec<&str> = CLASS_TO_FAMILY.iter().map(|(k, _)| *k).collect();
    cands.extend(OTHER_CLASSES.iter().copied());
    if let Some((key, _)) = best(&n, cands.into_iter(), 0.80) {
        if let Some((_, family)) = CLASS_TO_FAMILY.iter().find(|(k, _)| *k == key) {
            let inf = key.starts_with(INFAMOUS_PREFIX) || infamous;
            let display = if inf {
                format!("Infamous {}", title(family))
            } else {
                title(family)
            };
            return (display, (*family).to_string(), inf);
        }
        let inf = key.starts_with(INFAMOUS_PREFIX) || infamous;
        // A fuzzy OTHER_CLASSES hit may still be a catalog build (e.g. OCR
        // misread "Frosthend"); resolve it so the family gets scored.
        let key_base = key.strip_prefix(INFAMOUS_PREFIX).unwrap_or(key);
        if let Some(build) = catalog_329()
            .builds
            .iter()
            .find(|build| normalize(&build.name) == key_base)
        {
            return (
                if inf {
                    format!("Infamous {}", build.name)
                } else {
                    build.name.clone()
                },
                build.family.clone(),
                inf,
            );
        }
        return (title(key), "other".into(), inf);
    }

    let dynamic = catalog_329().builds.iter().map(|build| build.name.as_str());
    if let Some((key, _)) = best(base, dynamic, 0.80) {
        let build = catalog_329()
            .builds
            .iter()
            .find(|build| build.name == key)
            .expect("matched catalog build exists");
        return (
            if infamous {
                format!("Infamous {}", build.name)
            } else {
                build.name.clone()
            },
            build.family.clone(),
            infamous,
        );
    }
    (
        if raw.trim().is_empty() {
            "Unknown".into()
        } else {
            raw.trim().to_string()
        },
        "other".into(),
        infamous,
    )
}

pub fn canonical_skill(raw: &str) -> (String, f32) {
    let n = normalize(raw);
    if n.is_empty() {
        return (String::new(), 0.0);
    }
    if let Some((_, canon)) = SKILL_ALIASES.iter().find(|(k, _)| *k == n) {
        return ((*canon).to_string(), 1.0);
    }
    if let Some(skill) = catalog_329()
        .skills
        .iter()
        .find(|skill| normalize(&skill.name) == n)
    {
        return (skill.name.clone(), 1.0);
    }
    if let Some((key, score)) = best(&n, SKILL_ALIASES.iter().map(|(k, _)| *k), 0.82) {
        let canon = SKILL_ALIASES.iter().find(|(k, _)| *k == key).unwrap().1;
        return (canon.to_string(), score);
    }
    if let Some((key, score)) = best(
        &n,
        catalog_329().skills.iter().map(|skill| skill.name.as_str()),
        0.82,
    ) {
        return (key.to_string(), score);
    }
    (title(&n), 0.55)
}

pub fn canonical_support(raw: &str) -> (String, f32) {
    let template_key = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .trim_end_matches(".png")
        .trim_end_matches(".webp");
    if let Some(support) = catalog_329().supports.iter().find(|support| {
        support
            .icon
            .trim_end_matches(".png")
            .trim_end_matches(".webp")
            .eq_ignore_ascii_case(template_key)
    }) {
        return (support.canonical.clone(), 1.0);
    }
    let mut n = normalize(raw);
    let re_tier = Regex::new(r"\b(tier|t)\s*[123]\b").unwrap();
    n = re_tier.replace_all(&n, "").into_owned();
    let re_g = Regex::new(r"\b(lesser|greater|gilded)\b").unwrap();
    n = re_g.replace_all(&n, "").into_owned();
    n = re_space().replace_all(n.trim(), " ").into_owned();
    if n.is_empty() {
        return (String::new(), 0.0);
    }
    if let Some((_, canon)) = SUPPORT_ALIASES.iter().find(|(k, _)| *k == n) {
        return ((*canon).to_string(), 1.0);
    }
    if let Some(support) = catalog_329()
        .supports
        .iter()
        .find(|support| normalize(&support.name) == n)
    {
        return (support.canonical.clone(), 1.0);
    }
    if let Some((key, score)) = best(&n, SUPPORT_ALIASES.iter().map(|(k, _)| *k), 0.80) {
        let canon = SUPPORT_ALIASES.iter().find(|(k, _)| *k == key).unwrap().1;
        return (canon.to_string(), score);
    }
    if let Some((key, score)) = best(
        &n,
        catalog_329()
            .supports
            .iter()
            .map(|support| support.name.as_str()),
        0.80,
    ) {
        let support = catalog_329()
            .supports
            .iter()
            .find(|support| support.name == key)
            .expect("matched catalog support exists");
        return (support.canonical.clone(), score);
    }
    (n, 0.4)
}

pub fn parse_tier(raw: &str) -> u8 {
    let t = raw.to_lowercase();
    let re3 = Regex::new(r"\b(iii|tier[:\s]*3|\bt3\b|greater|gilded)\b").unwrap();
    if re3.is_match(&t) {
        return 3;
    }
    let re2 = Regex::new(r"\b(ii|tier[:\s]*2|\bt2\b)\b").unwrap();
    if re2.is_match(&t) {
        return 2;
    }
    let re1 = Regex::new(r"\b(lesser|tier[:\s]*1|\bt1\b)\b").unwrap();
    if re1.is_match(&t) {
        return 1;
    }
    if Regex::new(r"\biii\b").unwrap().is_match(&t) {
        return 3;
    }
    if Regex::new(r"\bii\b").unwrap().is_match(&t) {
        return 2;
    }
    if let Some(d) = t.chars().rev().find(|c| c.is_ascii_digit()) {
        if matches!(d, '1' | '2' | '3') {
            return d.to_digit(10).unwrap() as u8;
        }
    }
    2
}

fn title(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_infamous_manyshot() {
        let (display, family, infamous) = classify_class("Infamous Manyshot");
        assert_eq!(family, "manyshot");
        assert!(infamous);
        assert!(display.contains("Manyshot"));
    }

    #[test]
    fn fuzzy_misread_of_catalog_build_resolves_to_its_family() {
        let (display, family, _) = classify_class("Frosthend");
        assert_eq!(family, "frosthand");
        assert!(display.contains("Frosthand"));
    }

    #[test]
    fn skill_alias_kboc() {
        let (name, conf) = canonical_skill("KINETIC BLAST OF CLUSTERING");
        assert_eq!(name, "Kinetic Blast of Clustering");
        assert!(conf >= 0.9);
    }

    #[test]
    fn ocr_lcicle_rain() {
        let (name, conf) = canonical_skill("lCICLE RAIN");
        assert_eq!(name, "Icicle Rain");
        assert!(conf >= 0.8);
    }

    #[test]
    fn support_gmp_tier() {
        let (name, _) = canonical_support("Greater Multiple Projectiles (Tier: 3)");
        assert_eq!(name, "gmp");
        assert_eq!(parse_tier("Greater Multiple Projectiles (Tier: 3)"), 3);
    }

    #[test]
    fn tier_roman() {
        assert_eq!(parse_tier("II"), 2);
        assert_eq!(parse_tier("III"), 3);
        assert_eq!(parse_tier("Return T1"), 1);
    }

    #[test]
    fn bundled_329_catalog_is_current_and_complete() {
        assert_eq!(catalog_patch(), "3.29.3");
        assert_eq!(known_build_names().count(), 36);
        assert!(all_known_skills().len() >= 250);
        assert_eq!(
            canonical_skill("Soulrend of Reaping").0,
            "Soulrend of Reaping"
        );
        assert_eq!(canonical_skill("Conflagration").0, "Conflagration");
    }

    #[test]
    fn current_builds_classify_from_combined_ocr_headers() {
        let (display, family, infamous) = classify_class("Infamous Cruel Mistress   Lvl 84   Int");
        assert_eq!(display, "Infamous Cruel Mistress");
        assert_eq!(family, "cruel_mistress");
        assert!(infamous);
    }

    #[test]
    fn generated_template_names_resolve_to_scoring_ids() {
        assert_eq!(
            canonical_support("return__returnprojectiles.webp").0,
            "return"
        );
        assert_eq!(
            canonical_support("edwa__weaponelementaldamage.webp").0,
            "edwa"
        );
    }

    #[test]
    fn tiered_support_names_match_the_329_mercenary_labels() {
        assert_eq!(
            support_display_for_tier("added_fire", 3),
            "Greater Added Fire"
        );
        assert_eq!(support_display_for_tier("aoe", 3), "Greater Area of Effect");
        assert_eq!(support_display_for_tier("added_fire", 2), "Added Fire");
        assert_eq!(
            support_display_for_tier("aoe", 2),
            "Increased Area of Effect"
        );
    }

    #[test]
    fn generated_catalog_preserves_exact_skill_tier_pairs() {
        let raw = include_str!("../assets/catalog-3.29.json");
        assert!(
            !raw.starts_with('\u{feff}'),
            "catalog must be BOM-less UTF-8"
        );
        let json: serde_json::Value = serde_json::from_str(raw).expect("valid generated catalog");
        let generated_supports = json["supports"].as_array().expect("supports array");
        assert!(generated_supports.iter().all(|support| {
            support.get("skills").is_none()
                && support
                    .get("skill_tiers")
                    .is_some_and(serde_json::Value::is_array)
        }));

        let pair_count: usize = catalog_329()
            .supports
            .iter()
            .map(|support| support.skill_tiers.len())
            .sum();
        assert!(
            pair_count >= 1_000,
            "catalog unexpectedly lost skill+tier support placements: {pair_count}"
        );
        assert!(catalog_329()
            .supports
            .iter()
            .all(|support| support.skill_tiers.iter().all(|pair| pair
                .split_once('|')
                .is_some_and(|(_, tiers)| {
                    !tiers.is_empty() && tiers.bytes().all(|tier| (b'1'..=b'3').contains(&tier))
                }))));
    }

    #[test]
    fn visible_tier_reduces_shared_art_only_when_source_data_proves_it() {
        let mut resolved_collisions = 0usize;
        for support in &catalog_329().supports {
            let stem = support.icon.strip_suffix(".webp").unwrap_or(&support.icon);
            let art = stem.split_once("__").map(|(_, art)| art).unwrap_or(stem);
            for pair in &support.skill_tiers {
                let (skill, tiers) = pair.split_once('|').expect("validated skill tiers");
                for tier in tiers.bytes().map(|tier| tier - b'0') {
                    let broad = compatible_supports_for_art(art, skill);
                    let narrow = compatible_supports_for_skill_tier(art, skill, tier);
                    if broad.len() > 1 && narrow.len() == 1 {
                        resolved_collisions += 1;
                    }
                }
            }
        }
        assert!(
            resolved_collisions > 0,
            "visible tier should eliminate at least one shared-art collision"
        );

        // PoEDB lists both identities for this exact build and skill, and both
        // are backed by the same CDN pixels. Retaining both is essential: a
        // forced preference would silently mislabel legitimate Ironwood rolls.
        let holy =
            compatible_supports_for_skill_tier("mercsilverstrintsupportgem", "Holy Flame Totem", 2);
        let holy_ids: Vec<_> = holy.iter().map(|(canonical, _)| *canonical).collect();
        assert!(holy_ids.contains(&"ironwood"), "{holy_ids:?}");
        assert!(holy_ids.contains(&"physical_as_extra"), "{holy_ids:?}");
    }

    #[test]
    fn unavailable_tier_falls_back_to_conservative_skill_pool() {
        let broad = compatible_supports_for_art("mercsilverstrintsupportgem", "Holy Flame Totem");
        let unknown =
            compatible_supports_for_skill_tier("mercsilverstrintsupportgem", "Holy Flame Totem", 0);
        assert_eq!(unknown, broad);
    }
}
