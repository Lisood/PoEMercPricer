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
        family => match market::screen_for(family) {
            Some(screen) => market::score(merc, family, screen),
            None => match crate::catalog::family_listings(family) {
                Some(listings) => extra::score_generic(merc, family, listings),
                None => ScoreResult {
                    family: if family.is_empty() { "other" } else { family }.into(),
                    action: format!(
                        "{} could not be matched to a {} mercenary family",
                        if merc.class_name.is_empty() {
                            "This mercenary"
                        } else {
                            &merc.class_name
                        },
                        crate::catalog::catalog_patch_short()
                    ),
                    notes: vec![
                        format!(
                            "All {} catalog families are scored; bespoke screens: Kineticist, Manyshot, Combatant.",
                            crate::catalog::known_families().count()
                        ),
                    ],
                    ..Default::default()
                },
            },
        },
    }
}
