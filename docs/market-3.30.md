# 3.30 Covens warrant market: survey and scoring recalibration

Snapshot date: 2026-11-18 (survey run 03:41Z onward). League: Covens (3.30.0), launched 2026-11-13, so this is day 5.
This file replaces `docs/market-3.29.md` as the source for `src/scoring/`. Every weight in `src/scoring/market.rs` carries an inline comment with the numbers below, and the tests in `tests/scoring.rs` cite rows here.

## Currency context

| Source | Chaos per Divine | Fetched | Note |
|---|---|---|---|
| poe.ninja exchange overview (Covens, Currency) | 101.5c | 2026-11-18 | thin: 21 receive listings, 32 pay listings |
| Mirror of Kalandra | none | 2026-11-18 | poe.ninja lists the currency but has no Covens price line yet |

## Data and method

- Source: the official trade API, league Covens, `status: securable` (instant buyout), item level 83 or higher. All 4,340 listings are mercenary level 83.
- Sample: for each of the 65 warrant types (normal and Infamous are separate types), the 50 most expensive asks plus, for the 62 types with more than 50 listings, the 20 cheapest. Duplicates were removed. Infamous and normal types are pooled per family, as in 3.29. Families come from the build name via `assets/catalog-3.30.json`.
- Supports were mapped to catalog canonicals by dropping the `Greater ` / `Lesser ` prefix and matching the catalog name (`Area of Effect` is `Increased Area of Effect`). Every survey support resolved. One skill, `Steelskin Ally` (50 listings), is not in the catalog.
- Currency: 2,919 divine, 1,266 chaos and 155 mirror asks. Divine asks are converted at 101.5c.
- Outlier filter: mirror asks (155) and divine asks of 1,000d or more (2, including one at 99,999,999d) are dropped from every median and p75. They still count as top-50 asks when computing top-share (below), because their rank is all the filter trusts. 1,000d is also the cap the 3.29 ledger used.
- These are asks, not sales. Day-5 prices are unsettled, and the expensive side includes aspirational listings.

### Statistics

- `n` = cleaned (numeric) asks; `+mirror` = mirror asks dropped from the numbers.
- `med` / `p75` = median and 75th percentile of the cleaned asks.
- `top-share` = share of a feature's listings (mirrors included) that sit in their type's top-50 rather than its cheapest-20 band. The baseline is 50/70 = 0.71: a support that sellers ignore lands at about 0.71, a value gate lands near 1.00, and a brick lands well below 0.5.
- The middle of each type's book (for example the 4,600 Kineticist listings between the cheapest 20 and the dearest 50) was not sampled. So "not priced" below means "no difference between the bottom and the top of the book", not proof that the feature is worthless.

### Rules (applied by script, then checked by hand)

1. Minimum sample: a weight changes only when the feature has n >= 8 cleaned asks. Below that, the 3.29 value stays.
2. Positive weight (gate or skill bonus):
   - confirmed: top-share >= 0.85 and median >= family median. Keep it.
   - not priced: top-share <= 0.71 and median <= family median. Remove it.
   - brick: top-share <= 0.50 and median <= half the family median.
   - anything else: undecided, so the 3.29 value stays.
3. Penalty or brick: measured among warrants that carry the money skill. It is removed when the penalised feature reaches confirmed level (>= 0.85 and >= family median), confirmed when it meets the brick test, and otherwise kept.
4. Money skill: a skill on 95% or more of a family's warrants is "universal" and kept, since it carries no signal. When a non-universal money skill is not priced or a brick, it is swapped for a skill whose package reaches top-share >= 0.90 with n >= 8. With no such skill, the family loses its screen and falls back to `extra::score_generic` (support tiers + trade demand, capped at 79, flagged as an estimate).
5. New weights need stronger evidence than kept ones: top-share >= 0.90, n >= 8 and median >= family median, at most two new gates and two new skills per screen. New gates get 12 points (the median 3.29 positive gate weight), new skill bonuses 7 (the median 3.29 skill bonus), and new bricks -35 (the 3.29 Battlemage's Cry brick, sized for a ~97% price loss).
6. Family tier comes from the median of the family's top-50 asks: premium >= 50d, high 15-50d, mid 5-15d, floor < 5d. The full package (base + all positive gates at T3 + positive skills) targets the score band whose wording fits that price: premium 79 ("good", the non-jackpot cap), high 72 ("good"), mid 62 ("check"), floor 52 ("common"). When the 3.29 full package is below target, the base absorbs the difference and kept gate weights stay as they were. When it is above target, every positive weight is scaled down by the same factor.
7. Jackpot holds only for a premium family whose gate package (money skill + every positive gate at T2+) has n >= 8, median >= 50d and top-share >= 0.95. Jackpot screens target a full package of 85 and get no new gates, since their gates are the jackpot package.

Caveat on rule 6: top-50 depth differs by family. A type with 4,600 listings shows its top 1%, while one with 170 shows its top 30%, so tiers favour deep books. The cheapest-20 medians (3c to 15c everywhere) do not separate families at all.

## Family ranking (top-50 median ask)

- premium: Chaplain of the Well 300d (+34 mirror), Manyshot 200d (+27), Kineticist 200d (+32), Combatant 100d (+12), Thunderquiver 100d (+7), Cruel Mistress 50d, Sniper 50d.
- high: Withertouch 30d, Swiftblade 29.5d, Smoulderstrike 23d, Blade Ambusher 20d, Bladebitter 20d, Warpriest 15d.
- mid: Toxicologist 12d, Striker 12d, Shattersword 11d, Bastion 10.5d, then Stormhand, Fallen Reverend, Flamequiver, Cardinal, Bladecaster, Bloodletter and Shock Ambusher at 10d, Flamehand 9.5d, Winter Deacon 6d, and Frosthand, Storming Zealot, Frost Ambusher, Ripper and Sanguimancer at 5d.
- floor: Earthshaker 3.5d, Eruptor 3d, Reanimator 2d, Flaming Charlatan 2d.
- no data: Mysterious Diver and Warpriest of the Ruckus (0 listings).

Compared with 3.29: Thunderquiver, Sniper and Cruel Mistress moved from floor or mid-low up to premium; Withertouch and Blade Ambusher moved from floor to high; Eruptor and Flaming Charlatan fell to floor.

## Bespoke screens (kineticist.rs, manyshot.rs, combatant.rs)

| Family | Package | n (+mirror) | med / p75 | top-share | Decision |
|---|---|---|---|---|---|
| Kineticist | KBoC present / absent | 63 (+28) / 45 (+4) | 200d / 250d vs 3c / 5c | 0.93 / 0.31 | KBoC stays the gate (25) |
| Kineticist | KBoC::Return T3 + GMP T3 | 14 (+8) | 200d / 295d | 1.00 | jackpot holds; weights unchanged |
| Kineticist | ... with / without Greater Kinetic Blast | 7 (+3) / 7 (+5) | 200d / 200d | 1.00 / 1.00 | n < 8: GKB stays in the jackpot gate (3.29) |
| Kineticist | KBoC + Kinetic Bolt | 12 (+7) | 150d / 208d | 0.79 | undecided: -20 brick kept |
| Kineticist | KBoC + Kinetic Rain of Impact / Power Siphon | 6 (+3) / 0 | 113d / - | 0.89 / - | n < 8: bricks kept |
| Manyshot | Vaal Ice Shot present / absent | 63 (+16) / 50 (+11) | 200d / 250d vs 5c / 5c | 0.97 / 0.38 | VIS stays the gate |
| Manyshot | VIS::Return + Mirror Arrow | 31 (+9) | 200d / 263d | 1.00 | would qualify for jackpot on its own |
| Manyshot | VIS::Return + T3 EDWA + T3 Hypothermia (3.29 jackpot sub-gate) | 2 (+1) | 250d / 275d | 1.00 | n < 8: kept from 3.29, not re-proven |
| Manyshot | VIS + Icicle Rain | 14 (+2) | 175d / 288d | 0.88 | penalty contradicted: the -40 brick is removed. Icicle Rain never appears with the Vaal Ice Shot + Mirror Arrow jackpot pair (n=0; by pool construction, docs/market-3.29.md), so it still cannot jackpot |
| Manyshot | VIS + Frigid Forkshot | 0 | - | - | n < 8: -4 kept |
| Combatant | Frost Blades + Static Strike, no Spectral Helix | 47 (+4) | 100d / 150d | 1.00 | core confirmed |
| Combatant | FB::Return T3 + Chain T2+ + EDWA | 10 (+2) | 150d / 179d | 1.00 | jackpot holds (T3 EDWA n=6, kept from 3.29) |
| Combatant | Frost Blades + Wild Strike | 20 (+3) | 100d / 131d | 0.87 | penalty contradicted: the -28 penalty and brick are removed. FB+WS never carries Static Strike (n=0), so no jackpot |
| Combatant | FB Chain + Pierce | 16 (+1) | 100d / 100d | 0.76 | undecided: -12 kept |
| Combatant | Spectral Helix beside FB / WS | 16 (+1) / 19 (+1) | 5c / 3c | 0.47 / 0.25 | brick confirmed (-20) |
| Combatant | Wild Strike multistrike | 30 (+5) | 100d / 118d | 0.69 | undecided: -8 kept |
| Combatant | WS T3 EDWA + FA + Hypothermia (WS jackpot) | 0 | - | - | n < 8: kept from 3.29. WS without FB asks 5c (n=34, top-share 0.50) |
| Combatant | FB multistrike | 41 (+4) | 100d / 150d | 0.89 | not penalised; note reworded |

## Per-family table (market.rs screens and generic fallbacks)

Package columns describe the money skill. A gate list reads `support points`. "full" is the score of the full package at T3. Per-gate evidence is in the inline comments in `src/scoring/market.rs`.

| Family | Tier | n (+mirror) | family med / p75 | top-50 med / p75 | cheap-20 med | money skill: n, top-share, med/p75 | 3.29 screen | 3.30 decision |
|---|---|---|---|---|---|---|---|---|
| chaplain_of_the_well | premium | 34 (+34) | 5c / 300d | 300d / 338d | 5c | n=34+34mir t=0.71 5c/300d | new family (generic estimate) | Abyssal Spray base 55; return +12, faster_casting +12 (full 79) |
| thunderquiver | premium | 63 (+7) | 80d / 100d | 100d / 106d | 3c | n=63+7mir t=0.71 80d/100d | Lightning Arrow base 15; return +15, gmp +10; Galvanic Arrow -25 | Lightning Arrow base 47; return +15, gmp +10; Wrath +7, Galvanic Arrow -25 (full 79) |
| sniper | premium | 134 (+6) | 25d / 50d | 50d / 80d | 3c | n=66+2mir t=0.84 50d/95d | Tornado Shot base 15; gmp +10, brutality -40, arrow_nova -20 | Tornado Shot base 45; gmp +10, brutality -40, gilded_secondary_shots +12, arrow_nova +12 (full 79) |
| cruel_mistress | premium | 135 (+5) | 40d / 52d | 50d / 70d | 10c | n=70+5mir t=0.88 45d/69d | Soulrend of Reaping base 15; gmp +22, return +18; Envy +5, Zealotry +5 | Soulrend of Reaping base 40; gmp +22, return +18; Envy +5; **jackpot** (full 85) |
| withertouch | high | 139 (+1) | 20d / 40d | 30d / 50d | 5c | n=25 t=0.64 20d/50d | Scourstorm base 20; dot_multiplier +15, cdr +8, swift_affliction +8; Malevolence +8 | **generic estimate** (no screen) |
| swiftblade | high | 140 (+0) | 20d / 40d | 30d / 50d | 9c | n=140 t=0.71 20d/40d | Rallying Cry base 30; cdr +30, more_duration +25; jackpot | Rallying Cry base 25; cdr +25, more_duration +21 (full 71) |
| smoulderstrike | high | 66 (+4) | 15d / 30d | 23d / 50d | 4c | n=24+1mir t=0.80 21d/48d | Infernal Cry base 20; aoe +18, more_duration +15 | Infernal Cry base 39; aoe +18, more_duration +15 (full 72) |
| bladebitter | high | 138 (+2) | 10d / 29d | 20d / 40d | 5c | n=69 t=0.74 10d/25d | Pestilent Strike base 15; faster_attacks +10, chance_to_poison +8, dot_multiplier +10, ailment_effect +8; Malevolence +6 | Pestilent Strike base 48; faster_attacks +10, chance_to_poison +8; Malevolence +6 (full 72). Ailment Effect removed: the 3.30 catalog has no Pestilent Strike tier for it, so its sweep rows had 0 listings. Added Chaos on PS (n=32, 0.88) misses the 0.90 add bar, so there is no replacement |
| blade_ambusher | high | 137 (+3) | 12d / 30d | 20d / 50d | 3c | n=51+2mir t=0.75 15d/30d | Bear Trap base 5; trigger_radius +10, cdr +10 | Bear Trap base 62; trigger_radius +10 (full 72) |
| warpriest | high | 137 (+3) | 10d / 23d | 15d / 30d | 4c | n=100+1mir t=0.71 10d/25d | Herald of Purity base 8; minion_damage +18, pulverise +12, aoe +8, brutality +8 | **generic estimate** (no screen) |
| striker | mid | 139 (+1) | 10d / 20d | 12d / 25d | 4c | n=43+1mir t=0.75 10d/20d | Leap Slam base 8; gilded_frenzy +25, brutality +18 | Leap Slam base 19; gilded_frenzy +25, brutality +18 (full 62) |
| toxicologist | mid | 135 (+5) | 10d / 21d | 12d / 40d | 4c | n=44 t=0.82 10d/21d | Scourge Arrow of Menace base 15; gilded_additional_pods +18, gmp +12, mirage_archer +8, physical_as_extra_chaos +8 | Scourge Arrow of Menace base 16; gilded_additional_pods +18, gmp +12, mirage_archer +8, physical_as_extra_chaos +8 (full 62) |
| shattersword | mid | 70 (+0) | 10d / 20d | 11d / 34d | 5c | n=70 t=0.71 10d/20d | Rallying Cry base 25; cdr +28, more_duration +22; jackpot | Rallying Cry base 40; more_duration +22 (full 62) |
| bastion | mid | 140 (+0) | 10d / 15d | 10d / 25d | 5c | n=72 t=0.96 10d/25d | Impenetrable Bastion base 20; cdr +18, more_duration +15; Intimidating Cry +8 | Prismplate base 54; Intimidating Cry +8 (full 62) |
| fallen_reverend | mid | 140 (+0) | 8d / 11d | 10d / 20d | 15c | n=140 t=0.71 8d/11d | Reinforce: Fallen Bishop base 20; Wrath +20, Zealotry +20, Battlemage's Cry -35 | Reinforce: Fallen Bishop base 22; Wrath +20, Zealotry +20, Battlemage's Cry -35, Frostblink -35 (full 62) |
| cardinal | mid | 139 (+1) | 6d / 15d | 10d / 20d | 5c | n=85+1mir t=0.64 3d/10d | Consecrated Path base 20; gilded_consecration +20, faster_attacks +12, aoe +8 | **generic estimate** (no screen) |
| shock_ambusher | mid | 139 (+1) | 5d / 10d | 10d / 20d | 5c | n=139+1mir t=0.71 5d/10d | Vaal Lightning Trap base 25; added_lightning +15, more_duration +12 | Vaal Lightning Trap base 47; added_lightning +15 (full 62) |
| bladecaster | mid | 139 (+1) | 5d / 10d | 10d / 20d | 5c | n=31 t=0.68 6d/20d | Seismic Crush base 18; crit_damage +12, crit_chance +10, brutality +8, aoe +6 | Seismic Crush base 26; crit_damage +12, crit_chance +10, brutality +8, aoe +6 (full 62) |
| bloodletter | mid | 140 (+0) | 5d / 13d | 10d / 25d | 4c | n=78 t=0.69 5d/10d | Leap Slam base 8; gilded_frenzy +28, faster_attacks +18 | **generic estimate** (no screen) |
| flamequiver | mid | 139 (+1) | 7d / 14d | 10d / 18d | 4c | n=33 t=0.85 10d/25d | Artillery Ballista base 20; gilded_totemic_onslaught +18, aoe +8, fire_penetration +8, multiple_totems +12 | Artillery Ballista base 19; gilded_totemic_onslaught +17, aoe +8, fire_penetration +8, multiple_totems +11 (full 63) |
| stormhand | mid | 138 (+2) | 10d / 15d | 10d / 20d | 5c | n=41+1mir t=0.86 10d/20d | bespoke `extra.rs`: Arc 40, Chain 30, GCD 20, FC 5, BLS 15; jackpot | Arc base 20; chain +15, gilded_chain_distance +10, faster_casting +3, lucky_lightning_damage +6; Ball Lightning of Static +8 (full 62) |
| flamehand | mid | 70 (+0) | 5d / 10d | 10d / 14d | 5c | n=19 t=0.79 7d/10d | Rolling Magma base 25; gilded_area_per_projectile +25, gmp +15 | Rolling Magma base 20; gilded_area_per_projectile +20, gmp +12; Anger +5, Vaal Flameblast +5 (full 62) |
| winter_deacon | mid | 68 (+2) | 5d / 10d | 6d / 13d | 5c | n=39+1mir t=0.72 5d/10d | Earthquake of Winter base 20; concentrated_effect +15, hypothermia +15, freeze_chance +8 | Earthquake of Winter base 39; concentrated_effect +15, freeze_chance +8 (full 62) |
| storming_zealot | mid | 140 (+0) | 3d / 6d | 5d / 10d | 5c | n=48 t=0.73 5d/8d | Shockwave Totem of Shocking base 10; gilded_astral_totem +25, more_duration +10, aoe +10, multiple_totems +15 | Shockwave Totem of Shocking base 9; gilded_astral_totem +22, more_duration +9, aoe +9, multiple_totems +13 (full 62) |
| frosthand | mid | 140 (+0) | 5d / 10d | 5d / 10d | 5c | n=79 t=0.71 5d/7d | Ice Nova base 20; gilded_freezer_burn +18, cold_penetration +12, ailment_effect +8 | **generic estimate** (no screen) |
| frost_ambusher | mid | 69 (+1) | 3d / 6d | 5d / 10d | 4c | n=30+1mir t=0.68 4d/6d | Ice Trap base 18; cold_penetration +15, trigger_radius +12, trap_and_mine_damage +10 | Ice Trap base 47; cold_penetration +15 (full 62) |
| sanguimancer | mid | 140 (+0) | 2d / 6d | 5d / 10d | 5c | n=32 t=0.78 4d/10d | Vaal Reap base 20; gilded_searing_agony +18, dot_multiplier +10, cdr +10 | Vaal Reap base 24; gilded_searing_agony +18, dot_multiplier +10, cdr +10 (full 62) |
| ripper | mid | 138 (+2) | 2d / 5d | 5d / 10d | 5c | n=104+1mir t=0.71 2d/5d | Leap Slam base 8; gilded_frenzy +28, pulverise +10, brutality +10 | **generic estimate** (no screen) |
| earthshaker | floor | 138 (+2) | 2d / 5d | 4d / 9d | 8c | n=66 t=0.79 2d/6d | Molten Shell base 15; cdr +20, gilded_physical_damage_reduction +20; Burrow +5 | Ground Slam base 40; fortify +12 (full 52) |
| eruptor | floor | 140 (+0) | 1d / 5d | 3d / 5d | 10c | n=27 t=0.70 2d/5d | Flame Link base 25; gilded_empowered_link +25, more_duration +8, cdr +8, brutality -30 | Flame Link base 18; gilded_empowered_link +18, more_duration +6, cdr +6; Herald of Ash +5 (full 53). Brutality -30 removed: Flame Link cannot roll it in 3.30 |
| flaming_charlatan | floor | 70 (+0) | 1d / 4d | 2d / 6d | 5c | n=20 t=0.65 2d/6d | Wave of Conviction of Trarthus base 20; added_fire +20, cdr +15 | Wave of Conviction of Trarthus base 17; added_fire +17, cdr +13; Meteor +6 (full 53) |
| reanimator | floor | 140 (+0) | 1d / 4d | 2d / 5d | 5c | n=64 t=0.81 1d/5d | Raise Zombie of Falling base 18; minion_damage +15, added_chaos +10, melee_physical_damage +10 | Summon Raging Spirit base 40; minion_damage +12 (full 52) |
| mysterious_diver | no data | 0 | - | - | - | - | Frost Blades base 10; edwa +20, return +20, hypothermia +15 | kept (no data) |

### Notable decisions and their evidence

- Chaplain of the Well (new): Abyssal Spray, Abyssal Spray Detonation and Power of the Depths are on all 70 sampled warrants, so the skills alone separate nothing. 34 of the 50 dearest asks are mirrors, and the cheapest 20 sit at 2-5c. Alone, Return and Faster Casting on Abyssal Spray reach top-share 0.83 (n=11+12 mirror) and 0.84 (n=20+28). Together they reach 0.94 (n=8+10 mirror, 300d median), the only package that clears 0.90. GMP on Abyssal Spray is not priced (0.68, n=13+5). Screen: base 55 + Return 12 + Faster Casting 12 = 79, with no jackpot (0.94 < 0.95).
- Sniper: Arrow Nova on Tornado Shot flips from a -20 brick to a +12 gate (n=28+1, top-share 0.97, 50d). Brutality on Tornado Shot (n=34+1, 0.74, 38d median against a 25d family median) leans against the 3.29 -40 brick but does not reach the 0.85 bar, so the brick stays. TS::GMP + Arrow Nova is n=6, too thin for a jackpot.
- Thunderquiver: Lightning Arrow is on every warrant. Return on it is confirmed (0.86, 100d). Wrath is added (+7, 0.92, n=22+2). Galvanic Arrow is undecided (0.64, 75d), so its -25 stays. Return + GMP is n=5, so no jackpot.
- Cruel Mistress: SoR::GMP + Return is n=15+5 mirror, 50d median, top-share 1.00, so jackpot (new). Zealotry is not priced (0.71, 35d < 40d) and is removed. Envy is undecided (0.80) and stays at 5.
- Swiftblade and Shattersword lose jackpot. Swiftblade's RC::CDR + More Duration is a 30d median (n=33, top-share 1.00), below the 50d bar. Shattersword is a mid family, and CDR on its Rallying Cry is not priced (0.70, n=20).
- Stormhand moves from bespoke `extra.rs` into `market.rs` SCREENS, because its jackpot (the only reason it needed bespoke code) fails: it is a 10d family, and Arc + BLS + Chain + Gilded Chain Distance is n=1. Arc + BLS is n=13 at 10d, top-share 1.00. Lucky Lightning Damage on Arc is added (0.93, n=14+1). All points are scaled by 0.51 to the mid target.
- Bastion: Impenetrable Bastion is a brick (n=25, top-share 0.32, 10c median), and so are its CDR (0.50) and More Duration (0.30). Prismplate replaces it as the money skill (n=72, 0.96, 10.5d).
- Earthshaker: Molten Shell is not priced (0.67, n=53+1). Ground Slam::Fortify replaces it (n=13, top-share 1.00, 5d).
- Reanimator: Raise Zombie of Falling is not priced (0.67) and so are all three of its gates. Summon Raging Spirit::Minion Damage replaces it (n=42, 0.93, 2d).
- Fallen Reverend: Frostblink is a new -35 brick (n=40, top-share 0.47, 40c median against 7.5d). Battlemage's Cry is undecided (0.62, 5d), so its -35 stays.
- Generic fallback (screen removed): Cardinal (Consecrated Path 0.64, 3d median against 6d; none of the other skills reaches 0.90), Withertouch (Scourstorm 0.64, all three gates not priced), Frosthand (Ice Nova 0.71 at the family median), Bloodletter (Leap Slam 0.69), Ripper (Leap Slam 0.71, Pulverise and Brutality not priced) and Warpriest (Herald of Purity 0.71, three of four gates not priced). Nothing in these families separates top asks from bottom ones, so a screen would be invented, not measured.
- Generic estimate cap: `extra::score_generic` caps each estimate at its family's rule-6 tier target, so de-screened families cannot outrank screened ones. The cap is 72 for high (Withertouch, Warpriest), 62 for mid (Cardinal, Frosthand, Bloodletter, Ripper) and 52 for floor. Premium families and families with no measured tier (Warpriest of the Ruckus) keep the old 79 cap. Example: a Ripper with six T2 supports estimated about 72 ("good") before the cap and 62 ("check") after it.
- Catalog check: every gate must be rollable on its money skill at the tier it gates (T2 for positive market gates, the demanded tier for bespoke ones), per `skill_tiers` in `assets/catalog-3.30.json` and enforced by `every_gate_is_attainable_on_its_money_skill`. Two gates failed and were removed: Ailment Effect on Pestilent Strike (Bladebitter) and Brutality on Flame Link (Eruptor).
- Mysterious Diver (0 listings) keeps its 3.29 screen. Warpriest of the Ruckus (0 listings) stays generic.

## Unconfirmed or open

- Thunderquiver and Chaplain are premium families without a jackpot, while Cruel Mistress (50d) jackpots. Their packages are only thin (n=5 and top-share 0.94), not cheaper. Re-survey at a later league week.
- The Manyshot (T3 EDWA + T3 Hypothermia), Combatant (T3 EDWA, Wild Strike) and Kineticist (Greater Kinetic Blast) jackpot sub-gates are carried over from 3.29 untested (n < 8).
- Sniper's Brutality brick and Thunderquiver's Galvanic Arrow brick sit in the undecided zone but lean against the brick.
- The 3.30.0 patch notes (mercenaries deal up to 20% less attack damage at level 83; Kinetic Blast's Arcane Might is 150%, down from 200%; Greater Brutality and Greater Trap and Mine Damage are down to 25%, and Pulverise is up to 25%) were read as context only. No weight moved on them.
- The price sweep in `scripts/fetch-warrant-prices.ps1` samples the cheapest listings per package and will measure the unsampled middle of the books. Its 3.30 family, partner and price-floor tables follow the screens above.
