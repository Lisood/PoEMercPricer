mod combatant;
mod extra;
mod kineticist;
mod manyshot;
pub mod market;

use crate::models::{Mercenary, ScoreResult};

pub fn score_mercenary(merc: &Mercenary, assume_projectile_speed: bool) -> ScoreResult {
    match merc.family.to_lowercase().as_str() {
        "kineticist" => kineticist::score(merc),
        "manyshot" => manyshot::score(merc),
        "combatant" => combatant::score(merc, assume_projectile_speed),
        "stormhand" => extra::score_stormhand(merc),
        "sniper" => extra::score_sniper(merc),
        "thunderquiver" => extra::score_thunderquiver(merc),
        family => match market::screen_for(family) {
            Some(screen) => market::score(merc, family, screen),
            None => match crate::catalog::family_listings(family) {
                Some(listings) => extra::score_generic(merc, listings),
                None => ScoreResult {
                family: if family.is_empty() {
                    "other".into()
                } else {
                    family.into()
                },
                score: 0.0,
                band: "unsupported".into(),
                action: format!(
                    "{} could not be matched to a 3.29 mercenary family",
                    if merc.class_name.is_empty() {
                        "This mercenary"
                    } else {
                        &merc.class_name
                    }
                ),
                notes: vec![
                    "All 36 catalog families are scored; audited screens: Kineticist, Manyshot, Combatant, Sniper, Thunderquiver, Stormhand."
                        .into(),
                ],
                    ..Default::default()
                },
            },
        },
    }
}
