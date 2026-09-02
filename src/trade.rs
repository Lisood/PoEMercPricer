use std::sync::OnceLock;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::catalog::catalog_support_names;
use crate::models::{Mercenary, SupportTier};

const OFFICIAL_TRADE_SEARCH: &str = "https://www.pathofexile.com/trade/search";
// Anonymous searches currently accept one mercenary group with six filters.
// More groups are rejected as too complex even when they contain fewer filters.
const SAFE_GROUP_FILTER_LIMIT: usize = 6;

#[derive(Clone, Debug)]
pub struct TradeSearch {
    pub url: String,
    pub selected_skill: String,
    pub included_skills: usize,
    pub available_skills: usize,
    pub included_filters: usize,
    pub available_filters: usize,
}

#[derive(Debug, Deserialize)]
struct TradeStatCatalog {
    patch: String,
    league: String,
    source: String,
    entries: Vec<TradeStat>,
}

#[derive(Debug, Deserialize)]
struct TradeStat {
    id: String,
    text: String,
}

fn trade_stats() -> &'static TradeStatCatalog {
    static STATS: OnceLock<TradeStatCatalog> = OnceLock::new();
    STATS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/trade-stats-3.29.json"))
            .expect("bundled official 3.29 trade stat catalog must be valid")
    })
}

fn key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn support_name_and_tier(text: &str) -> Option<(&str, SupportTier)> {
    let (name, tier) = text.rsplit_once(" (Tier ")?;
    let tier = tier.strip_suffix(')')?.parse::<u8>().ok()?;
    Some((name, SupportTier::from_u8(tier)))
}

fn support_key(value: &str) -> String {
    let value = value.trim();
    let without_tier_prefix = ["Lesser ", "Greater "]
        .into_iter()
        .find_map(|prefix| value.strip_prefix(prefix))
        .unwrap_or(value);
    let normalized = key(without_tier_prefix);
    if normalized == "areaofeffect" {
        "increasedareaofeffect".into()
    } else {
        normalized
    }
}

fn skill_trade_id(name: &str) -> Result<&'static str> {
    let wanted = key(name);
    let matches: Vec<_> = trade_stats()
        .entries
        .iter()
        .filter(|entry| entry.id.starts_with("mercenary.skill_") && key(&entry.text) == wanted)
        .collect();
    match matches.as_slice() {
        [entry] => Ok(entry.id.as_str()),
        [] => bail!("no official trade stat for skill {name}"),
        _ => bail!("multiple official trade stats for skill {name}"),
    }
}

fn support_trade_id(canonical: &str, tier: SupportTier, skill_name: &str) -> Result<&'static str> {
    if tier == SupportTier::Absent {
        bail!("support {canonical} has no visible tier");
    }
    let known_names = catalog_support_names(canonical);
    if known_names.is_empty() {
        bail!("unknown support identity {canonical}");
    }
    let known_keys: Vec<_> = known_names.into_iter().map(support_key).collect();
    let matches: Vec<_> = trade_stats()
        .entries
        .iter()
        .filter(|entry| entry.id.starts_with("mercenary.support_"))
        .filter(|entry| {
            support_name_and_tier(&entry.text).is_some_and(|(name, visible_tier)| {
                visible_tier == tier && known_keys.contains(&support_key(name))
            })
        })
        .collect();
    match matches.as_slice() {
        [entry] => Ok(entry.id.as_str()),
        [] => bail!(
            "no official trade stat for support {canonical} at tier {}",
            tier as u8
        ),
        entries if canonical == "gilded_extra_targets" => {
            let expected_hash = match key(skill_name).as_str() {
                "smite" => "37259",
                "lightningarrow" | "greaterlightningarrow" => "58471",
                _ => bail!("no skill-bound Extra Targets trade stat for {skill_name}"),
            };
            entries
                .iter()
                .find(|entry| entry.id.ends_with(expected_hash))
                .map(|entry| entry.id.as_str())
                .with_context(|| format!("missing Extra Targets hash {expected_hash}"))
        }
        _ => bail!(
            "multiple official trade stats for support {canonical} at tier {} on {skill_name}",
            tier as u8
        ),
    }
}

/// Official item-type ids from `/api/trade/data/items` for patch 3.29. A scan
/// identifies both build and rarity, so unlike a build-wide market report we
/// can select one exact warrant type without splitting the user's result pool.
fn warrant_trade_type(mercenary: &Mercenary) -> Result<&'static str> {
    let class = mercenary
        .class_name
        .strip_prefix("Infamous ")
        .unwrap_or(&mercenary.class_name);
    let build = if class.trim().is_empty() {
        key(&mercenary.family)
    } else {
        key(class)
    };
    let (ordinary, infamous) = match build.as_str() {
        "manyshot" => (Some("EleBowRangerClones"), Some("EleBowRangerClonesNoble")),
        "kineticist" => (
            Some("MiscScionWandAttacks"),
            Some("MiscScionWandAttacksNoble"),
        ),
        "combatant" => (
            Some("MeleeAOEStrikeDuelistRangeStrikes"),
            Some("MeleeAOEStrikeDuelistRangeStrikesNoble"),
        ),
        "sniper" => (
            Some("NonEleBowRangerPhys"),
            Some("NonEleBowRangerPhysNoble"),
        ),
        "cruelmistress" => (
            Some("ChaosMinionWitchChaosHit"),
            Some("ChaosMinionWitchChaosHitNoble"),
        ),
        "thunderquiver" => (Some("EleBowRangerLightning"), None),
        "flamequiver" => (Some("EleBowRangerFire"), Some("EleBowRangerFireNoble")),
        "toxicologist" => (
            Some("NonEleBowRangerChaos"),
            Some("NonEleBowRangerChaosNoble"),
        ),
        "bladeambusher" => (
            Some("TrapsMinesShadowAttack"),
            Some("TrapsMinesShadowAttackNoble"),
        ),
        "striker" => (
            Some("MeleeStrikesMaraduerPhys"),
            Some("MeleeStrikesMaraduerPhysNoble"),
        ),
        "bladebitter" => (Some("Crit1HShadowPoison"), Some("Crit1HShadowPoisonNoble")),
        "stormhand" => (
            Some("ElementalWitchLightning"),
            Some("ElementalWitchLightningNoble"),
        ),
        "withertouch" => (
            Some("ChaosMinionWitchDot"),
            Some("ChaosMinionWitchDotNoble"),
        ),
        "mysteriousdiver" => (Some("DivingDuelist"), Some("DivingDuelistNoble")),
        "frosthand" => (Some("ElementalWitchCold"), Some("ElementalWitchColdNoble")),
        "stormingzealot" => (
            Some("PhysConvertTemplarLightning"),
            Some("PhysConvertTemplarLightningNoble"),
        ),
        "bladecaster" => (
            Some("Crit1HShadowPhysSpell"),
            Some("Crit1HShadowPhysSpellNoble"),
        ),
        "shockambusher" => (
            Some("TrapsMinesShadowLightning"),
            Some("TrapsMinesShadowLightningNoble"),
        ),
        "cardinal" => (
            Some("AurasMinionsTemplarStaff"),
            Some("AurasMinionsTemplarStaffNoble"),
        ),
        "warpriest" => (
            Some("AurasMinionsTemplarSmite"),
            Some("AurasMinionsTemplarSmiteNoble"),
        ),
        "swiftblade" => (
            Some("MeleeAOEStrikeDuelistCyclone"),
            Some("MeleeAOEStrikeDuelistCycloneNoble"),
        ),
        "smoulderstrike" => (Some("MeleeStrikesMarauderFire"), None),
        "sanguimancer" => (Some("MiscScionPhysDot"), Some("MiscScionPhysDotNoble")),
        "earthshaker" => (
            Some("MeleeAOEMarauderPhysSlam"),
            Some("MeleeAOEMarauderPhysSlamNoble"),
        ),
        "reanimator" => (
            Some("ChaosMinionWitchInstability"),
            Some("ChaosMinionWitchInstabilityNoble"),
        ),
        "bloodletter" => (
            Some("PhysicalDuelistBleed"),
            Some("PhysicalDuelistBleedNoble"),
        ),
        "fallenreverend" => (
            Some("AurasMinionsTemplarSpectres"),
            Some("AurasMinionsTemplarSpectresNoble"),
        ),
        "bastion" => (
            Some("PhysicalDuelistShields"),
            Some("PhysicalDuelistShieldsNoble"),
        ),
        "ripper" => (
            Some("MeleeAOEMarauderNonSlam"),
            Some("MeleeAOEMarauderNonSlamNoble"),
        ),
        "eruptor" => (
            Some("MeleeAOEMarauderFireSlam"),
            Some("MeleeAOEMarauderFireSlamNoble"),
        ),
        "flamehand" => (Some("ElementalWitchFire"), None),
        "winterdeacon" => (Some("PhysConvertTemplarCold"), None),
        "frostambusher" => (Some("TrapsMinesShadowCold"), None),
        "flamingcharlatan" => (Some("PhysConvertTemplarFire"), None),
        "shattersword" => (Some("PhysicalDuelistSteel"), None),
        "warpriestoftheruckus" => (None, Some("AurasMinionsTemplarSmiteRuckusNoble")),
        _ => bail!(
            "no official warrant type for build {}",
            mercenary.class_name
        ),
    };
    if mercenary.infamous {
        infamous.context("this build has no Infamous warrant type")
    } else {
        ordinary.context("this build only exists as an Infamous warrant")
    }
}

/// Build the anonymous-safe, skill-bound query used by the official trade site.
/// PoE currently permits one anonymous `mercenary` group. We therefore select
/// the scan's most-supported skill and keep its supports exact instead of
/// flattening supports across skills and returning false-positive matches.
fn build_trade_query(mercenary: &Mercenary) -> Result<(Value, usize, usize, String)> {
    if mercenary.skills.is_empty() {
        bail!("scan has no mercenary skills to search");
    }

    let mut candidates = Vec::with_capacity(mercenary.skills.len());
    for (skill_index, skill) in mercenary.skills.iter().enumerate() {
        let skill_name = skill.canonical.trim();
        let skill_id = skill_trade_id(skill_name)?;
        let mut support_filters = Vec::with_capacity(skill.supports.len());
        for (support_index, support) in skill.supports.iter().enumerate() {
            if matches!(support.canonical.as_str(), "" | "unknown" | "ambiguous") {
                bail!(
                    "select the exact support for {} before opening trade",
                    skill_name
                );
            }
            let id = support_trade_id(&support.canonical, support.tier, skill_name)
                .with_context(|| format!("{} on {}", support.name, skill_name))?;
            support_filters.push((std::cmp::Reverse(support.tier as u8), support_index, id));
        }
        let tier_sum = skill
            .supports
            .iter()
            .map(|support| support.tier as usize)
            .sum::<usize>();
        let tier_three_count = skill
            .supports
            .iter()
            .filter(|support| support.tier == SupportTier::T3)
            .count();
        candidates.push((
            tier_three_count,
            tier_sum,
            skill.supports.len(),
            std::cmp::Reverse(skill_index),
            skill_name.to_owned(),
            skill_id,
            support_filters,
        ));
    }

    let (_, _, _, _, selected_skill, skill_id, mut support_filters) = candidates
        .into_iter()
        .max_by_key(|candidate| (candidate.0, candidate.1, candidate.2, candidate.3))
        .context("scan has no searchable skill")?;
    support_filters.sort_by_key(|entry| (entry.0, entry.1));
    let available_filters = mercenary.skills.len()
        + mercenary
            .skills
            .iter()
            .map(|skill| skill.supports.len())
            .sum::<usize>();
    let mut filters = vec![json!({ "id": skill_id })];
    filters.extend(
        support_filters
            .into_iter()
            .take(SAFE_GROUP_FILTER_LIMIT - 1)
            .map(|(_, _, id)| json!({ "id": id })),
    );
    let included_filters = filters.len();
    let warrant_type = warrant_trade_type(mercenary)?;
    let level_filter = mercenary.level.map(|level| {
        json!({
            "misc_filters": {
                "filters": { "ilvl": { "min": level, "max": level } }
            }
        })
    });

    let mut query = json!({
        "query": {
            "status": { "option": "securable" },
            "type": { "option": warrant_type, "discriminator": "mercenary_warrant" },
            "stats": [{ "type": "mercenary", "filters": filters }]
        },
        "sort": { "price": "asc" }
    });
    if let Some(level_filter) = level_filter {
        query["query"]["filters"] = level_filter;
    }
    Ok((query, included_filters, available_filters, selected_skill))
}

/// Return only the official request payload. Internal accounting metadata is
/// deliberately kept out of the JSON sent to Path of Exile.
pub fn trade_query(mercenary: &Mercenary) -> Result<Value> {
    Ok(build_trade_query(mercenary)?.0)
}

pub fn trade_search(mercenary: &Mercenary, league: &str) -> Result<TradeSearch> {
    let league = league.trim();
    if league.is_empty() {
        bail!("trade league is empty");
    }
    if league.len() > 64 || league.chars().any(char::is_control) {
        bail!("trade league is invalid");
    }
    let (query, included_filters, available_filters, selected_skill) =
        build_trade_query(mercenary)?;
    let query = serde_json::to_string(&query)?;
    let url = format!(
        "{}/{}?q={}",
        OFFICIAL_TRADE_SEARCH,
        percent_encode_component(league),
        percent_encode_component(&query)
    );
    Ok(TradeSearch {
        url,
        selected_skill,
        included_skills: 1,
        available_skills: mercenary.skills.len(),
        included_filters,
        available_filters,
    })
}

pub fn trade_search_url(mercenary: &Mercenary, league: &str) -> Result<String> {
    Ok(trade_search(mercenary, league)?.url)
}

fn percent_encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

pub fn bundled_trade_stat_provenance() -> (&'static str, &'static str, &'static str) {
    let stats = trade_stats();
    (&stats.patch, &stats.league, &stats.source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{catalog_skill_names, catalog_skill_support_tiers};
    use crate::models::kineticist_jackpot_fixture;

    #[test]
    fn kineticist_query_binds_supports_to_their_skill() {
        let (mercenary, _) = kineticist_jackpot_fixture();
        let query = trade_query(&mercenary).expect("build official trade query");
        let groups = query["query"]["stats"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert!(groups.iter().all(|group| group["type"] == "mercenary"));
        assert_eq!(groups[0]["filters"].as_array().unwrap().len(), 5);
        assert_eq!(query["query"]["status"]["option"], "securable");
        assert_eq!(
            query["query"]["type"]["option"],
            "MiscScionWandAttacksNoble"
        );
        assert_eq!(
            query["query"]["filters"]["misc_filters"]["filters"]["ilvl"]["min"],
            83
        );
        assert_eq!(
            query["query"]["filters"]["misc_filters"]["filters"]["ilvl"]["max"],
            83
        );
        assert_eq!(query["sort"]["price"], "asc");
        assert!(query.get("_poemerc").is_none());

        let search = trade_search(&mercenary, "Allflame").unwrap();
        assert_eq!(search.selected_skill, "Kinetic Blast of Clustering");
        assert_eq!(search.included_skills, 1);
        assert_eq!(search.available_skills, 3);
        assert_eq!(search.included_filters, 5);
        assert_eq!(search.available_filters, 7);
    }

    #[test]
    fn official_url_contains_one_offline_encoded_query() {
        let (mercenary, _) = kineticist_jackpot_fixture();
        let url = trade_search_url(&mercenary, "Allflame").unwrap();
        assert!(url.starts_with("https://www.pathofexile.com/trade/search/Allflame?q="));
        assert!(!url.contains(' '));
        assert!(!url.contains("xddbsns"));

        let hardcore = trade_search_url(&mercenary, "Hardcore Allflame").unwrap();
        assert!(hardcore.contains("/Hardcore%20Allflame?q="));
    }

    #[test]
    fn unresolved_support_is_never_silently_omitted() {
        let (mut mercenary, _) = kineticist_jackpot_fixture();
        mercenary.skills[0].supports[0].canonical = "ambiguous".into();
        let error = trade_query(&mercenary).unwrap_err().to_string();
        assert!(error.contains("select the exact support"));
    }

    #[test]
    fn bundled_stats_are_the_current_official_snapshot() {
        let (patch, league, source) = bundled_trade_stat_provenance();
        assert_eq!(patch, "3.29");
        assert_eq!(league, "Allflame");
        assert_eq!(source, "https://www.pathofexile.com/api/trade/data/stats");
        assert_eq!(trade_stats().entries.len(), 534);
    }

    #[test]
    fn every_329_skill_and_attainable_support_tier_has_one_official_trade_id() {
        let mut skill_count = 0;
        for skill in catalog_skill_names() {
            skill_trade_id(skill).unwrap_or_else(|error| panic!("{skill}: {error}"));
            skill_count += 1;
        }
        assert_eq!(skill_count, 267);

        let support_tiers = catalog_skill_support_tiers();
        for (skill, canonical, tier) in &support_tiers {
            support_trade_id(canonical, SupportTier::from_u8(*tier), skill)
                .unwrap_or_else(|error| panic!("{canonical} T{tier} on {skill}: {error}"));
        }
        assert_eq!(support_tiers.len(), 4_028, "attainable pair census");
    }
}
