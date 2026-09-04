//! Market ask estimate from the bundled trade-site snapshot
//! (`assets/warrant-prices-3.29.json`).
//!
//! The snapshot holds, per family, the cheapest instant-buyout asks for a few
//! support "packages": `base` (any warrant of that family), `money` (has the
//! money skill) and `money_gates` (money skill with its skill-bound gate
//! supports at the required tiers). A scanned mercenary gets the most specific
//! package it satisfies. These are asks at the snapshot date, not sales.

use std::cmp::Ordering;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::models::{Mercenary, SupportTier};

const SNAPSHOT_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/warrant-prices-3.29.json"));

#[derive(Clone, Debug, Deserialize)]
pub struct Snapshot {
    pub generated_at: String,
    #[serde(default)]
    pub league: String,
    #[serde(default)]
    pub patch: String,
    #[serde(default)]
    pub source: String,
    pub chaos_per_divine: f32,
    #[serde(default)]
    pub placeholder: bool,
    /// Minimum mercenary level the searches were filtered to (0 = none).
    #[serde(default)]
    pub min_level: u32,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Row {
    pub family: String,
    #[serde(default)]
    pub infamous: Option<bool>,
    pub package: String,
    #[serde(default)]
    pub money_skill: Option<String>,
    #[serde(default)]
    pub gates: Vec<Gate>,
    #[serde(default)]
    pub listings: u32,
    #[serde(default)]
    pub sampled: u32,
    #[serde(default)]
    pub lowest_chaos: f32,
    #[serde(default)]
    pub median_chaos: f32,
    #[serde(default)]
    pub p75_chaos: f32,
    #[serde(default)]
    pub median_age_days: f32,
    #[serde(default)]
    pub fresh_share: f32,
    #[serde(default)]
    pub query_id: String,
    /// Partner-rule rows: listings fetched and those whose skill list matched
    /// the scorer's partner/brick rule (0 when the row was not filtered).
    #[serde(default)]
    pub scanned: u32,
    #[serde(default)]
    pub passed: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Gate {
    pub canonical: String,
    pub tier: u8,
}

pub fn snapshot() -> &'static Snapshot {
    static SNAPSHOT: OnceLock<Snapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        serde_json::from_str(SNAPSHOT_JSON).expect("bundled warrant-prices-3.29.json parses")
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarketEstimate {
    pub package: &'static str,
    pub money_skill: Option<String>,
    pub listings: u32,
    pub low_chaos: f32,
    pub typical_chaos: f32,
    pub high_chaos: f32,
    pub median_age_days: f32,
    pub fresh_share: f32,
    pub chaos_per_divine: f32,
    pub league: String,
    /// YYYY-MM-DD of the snapshot.
    pub snapshot_date: String,
    pub placeholder: bool,
    pub min_level: u32,
    pub scanned: u32,
    pub passed: u32,
}

/// `money_gates` > `money` > `base`; anything else is not a package we price.
fn package_rank(package: &str) -> Option<u8> {
    match package {
        "money_gates" => Some(2),
        "money" => Some(1),
        "base" => Some(0),
        _ => None,
    }
}

/// Does this mercenary carry everything the row's package requires?
pub fn satisfies(merc: &Mercenary, row: &Row) -> bool {
    if row.package == "base" {
        return true;
    }
    let Some(skill) = row
        .money_skill
        .as_deref()
        .and_then(|name| merc.skill(&[name]))
    else {
        return false;
    };
    row.package != "money_gates"
        || row.gates.iter().all(|gate| {
            skill.support_tier(&[gate.canonical.as_str()]) >= SupportTier::from_u8(gate.tier)
        })
}

pub fn estimate(merc: &Mercenary) -> Option<MarketEstimate> {
    estimate_for(merc, false)
}

/// A bricked mercenary (see `ScoreResult::bricks`) is priced off the family
/// floor: its money package is exactly what the brick destroys.
pub fn estimate_for(merc: &Mercenary, bricked: bool) -> Option<MarketEstimate> {
    let snap = snapshot();
    let row = snap
        .rows
        .iter()
        .filter(|row| row.listings > 0 && row.family.eq_ignore_ascii_case(&merc.family))
        .filter(|row| !bricked || row.package == "base")
        .filter(|row| {
            row.infamous
                .is_none_or(|infamous| infamous == merc.infamous)
        })
        .filter_map(|row| package_rank(&row.package).map(|rank| (rank, row)))
        .filter(|(_, row)| satisfies(merc, row))
        .max_by(|(rank_a, a), (rank_b, b)| {
            rank_a.cmp(rank_b).then(
                a.median_chaos
                    .partial_cmp(&b.median_chaos)
                    .unwrap_or(Ordering::Equal),
            )
        })
        .map(|(_, row)| row)?;
    Some(MarketEstimate {
        package: row.package.as_str(),
        money_skill: row.money_skill.clone(),
        listings: row.listings,
        low_chaos: row.lowest_chaos,
        typical_chaos: row.median_chaos,
        high_chaos: row.p75_chaos,
        median_age_days: row.median_age_days,
        fresh_share: row.fresh_share,
        chaos_per_divine: snap.chaos_per_divine,
        league: snap.league.clone(),
        snapshot_date: snap.generated_at.chars().take(10).collect(),
        placeholder: snap.placeholder,
        min_level: snap.min_level,
        scanned: row.scanned,
        passed: row.passed,
    })
}

/// (amount, unit): ("1.5", "div") from half a divine up, otherwise ("35", "c").
fn amount(chaos: f32, rate: f32) -> (String, &'static str) {
    let div = if rate > 0.0 { chaos / rate } else { 0.0 };
    if div >= 10.0 {
        (format!("{div:.0}"), "div")
    } else if div >= 0.5 {
        (format!("{div:.1}"), "div")
    } else {
        (format!("{chaos:.0}"), "c")
    }
}

/// "1.5 div" when the value is at least half a divine, else "35c".
pub fn currency(chaos: f32, rate: f32) -> String {
    let (n, unit) = amount(chaos, rate);
    if unit == "div" {
        format!("{n} div")
    } else {
        format!("{n}c")
    }
}

impl MarketEstimate {
    /// "≈ 4.5–9.0 div", "≈ 5–60c" or "≈ 60c–1.5 div".
    pub fn range_label(&self) -> String {
        let (low, low_unit) = amount(self.low_chaos, self.chaos_per_divine);
        let (high, high_unit) = amount(self.high_chaos, self.chaos_per_divine);
        if low == high && low_unit == high_unit {
            format!("≈ {}", currency(self.low_chaos, self.chaos_per_divine))
        } else if low_unit == high_unit {
            if low_unit == "div" {
                format!("≈ {low}–{high} div")
            } else {
                format!("≈ {low}–{high}c")
            }
        } else {
            format!(
                "≈ {}–{}",
                currency(self.low_chaos, self.chaos_per_divine),
                currency(self.high_chaos, self.chaos_per_divine)
            )
        }
    }

    pub fn confidence(&self) -> &'static str {
        let depth = if self.passed > 0 {
            self.passed
        } else {
            self.listings
        };
        if depth < 5 {
            "thin market"
        } else if self.median_age_days > 14.0 {
            "stale asks"
        } else {
            ""
        }
    }

    /// The trade API caps `total` at 10,000.
    fn listed(&self) -> String {
        if self.listings >= 10_000 {
            "10000+".into()
        } else {
            self.listings.to_string()
        }
    }

    /// "12 listed · typical ask 9.0 div · median listing age 4 d · Allflame snapshot 2026-09-02"
    pub fn detail_line(&self) -> String {
        let league = if self.league.is_empty() {
            "trade".to_string()
        } else {
            self.league.clone()
        };
        let level = if self.min_level > 0 {
            format!(" · level {}+", self.min_level)
        } else {
            String::new()
        };
        let matched = if self.passed > 0 {
            format!(
                " · {} of {} cheapest match this package",
                self.passed, self.scanned
            )
        } else {
            String::new()
        };
        format!(
            "{} listed{level}{matched} · typical ask {} · median listing age {:.0} d · {league} snapshot {}{}",
            self.listed(),
            currency(self.typical_chaos, self.chaos_per_divine),
            self.median_age_days,
            self.snapshot_date,
            if self.placeholder {
                " (sample data)"
            } else {
                ""
            }
        )
    }

    /// One line for the copied summary and the CLI.
    pub fn summary_line(&self) -> String {
        let confidence = self.confidence();
        format!(
            "Market: {} ({} listed, snapshot {}{}{})",
            self.range_label(),
            self.listed(),
            self.snapshot_date,
            if self.placeholder {
                ", sample data"
            } else {
                ""
            },
            if confidence.is_empty() {
                String::new()
            } else {
                format!(", {confidence}")
            }
        )
    }
}

/// Summary/CLI line whether or not the snapshot covers this build.
pub fn summary_line(estimate: Option<&MarketEstimate>) -> String {
    estimate.map_or_else(
        || "Market: no listings data for this build".to_string(),
        MarketEstimate::summary_line,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_switches_units_at_half_a_divine() {
        assert_eq!(currency(35.0, 200.0), "35c");
        assert_eq!(currency(99.0, 200.0), "99c");
        assert_eq!(currency(100.0, 200.0), "0.5 div");
        assert_eq!(currency(300.0, 200.0), "1.5 div");
        assert_eq!(currency(1900.0, 200.0), "9.5 div");
        assert_eq!(currency(9000.0, 200.0), "45 div");
        assert_eq!(currency(50.0, 0.0), "50c");
    }

    fn sample(low: f32, high: f32) -> MarketEstimate {
        MarketEstimate {
            package: "base",
            money_skill: None,
            listings: 12,
            low_chaos: low,
            typical_chaos: (low + high) / 2.0,
            high_chaos: high,
            median_age_days: 4.0,
            fresh_share: 0.5,
            chaos_per_divine: 200.0,
            league: "Allflame".into(),
            snapshot_date: "2026-09-02".into(),
            placeholder: false,
            min_level: 0,
            scanned: 0,
            passed: 0,
        }
    }

    #[test]
    fn range_label_shares_a_unit_when_it_can() {
        assert_eq!(sample(900.0, 1800.0).range_label(), "≈ 4.5–9.0 div");
        assert_eq!(sample(5.0, 60.0).range_label(), "≈ 5–60c");
        assert_eq!(sample(60.0, 300.0).range_label(), "≈ 60c–1.5 div");
    }

    #[test]
    fn confidence_words() {
        let mut e = sample(5.0, 60.0);
        assert_eq!(e.confidence(), "");
        e.median_age_days = 15.0;
        assert_eq!(e.confidence(), "stale asks");
        e.listings = 4;
        assert_eq!(e.confidence(), "thin market");
    }

    #[test]
    fn detail_and_summary_lines() {
        let mut e = sample(900.0, 1800.0);
        assert_eq!(
            e.detail_line(),
            "12 listed · typical ask 6.8 div · median listing age 4 d · Allflame snapshot 2026-09-02"
        );
        assert_eq!(
            e.summary_line(),
            "Market: ≈ 4.5–9.0 div (12 listed, snapshot 2026-09-02)"
        );
        e.placeholder = true;
        assert!(e.detail_line().ends_with(" (sample data)"));
        assert!(e.summary_line().ends_with(", sample data)"));
        assert_eq!(
            summary_line(None),
            "Market: no listings data for this build"
        );
    }
}
