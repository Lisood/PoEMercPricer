use std::sync::OnceLock;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::catalog::catalog_support_names;
use crate::models::{Mercenary, SupportTier};

const OFFICIAL_TRADE_SEARCH: &str = "https://www.pathofexile.com/trade/search";
// Anonymous searches accept one mercenary group with six filters. Any second
// group is rejected as too complex (verified live 2026-09-02); logging in on
// pathofexile.com lifts that, which is what `every_skill` relies on.
const SAFE_GROUP_FILTER_LIMIT: usize = 6;
// Logged in, five mercenary groups (the money skill with its supports, then
// four skill-only groups) search fine; six groups are rejected (verified live
// 2026-09-02).
const LOGGED_IN_GROUP_LIMIT: usize = 5;
// "securable" = instant-buyout listings, online or offline. It is the widest
// pool of firm asks: live it returned 586 Manyshot listings where "online"
// returned 3, so it is the better price-check population.
const LISTING_STATUS: &str = "securable";

#[derive(Clone, Debug)]
pub struct TradeSearch {
    pub url: String,
    pub selected_skill: String,
    pub included_skills: usize,
    pub available_skills: usize,
    pub included_filters: usize,
    pub available_filters: usize,
    /// Skills and supports left out of the query, with the reason.
    pub dropped: Vec<String>,
}

/// The family's money skill(s), most valuable first. Trade search prefers
/// these over whichever row happens to carry the most T3 supports.
fn money_skills(family: &str) -> &'static [&'static str] {
    match family {
        "kineticist" => &["Kinetic Blast of Clustering", "Greater Kinetic Blast"],
        "manyshot" => &["Vaal Ice Shot", "Ice Shot"],
        "combatant" => &["Frost Blades", "Wild Strike", "Static Strike"],
        other => match crate::scoring::market::screen_for(other) {
            Some(screen) => screen.money,
            None => &[],
        },
    }
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
        serde_json::from_str(include_str!(concat!(
            env!("OUT_DIR"),
            "/trade-stats-3.30.json"
        )))
        .expect("bundled official 3.30 trade stat catalog must be valid")
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

fn skill_trade_id(name: &str, build: &str) -> Result<&'static str> {
    let wanted = key(name);
    let matches: Vec<_> = trade_stats()
        .entries
        .iter()
        .filter(|entry| entry.id.starts_with("mercenary.skill_") && key(&entry.text) == wanted)
        .collect();
    match matches.as_slice() {
        [entry] => Ok(entry.id.as_str()),
        [] => bail!("no official trade stat for skill {name}"),
        entries if wanted == "heraldofash" => {
            // 3.30 listings carry 31201 on Chaplain of the Well and 44334 on
            // every other build (Eruptor, Flaming Charlatan).
            let expected_hash = if key(build) == "chaplainofthewell" {
                "31201"
            } else {
                "44334"
            };
            entries
                .iter()
                .find(|entry| entry.id.ends_with(expected_hash))
                .map(|entry| entry.id.as_str())
                .with_context(|| format!("missing Herald of Ash hash {expected_hash}"))
        }
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
            // Both bundled stats share the text "Gilded Extra Targets (Tier 3)";
            // the site scopes 37259 to Smite and 58471 to Lightning Arrow.
            let expected_hash = match key(skill_name).as_str() {
                // Vaal Smite on 37259 confirmed live in mercenarySkills, 2026-11-18.
                "smite" | "vaalsmite" => "37259",
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

/// The scanned build without its Infamous prefix, else the family id.
fn build_label(mercenary: &Mercenary) -> &str {
    let class = mercenary
        .class_name
        .strip_prefix("Infamous ")
        .unwrap_or(&mercenary.class_name);
    if class.trim().is_empty() {
        mercenary.family.as_str()
    } else {
        class
    }
}

/// Official item-type ids from `/api/trade/data/items` for patch 3.30. A scan
/// identifies both build and rarity, so unlike a build-wide market report we
/// can select one exact warrant type without splitting the user's result pool.
fn warrant_trade_type(mercenary: &Mercenary) -> Result<&'static str> {
    let label = build_label(mercenary);
    let (ordinary, infamous) = match key(label).as_str() {
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
        "chaplainofthewell" => (Some("AbyssalTemplar"), None),
        _ => bail!("no official warrant type for build {label}"),
    };
    if mercenary.infamous {
        infamous.context("this build has no Infamous warrant type")
    } else {
        ordinary.context("this build only exists as an Infamous warrant")
    }
}

/// Build the skill-bound query used by the official trade site. By default one
/// `mercenary` group (the anonymous limit) holds the family's money skill
/// (falling back to the scan's most-supported skill) with its supports exact.
/// With `every_skill` each skill gets its own group, money skill first, which
/// matches the whole warrant but needs a pathofexile.com login. Skills and
/// supports with no official stat are dropped and reported, never searched wrong.
fn build_trade_query(
    mercenary: &Mercenary,
    every_skill: bool,
) -> Result<(Value, usize, usize, usize, String, Vec<String>)> {
    if mercenary.skills.is_empty() {
        bail!("scan has no mercenary skills to search");
    }

    let money = money_skills(&mercenary.family.to_lowercase());
    let mut dropped = Vec::new();
    let mut candidates = Vec::with_capacity(mercenary.skills.len());
    for (skill_index, skill) in mercenary.skills.iter().enumerate() {
        let skill_name = skill.canonical.trim();
        let skill_id = match skill_trade_id(skill_name, build_label(mercenary)) {
            Ok(id) => id,
            Err(error) => {
                dropped.push(error.to_string());
                continue;
            }
        };
        let mut support_filters = Vec::with_capacity(skill.supports.len());
        for (support_index, support) in skill.supports.iter().enumerate() {
            let resolved = if matches!(support.canonical.as_str(), "" | "unknown" | "ambiguous") {
                Err(anyhow::anyhow!("select the exact support to search it"))
            } else {
                support_trade_id(&support.canonical, support.tier, skill_name)
            };
            match resolved {
                Ok(id) => {
                    support_filters.push((std::cmp::Reverse(support.tier as u8), support_index, id))
                }
                Err(error) => dropped.push(format!("{} on {skill_name}: {error}", support.name)),
            }
        }
        let money_rank = money
            .iter()
            .position(|name| name.eq_ignore_ascii_case(skill_name))
            .map_or(0, |index| money.len() - index);
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
            money_rank,
            tier_three_count,
            tier_sum,
            support_filters.len(),
            std::cmp::Reverse(skill_index),
            skill_name.to_owned(),
            skill_id,
            support_filters,
        ));
    }

    candidates.sort_by_key(|c| std::cmp::Reverse((c.0, c.1, c.2, c.3, c.4)));
    if candidates.is_empty() {
        bail!("scan has no searchable skill");
    }
    candidates.truncate(if every_skill {
        LOGGED_IN_GROUP_LIMIT
    } else {
        1
    });
    let selected_skill = candidates[0].5.clone();
    let available_filters = mercenary.skills.len()
        + mercenary
            .skills
            .iter()
            .map(|skill| skill.supports.len())
            .sum::<usize>();
    let mut included_filters = 0;
    let groups: Vec<Value> = candidates
        .into_iter()
        .enumerate()
        .map(
            |(rank, (_, _, _, _, _, _, skill_id, mut support_filters))| {
                support_filters.sort_by_key(|entry| (entry.0, entry.1));
                let mut filters = vec![json!({ "id": skill_id })];
                // Only the money skill keeps its supports; the other groups are
                // skill-only so the whole query stays under the logged-in limit.
                let support_budget = if rank == 0 {
                    SAFE_GROUP_FILTER_LIMIT - 1
                } else {
                    0
                };
                filters.extend(
                    support_filters
                        .into_iter()
                        .take(support_budget)
                        .map(|(_, _, id)| json!({ "id": id })),
                );
                included_filters += filters.len();
                json!({ "type": "mercenary", "filters": filters })
            },
        )
        .collect();
    let included_skills = groups.len();
    let warrant_type = warrant_trade_type(mercenary)?;
    // Min only: an exact ilvl turns a one-level OCR misread into zero results.
    let level_filter = mercenary.level.map(|level| {
        json!({
            "misc_filters": {
                "filters": { "ilvl": { "min": level } }
            }
        })
    });

    let mut query = json!({
        "query": {
            "status": { "option": LISTING_STATUS },
            "type": { "option": warrant_type, "discriminator": "mercenary_warrant" },
            "stats": groups
        },
        "sort": { "price": "asc" }
    });
    if let Some(level_filter) = level_filter {
        query["query"]["filters"] = level_filter;
    }
    Ok((
        query,
        included_skills,
        included_filters,
        available_filters,
        selected_skill,
        dropped,
    ))
}

/// Return only the official request payload. Internal accounting metadata is
/// deliberately kept out of the JSON sent to Path of Exile.
pub fn trade_query(mercenary: &Mercenary, every_skill: bool) -> Result<Value> {
    Ok(build_trade_query(mercenary, every_skill)?.0)
}

pub fn trade_search(mercenary: &Mercenary, league: &str, every_skill: bool) -> Result<TradeSearch> {
    let league = league.trim();
    if league.is_empty() {
        bail!("trade league is empty");
    }
    if league.len() > 64 || league.chars().any(char::is_control) {
        bail!("trade league is invalid");
    }
    let (query, included_skills, included_filters, available_filters, selected_skill, dropped) =
        build_trade_query(mercenary, every_skill)?;
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
        included_skills,
        available_skills: mercenary.skills.len(),
        included_filters,
        available_filters,
        dropped,
    })
}

pub fn trade_search_url(mercenary: &Mercenary, league: &str, every_skill: bool) -> Result<String> {
    Ok(trade_search(mercenary, league, every_skill)?.url)
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
    use crate::models::{kineticist_jackpot_fixture, Skill};

    #[test]
    fn kineticist_query_binds_supports_to_their_skill() {
        let (mercenary, _) = kineticist_jackpot_fixture();
        let query = trade_query(&mercenary, false).expect("build official trade query");
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
        assert!(query["query"]["filters"]["misc_filters"]["filters"]["ilvl"]["max"].is_null());
        assert_eq!(query["sort"]["price"], "asc");
        assert!(query.get("_poemerc").is_none());

        let search = trade_search(&mercenary, "Covens", false).unwrap();
        assert_eq!(search.selected_skill, "Kinetic Blast of Clustering");
        assert_eq!(search.included_skills, 1);
        assert_eq!(search.available_skills, 3);
        assert_eq!(search.included_filters, 5);
        assert_eq!(search.available_filters, 7);
    }

    #[test]
    fn every_skill_mode_emits_one_group_per_skill_money_skill_first() {
        let (mercenary, _) = kineticist_jackpot_fixture();
        let query = trade_query(&mercenary, true).unwrap();
        let groups = query["query"]["stats"].as_array().unwrap();
        assert_eq!(groups.len(), 3);
        assert!(groups.iter().all(|group| group["type"] == "mercenary"));
        assert_eq!(groups[0]["filters"].as_array().unwrap().len(), 5);
        let search = trade_search(&mercenary, "Covens", true).unwrap();
        assert_eq!(search.selected_skill, "Kinetic Blast of Clustering");
        assert_eq!(search.included_skills, 3);
        assert_eq!(search.included_filters, 7);
        assert!(groups[1..]
            .iter()
            .all(|group| group["filters"].as_array().unwrap().len() == 1));

        let mut six = mercenary.clone();
        for name in ["Haste", "Frost Bomb", "Blink Arrow", "Grace"] {
            six.skills.push(Skill::new(name, name));
        }
        let search = trade_search(&six, "Covens", true).unwrap();
        assert_eq!(search.available_skills, 7);
        assert_eq!(search.included_skills, LOGGED_IN_GROUP_LIMIT);
    }

    #[test]
    fn official_url_contains_one_offline_encoded_query() {
        let (mercenary, _) = kineticist_jackpot_fixture();
        let url = trade_search_url(&mercenary, "Covens", false).unwrap();
        assert!(url.starts_with("https://www.pathofexile.com/trade/search/Covens?q="));
        assert!(!url.contains(' '));
        assert!(!url.contains("xddbsns"));

        let hardcore = trade_search_url(&mercenary, "Hardcore Covens", false).unwrap();
        assert!(hardcore.contains("/Hardcore%20Covens?q="));
    }

    #[test]
    fn unresolved_support_is_dropped_and_reported() {
        let (mut mercenary, _) = kineticist_jackpot_fixture();
        mercenary.skills[0].supports[0].canonical = "ambiguous".into();
        mercenary.skills[0].supports[1].tier = SupportTier::Absent;
        let search = trade_search(&mercenary, "Covens", false).unwrap();
        assert_eq!(search.selected_skill, "Kinetic Blast of Clustering");
        assert_eq!(search.included_filters, 3);
        assert_eq!(search.dropped.len(), 2);
        assert!(search.dropped[0].contains("select the exact support"));
        assert!(search.dropped[1].contains("no visible tier"));
    }

    #[test]
    fn money_skill_beats_a_utility_row_with_more_t3_supports() {
        let (mut mercenary, _) = kineticist_jackpot_fixture();
        mercenary.skills[0].supports.truncate(1);
        mercenary.skills[2] = Skill::new("Haste", "Haste").with_supports(vec![
            ("return".into(), SupportTier::T3),
            ("gmp".into(), SupportTier::T3),
            ("edwa".into(), SupportTier::T3),
        ]);
        let search = trade_search(&mercenary, "Covens", false).unwrap();
        assert_eq!(search.selected_skill, "Kinetic Blast of Clustering");
        assert_eq!(search.included_filters, 2);

        // No money skill for the family: fall back to the best-supported row.
        mercenary.family = "other".into();
        let search = trade_search(&mercenary, "Covens", false).unwrap();
        assert_eq!(search.selected_skill, "Haste");
    }

    #[test]
    fn support_filters_are_capped_at_five_per_group() {
        let (mut mercenary, _) = kineticist_jackpot_fixture();
        mercenary.skills[0] =
            Skill::new("Kinetic Blast of Clustering", "Kinetic Blast of Clustering").with_supports(
                vec![
                    ("return".into(), SupportTier::T3),
                    ("gmp".into(), SupportTier::T3),
                    ("chain".into(), SupportTier::T2),
                    ("edwa".into(), SupportTier::T3),
                    ("faster_attacks".into(), SupportTier::T3),
                    ("pierce".into(), SupportTier::T3),
                ],
            );
        let search = trade_search(&mercenary, "Covens", false).unwrap();
        assert_eq!(search.included_filters, SAFE_GROUP_FILTER_LIMIT);
        assert_eq!(search.available_filters, 9);
        assert!(search.dropped.is_empty(), "{:?}", search.dropped);
    }

    #[test]
    fn extra_targets_hash_is_pinned_to_its_skill() {
        let lookup = |skill| support_trade_id("gilded_extra_targets", SupportTier::T3, skill);
        assert_eq!(lookup("Smite").unwrap(), "mercenary.support_37259");
        assert_eq!(lookup("Vaal Smite").unwrap(), "mercenary.support_37259");
        assert_eq!(
            lookup("Lightning Arrow").unwrap(),
            "mercenary.support_58471"
        );
        assert!(lookup("Haste").is_err());
    }

    #[test]
    fn generic_family_without_money_skill_searches_its_best_supported_row() {
        assert!(money_skills("withertouch").is_empty());
        let mercenary = Mercenary {
            class_name: "Withertouch".into(),
            family: "withertouch".into(),
            skills: vec![
                Skill::new("Haste", "Haste"),
                Skill::new("Essence Drain of Wickedness", "Essence Drain of Wickedness")
                    .with_supports(vec![("gmp".into(), SupportTier::T3)]),
            ],
            ..Default::default()
        };
        let search = trade_search(&mercenary, "Covens", false).unwrap();
        assert_eq!(search.selected_skill, "Essence Drain of Wickedness");
        assert_eq!(search.included_skills, 1);
        assert_eq!(search.included_filters, 2);
        assert!(search.dropped.is_empty(), "{:?}", search.dropped);
        let query = trade_query(&mercenary, false).unwrap();
        assert_eq!(query["query"]["type"]["option"], "ChaosMinionWitchDot");
    }

    #[test]
    fn steelskin_ally_has_its_330_trade_stat() {
        assert_eq!(
            skill_trade_id("Steelskin Ally", "").unwrap(),
            "mercenary.skill_17105"
        );
    }

    #[test]
    fn herald_of_ash_hash_is_pinned_to_its_build() {
        let lookup = |build| skill_trade_id("Herald of Ash", build).unwrap();
        assert_eq!(lookup("Chaplain of the Well"), "mercenary.skill_31201");
        assert_eq!(lookup("chaplain_of_the_well"), "mercenary.skill_31201");
        assert_eq!(lookup("Eruptor"), "mercenary.skill_44334");
        assert_eq!(lookup("Flaming Charlatan"), "mercenary.skill_44334");

        let mut chaplain = Mercenary {
            class_name: "Chaplain of the Well".into(),
            skills: vec![Skill::new("Herald of Ash", "Herald of Ash")],
            ..Default::default()
        };
        let query = trade_query(&chaplain, false).unwrap();
        assert_eq!(
            query["query"]["stats"][0]["filters"][0]["id"],
            "mercenary.skill_31201"
        );
        chaplain.class_name = "Infamous Eruptor".into();
        chaplain.infamous = true;
        let query = trade_query(&chaplain, false).unwrap();
        assert_eq!(
            query["query"]["stats"][0]["filters"][0]["id"],
            "mercenary.skill_44334"
        );
    }

    #[test]
    fn bundled_stats_are_the_current_official_snapshot() {
        let (patch, league, source) = bundled_trade_stat_provenance();
        assert_eq!(patch, "3.30");
        assert_eq!(league, "Covens");
        assert_eq!(source, "https://www.pathofexile.com/api/trade/data/stats");
        assert_eq!(trade_stats().entries.len(), 539);
    }

    /// Every `disc: mercenary_warrant` entry of `/api/trade/data/items`,
    /// fetched 2026-11-18 for 3.30 Covens, as (warrant text, type id).
    const WARRANT_TYPES_330: &[(&str, &str)] = &[
        ("Bastion", "PhysicalDuelistShields"),
        ("Blade Ambusher", "TrapsMinesShadowAttack"),
        ("Bladebitter", "Crit1HShadowPoison"),
        ("Bladecaster", "Crit1HShadowPhysSpell"),
        ("Bloodletter", "PhysicalDuelistBleed"),
        ("Cardinal", "AurasMinionsTemplarStaff"),
        ("Chaplain of the Well", "AbyssalTemplar"),
        ("Combatant", "MeleeAOEStrikeDuelistRangeStrikes"),
        ("Cruel Mistress", "ChaosMinionWitchChaosHit"),
        ("Earthshaker", "MeleeAOEMarauderPhysSlam"),
        ("Eruptor", "MeleeAOEMarauderFireSlam"),
        ("Fallen Reverend", "AurasMinionsTemplarSpectres"),
        ("Flamehand", "ElementalWitchFire"),
        ("Flamequiver", "EleBowRangerFire"),
        ("Flaming Charlatan", "PhysConvertTemplarFire"),
        ("Frost Ambusher", "TrapsMinesShadowCold"),
        ("Frosthand", "ElementalWitchCold"),
        ("Infamous Bastion", "PhysicalDuelistShieldsNoble"),
        ("Infamous Blade Ambusher", "TrapsMinesShadowAttackNoble"),
        ("Infamous Bladebitter", "Crit1HShadowPoisonNoble"),
        ("Infamous Bladecaster", "Crit1HShadowPhysSpellNoble"),
        ("Infamous Bloodletter", "PhysicalDuelistBleedNoble"),
        ("Infamous Cardinal", "AurasMinionsTemplarStaffNoble"),
        (
            "Infamous Combatant",
            "MeleeAOEStrikeDuelistRangeStrikesNoble",
        ),
        ("Infamous Cruel Mistress", "ChaosMinionWitchChaosHitNoble"),
        ("Infamous Earthshaker", "MeleeAOEMarauderPhysSlamNoble"),
        ("Infamous Eruptor", "MeleeAOEMarauderFireSlamNoble"),
        (
            "Infamous Fallen Reverend",
            "AurasMinionsTemplarSpectresNoble",
        ),
        ("Infamous Flamequiver", "EleBowRangerFireNoble"),
        ("Infamous Frosthand", "ElementalWitchColdNoble"),
        ("Infamous Kineticist", "MiscScionWandAttacksNoble"),
        ("Infamous Manyshot", "EleBowRangerClonesNoble"),
        ("Infamous Mysterious Diver", "DivingDuelistNoble"),
        ("Infamous Reanimator", "ChaosMinionWitchInstabilityNoble"),
        ("Infamous Ripper", "MeleeAOEMarauderNonSlamNoble"),
        ("Infamous Sanguimancer", "MiscScionPhysDotNoble"),
        ("Infamous Shock Ambusher", "TrapsMinesShadowLightningNoble"),
        ("Infamous Sniper", "NonEleBowRangerPhysNoble"),
        ("Infamous Stormhand", "ElementalWitchLightningNoble"),
        (
            "Infamous Storming Zealot",
            "PhysConvertTemplarLightningNoble",
        ),
        ("Infamous Striker", "MeleeStrikesMaraduerPhysNoble"),
        ("Infamous Swiftblade", "MeleeAOEStrikeDuelistCycloneNoble"),
        ("Infamous Toxicologist", "NonEleBowRangerChaosNoble"),
        ("Infamous Warpriest", "AurasMinionsTemplarSmiteNoble"),
        (
            "Infamous Warpriest of the Ruckus",
            "AurasMinionsTemplarSmiteRuckusNoble",
        ),
        ("Infamous Withertouch", "ChaosMinionWitchDotNoble"),
        ("Kineticist", "MiscScionWandAttacks"),
        ("Manyshot", "EleBowRangerClones"),
        ("Mysterious Diver", "DivingDuelist"),
        ("Reanimator", "ChaosMinionWitchInstability"),
        ("Ripper", "MeleeAOEMarauderNonSlam"),
        ("Sanguimancer", "MiscScionPhysDot"),
        ("Shattersword", "PhysicalDuelistSteel"),
        ("Shock Ambusher", "TrapsMinesShadowLightning"),
        ("Smoulderstrike", "MeleeStrikesMarauderFire"),
        ("Sniper", "NonEleBowRangerPhys"),
        ("Stormhand", "ElementalWitchLightning"),
        ("Storming Zealot", "PhysConvertTemplarLightning"),
        ("Striker", "MeleeStrikesMaraduerPhys"),
        ("Swiftblade", "MeleeAOEStrikeDuelistCyclone"),
        ("Thunderquiver", "EleBowRangerLightning"),
        ("Toxicologist", "NonEleBowRangerChaos"),
        ("Warpriest", "AurasMinionsTemplarSmite"),
        ("Winter Deacon", "PhysConvertTemplarCold"),
        ("Withertouch", "ChaosMinionWitchDot"),
    ];

    #[test]
    fn every_330_warrant_type_is_mapped_and_no_other_id_is() {
        let known: std::collections::HashSet<&str> =
            WARRANT_TYPES_330.iter().map(|(_, id)| *id).collect();
        for (label, id) in WARRANT_TYPES_330 {
            let infamous = label.starts_with("Infamous ");
            let mut mercenary = Mercenary {
                class_name: (*label).to_string(),
                infamous,
                ..Default::default()
            };
            assert_eq!(warrant_trade_type(&mercenary).unwrap(), *id, "{label}");
            // The other rarity must map to a listed id or to nothing at all.
            mercenary.infamous = !infamous;
            if let Ok(other) = warrant_trade_type(&mercenary) {
                assert!(known.contains(other), "{label} maps to unlisted {other}");
            }
        }
    }

    #[test]
    fn every_330_skill_and_attainable_support_tier_has_one_official_trade_id() {
        let mut skill_count = 0;
        for skill in catalog_skill_names() {
            skill_trade_id(skill, "").unwrap_or_else(|error| panic!("{skill}: {error}"));
            skill_count += 1;
        }
        assert_eq!(skill_count, 272);

        let support_tiers = catalog_skill_support_tiers();
        for (skill, canonical, tier) in &support_tiers {
            support_trade_id(canonical, SupportTier::from_u8(*tier), skill)
                .unwrap_or_else(|error| panic!("{canonical} T{tier} on {skill}: {error}"));
        }
        // 3.30 catalog: Steelskin Ally is a real skill again; [DNT] Unused rows stay scrubbed.
        assert_eq!(support_tiers.len(), 4_068, "attainable pair census");
    }
}
