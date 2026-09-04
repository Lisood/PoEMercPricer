# 3.29 Allflame warrant market: what is actually popular and expensive

Snapshot date: 2026-09-02 (data pulled 09:46Z to 10:30Z). League: Curse of the Allflame (3.29.3), launched 2026-07-24.

## Currency context

| Source | Chaos per Divine | Fetched | URL |
|---|---|---|---|
| poe.ninja exchange overview (`rates.divine` = 0.002674) | 374c | 2026-09-02 | https://poe.ninja/poe1/api/economy/exchange/current/overview?league=Allflame&type=Currency |
| Perandus Ledger export (`divineRate`) | 391.2c | 2026-09-02T09:46Z | https://xddbsns.com/data/allflame/mercenary-warrants.json |
| Mirror of Kalandra (poe.ninja `primaryValue`) | 387,722c ≈ 1,037d | 2026-09-02 | same poe.ninja URL |

All floors below are converted at 391.25c/d (the rate the ledger ran at). Rate has drifted from 355c (2026-09-01, research-3.29.md) to 374-391c in a day, so store prices in chaos and re-convert at read time.

## Primary data and method

- Perandus Ledger public exports: per-build listing files `https://xddbsns.com/data/allflame/mercenary-build-<slug>.json` (all 35 slugs fetched) and the summary `mercenary-warrants.json` (1,079,208 listings, generated 2026-09-02T09:46:14Z). Rows are `[currency, value, "six-slot code", level-83 offset]`; decoded with the site's own `https://xddbsns.com/js/merc-filter.js` (support bound to the skill carrying it). Only level 83/84 warrants are in the ledger; Infamous and normal are pooled ("Infamous changes the odds of rolling a good mercenary, not what a rolled one is worth", https://xddbsns.com/mercenary-warrant.html).
- Official trade API (`POST https://www.pathofexile.com/api/trade/search/Allflame`, anonymous) for securable-vs-online counts on 2026-09-02.
- Guides/tier lists dated Jul 17 to Aug 30 2026 for demand signals (URLs inline). Reddit returned 403 during collection; the Reddit citations in research-3.29.md were not re-verified.

Columns: `n` = matching listings, `min` = cheapest ask, `5th` = fifth-cheapest (depth signal), `med` = median ask. Every number here is an ask, not a sale: the ledger's own code comment says "Warrants that actually leave the market clear below the standing floor" (merc-filter.js, `mercStats`).

## Ranked packages (top 14, by 5th-cheapest ask with n ≥ 5)

| # | Family | Infamous? | Money skill | Gate supports (tier) + required partner skill | n | min / 5th / med | Evidence |
|---|---|---|---|---|---|---|---|
| 1 | Kineticist | pooled | Kinetic Blast of Clustering | Return III + GMP III on KBoC; Greater Kinetic Blast as pair partner; + Haste | 20 | 500d / 700d / 850d | ledger kineticist.json 2026-09-02; IGGM "1.5 to 5 mirrors" tier (Aug 12) https://www.iggm.com/news/poe-3-29-endgame-kineticist-mercenary-guide-avoid-overpaying-10-divines-to-5-mirrors |
| 2 | Kineticist | pooled | KBoC | Return III + GMP III + GKB | 74 | 100d / 200d / 680d | ledger; research-3.29.md had 140d/230d on 09-01 |
| 3 | Kineticist | pooled | KBoC | Return III + GKB + Haste + Inspiring Cry | 117 | 41d / 80d / 300d | ledger; mmoexp "Haste priority #1, Inspiring Cry #2" (Aug 15) https://www.mmoexp.com/News/path-of-exile-kinetic-blast-mercenary-build-best-budget-build-under-5-divine.html |
| 4 | Swiftblade | pooled | Rallying Cry | Greater CDR III + Greater More Duration III (+ Pride: 28d/60d, n=25) | 37 | 20d / 50d / 150d | ledger swiftblade.json |
| 5 | Kineticist | pooled | KBoC | Return III + Greater Fork III + GKB | 131 | 19d / 50d / 300d | ledger; IGGM "Chain II / Fork III, avoid Pierce" |
| 6 | Shattersword | pooled | Rallying Cry | Greater CDR III + Greater More Duration III | 9 | 6d / 25d / 25d | ledger shattersword.json (thin) |
| 7 | Combatant | pooled | Frost Blades | Return III + Greater EDWA III + Chain II; Static Strike partner | 101 | 5d / 15d / 100d | ledger combatant.json; research-3.29.md 10d/20d on 09-01 |
| 8 | Kineticist | pooled | KBoC | Return III + Chain II + GKB | 316 | 10d / 15d / 200d | ledger |
| 9 | Manyshot | pooled | Vaal Ice Shot | Return III + Greater EDWA III + Greater Hypothermia III; Mirror Arrow partner | 89 | 350c / 15d / 100d | ledger manyshot.json; research 8d/10d on 09-01 |
| 10 | Kineticist | pooled | KBoC | Return III + GKB (no other gate) | 1,364 | 1.3d / 5d / 100d | ledger; mmoexp "~50c budget, ~20d upgraded" |
| 11 | Stormhand | pooled | Arc | Chain II + Gilded Chain Distance III; Ball Lightning of Static partner | 128 | 3d / 5d / 50d | ledger stormhand.json; research 2d/5d on 09-01 |
| 12 | Flamehand | pooled | Rolling Magma | Gilded Area per Projectile III + GMP III | 16 | 5c / 5d / 100d | ledger flamehand.json (thin, scarce family: 8,249 listings) |
| 13 | Fallen Reverend | pooled | Wrath + Zealotry (aura pair) | no support gate; + Reinforce: Fallen Bishop/Emperor 300c to 1d | 1,500 | 200c / 250c / 5d | ledger fallen-reverend.json; sportskeeda "Wrath or Zealotry" (Jul 30) https://www.sportskeeda.com/mmo/path-exile-mercenary-tier-list-3-29 |
| 14 | Manyshot | pooled | Vaal Ice Shot | Return III + Greater EDWA III; Mirror Arrow | 1,010 | 199c / 1d / 40d | ledger |

Liquid tier (large n, real floor): Manyshot VIS::Return + Mirror Arrow, n=10,052, 30c / 100c / 19d; Combatant FB::Return + Static Strike, n=5,027, 20c / 23c / 10d; Kineticist KBoC + GKB (any supports), n=14,245, 25c / 40c / 9d.

Ledger cap: asks above 1,000d are stored as 1,000d (`capDivine`); eight Kineticist rows sit at the cap. 877 Kineticist and 1,140 Manyshot rows are priced in Mirrors.

## Per-family tier table (36 catalog families)

Tier rule: premium = best skill-bound package has 5th-cheapest ≥ 5d; mid = 50c to 5d; floor = ≤ 50c. "Ask" columns are min / 5th at the stated package. The last column compares the row against `src/scoring/market.rs` SCREENS. Four rows disagreed on 2026-09-02 and were fixed the same day; they say so. One open disagreement is left, on Blade Ambusher's money skill.

| Family | Tier | Money skill + gates people search (partner) | n | min / 5th | market.rs / notes |
|---|---|---|---|---|---|
| Kineticist | premium | KBoC :: Return III (+GMP III / Chain II / Greater Fork III); GKB partner; Haste, Inspiring Cry | 1,364 | 1.3d / 5d | not in SCREENS (audited). KBoC::Return with Barrage instead of GKB is 1d/1d (n=1,676); GMP-without-Return is 1d/1d (n=1,523): Return is the gate, GMP the multiplier. Bricks: Kinetic Bolt, Kinetic Rain of Impact (pool partners) |
| Manyshot | premium | Vaal Ice Shot :: Return III (+Greater EDWA III, +Greater Hypothermia III); Mirror Arrow partner | 10,052 | 30c / 100c | not in SCREENS. Return on plain Ice Shot alone: 1c/1c (n=62,068). IS::Return+GMP with VIS::Return: 10c / 80c (n=852), well under the 50d in research-3.29.md; that premium has collapsed. GMP on VIS: 5c/10c, no gate |
| Combatant | premium | Frost Blades :: Return III (+Greater EDWA III, +Chain II); Static Strike partner; Wrath | 5,027 | 20c / 23c | not in SCREENS. Wild Strike::Return 1c; Multistrike on FB costs nothing but guides say avoid (Odealo, Aug 18) https://odealo.com/articles/mercenaries-of-trarthus-in-depth-merc-guide |
| Swiftblade | premium | Rallying Cry :: Greater CDR III + Greater More Duration III; Pride | 37 | 20d / 50d | agrees; tier-2 pair is 1c/10c (n=419), so jackpot must require both at T3 |
| Shattersword | premium (thin) | Rallying Cry :: Greater CDR III + Greater More Duration III | 9 | 6d / 25d | agrees; the Lancing Steel::Gilded Scattershot bonus was dropped, since that package is 1c/1c (n=267) |
| Stormhand | premium (thin) | Arc :: Chain II + Gilded Chain Distance III; Ball Lightning of Static partner | 128 | 3d / 5d | not in SCREENS; without BLoS partner 3c/5c (n=1,245) |
| Fallen Reverend | mid (deep) | Wrath + Zealotry; Reinforce Bishop/Emperor/Osseotitan | 1,500 | 200c / 250c | agrees; Battlemage's Cry pair 1c/3c confirms -35 brick |
| Flamehand | mid (thin) | Rolling Magma :: Gilded Area per Projectile III + GMP III | 16 | 5c / 5d | agrees; GAPP alone 5c/15c (n=100) |
| Mysterious Diver | mid | Frost Blades :: Return III + Greater EDWA III + Greater Hypothermia III | 37 | 100c / 1d | agrees (Return+GEDWA only: 1c/3c) |
| Eruptor | mid | Flame Link :: Gilded Empowered Link III (+Greater More Duration III 1d/1d n=20; +Greater CDR III 50c/1d n=24) | 217 | 3c / 40c | agrees; Brutality is a build brick not a price brick (Odealo Luminary guide, Jul 26) https://odealo.com/articles/mercenary-luminary-build |
| Bastion | mid | Impenetrable Bastion :: Greater CDR III + Greater More Duration III | 55 | 5c / 1d | agrees; add presence bonus for Determination + Intimidating Cry pair (49c/50c, n=461, warrants.json) |
| Cardinal | mid-low | Consecrated Path :: Gilded Consecration III + Greater Faster Attacks III | 41 | 5c / 100c | agrees; GC alone 1c/5c |
| Earthshaker | mid-low | Molten Shell :: Greater CDR III + Gilded Physical Damage Reduction III | 36 | 10c / 50c | agrees |
| Bloodletter | mid-low | Leap Slam :: Gilded Frenzy III + Greater Faster Attacks III | 36 | 10c / 50c | agrees; Gilded Frenzy alone 1c/1c |
| Flaming Charlatan | mid-low (thin) | Wave of Conviction of Trarthus :: Greater Added Fire III + Greater CDR III | 12 | 22c / 1d | agrees |
| Cruel Mistress | mid-low | Soulrend of Reaping :: GMP III + Return III (10c/20c, n=1,513); Envy + Zealotry + Forbidden Rite Totem + SoR 5c/30c (n=672) | 1,513 | 10c / 20c | disagreed and was fixed: Envy+Zealotry alone is 1c/3c (n=5,501), so the +25 "chaos curse-bot demand" was not priced, and Void Sphere::Gilded Sphere Frequency is 1c/1c (n=2,497). The screen is now Soulrend of Reaping with GMP and Return as gates and the aura pair as a 5+5 presence bonus |
| Warpriest | floor/mid-low | Herald of Purity :: Greater Minion Damage III + Greater Pulverise III | 192 | 5c / 20c | agrees (Dominating Blow 1c) |
| Smoulderstrike | floor | Infernal Cry :: Greater AoE III + Greater More Duration III | 42 | 1c / 30c | agrees in shape; T2 AoE is 1c/1c; akrpg calls it "highest DPS in open maps" but market does not pay https://www.akrpg.com/news/1076--poe-329-mercenary-tier-list--best-merc-for-earlymidendgame |
| Toxicologist | floor | Scourge Arrow of Menace :: Gilded Additional Pods III + GMP III | 191 | 5c / 15c | agrees; Pods alone 1c/1c |
| Sniper | floor (asks sit) | Tornado Shot :: GMP III + Gilded Secondary Shots III; Grace + Haste | 740 | 1c / 5c | disagreed and was fixed: base 40 + GMP 25 with `jackpot: true` was not supported by any ask; TS::GMP is 1c/1c (n=8,383), no-Brutality 1c/2c, and median asks of 3 to 9d are unsold. The screen is now base 15, GMP 10, no jackpot, with Brutality and Arrow Nova as penalties. Guides praise it (aoeah B-tier Jul 27 https://www.aoeah.com/news/4710--poe-329-best-mercenary-tier-list-locations--how-to-farm; an uber run on video https://www.youtube.com/watch?v=L23-BIguod0) |
| Thunderquiver | floor | Lightning Arrow :: Return III + GMP III; Wrath, Precision | 3,404 | 1c / 1c | disagreed and was fixed: `jackpot: true` was unsupported, and even +Wrath with no Storm Rain is 1c/5c (n=732). The screen is now base 15 with Return and GMP gates, no jackpot. akrpg's "sleeper" is a use-value claim, not resale |
| Flamequiver | floor | Artillery Ballista :: Gilded Totemic Onslaught III + Multiple Totems III | 126 | 1c / 10c | agrees |
| Storming Zealot | floor | Shockwave Totem of Shocking :: Gilded Astral Totem III + Multiple Totems III | 64 | 5c / 10c | agrees (20k/21k listings are Divine Ire dump) |
| Bladecaster | floor | Seismic Crush :: Greater Crit Damage III + Greater Crit Chance III | 65 | 1c / 15c | agrees |
| Winter Deacon | floor | Earthquake of Winter :: Greater Conc Effect III + Greater Hypothermia III | 98 | 1c / 10c | agrees |
| Withertouch | floor | Scourstorm :: Greater DoT Multiplier III + Greater Swift Affliction III | 134 | 1c / 4c | agrees; Malevolence adds nothing |
| Frosthand | floor | Ice Nova :: Gilded Freezer Burn III | 879 | 1c / 1c | agrees; no package clears 1c |
| Frost Ambusher | floor | Ice Trap :: Greater Cold Pen III + Greater Trigger Radius III | 107 | 1c / 2c | agrees |
| Shock Ambusher | floor | Vaal Lightning Trap :: Greater Added Lightning III; Skitterbots + Zealotry | 2,422 | 1c / 3c | agrees |
| Reanimator | floor | Raise Zombie of Falling :: Greater Minion Damage III | 1,092 | 1c / 1c | agrees |
| Blade Ambusher | floor | Blade Trap :: Greater Crit Damage III + Greater Crit Chance III (5c/5c, n=721) | 33,066 | 1c / 1c | agrees on the verdict ("great to use, worthless to sell"), but the screen names a different money skill: `market.rs` gates Bear Trap with Trigger Radius and Cooldown Recovery. Worth re-checking against the live pool. Crit and phys-as-chaos supports never appear on Spectral Helix of Trarthus; its pool is Multiple Traps, Charged Traps, Trigger Radius, Throwing Speed |
| Bladebitter | floor | Pestilent Strike :: Greater DoT Multiplier III + Greater Chance to Poison III; Grace + Malevolence | 448 | 1c / 2c | agrees |
| Sanguimancer | floor | Vaal Reap :: Gilded Searing Agony III | 506 | 1c / 2c | agrees |
| Striker | floor | Leap Slam :: Gilded Frenzy III; Physical Aegis + Determination + Vitality | 746 | 1c / 1c | agrees |
| Ripper | floor | Leap Slam (of Groundbreaking) :: Gilded Frenzy III | 747 | 1c / 1c | agrees |
| Warpriest of the Ruckus | no data | Infamous-only; ~2 listings; shares Holy Relic + Smite signature with Warpriest (merc-filter.js comment) | none | none | generic fallback stays |

## Demand signals

- Luminary (Scion) is the demand engine: "15% endgame play rate, second overall", nearly doubling from week one (mmoexp, Aug 21) https://www.mmoexp.com/News/poe-3-29-curse-of-the-allflame-meta-breakdown-top-ascendancies-build-guide-2026.html. Kineticist is "by far the most-used Mercenary"; Manyshot "the best found so far", Combatant single-target, Kineticist "very rare" (rpgstash, Jul 17) https://www.rpgstash.com/blog/poe-luminary-mercenaries-guide. Odealo names the same three carries and scales them via Flame Link / Destructive Link https://odealo.com/articles/mercenary-luminary-build.
- Flame Link Luminary is the second big archetype: "Flame Link paired with Enemy's Embrace" (mmoexp) and life-stacking Flame Link with a KBoC + GMP + Return merc (poecurrency, Jul 28) https://www.poecurrency.com/news/poe-3-29-life-stacking-flame-link-turns-mercenary-into-million-damage-flamethrower. Brutality on the merc is the build brick (Odealo, expcarry Jul 26 https://expcarry.com/poe-3-29-luminary-scion-mercenary-build).
- Aura bots: Fallen Reverend "top pick for lightning spell casters", Wrath + Zealotry warrants "sell for solid divines" (aoeah Jul 27); dual-aura mercs reach aura level 26 to 28 (poecurrency, Jul 24) https://www.poecurrency.com/news/poe-3-29-luminary-ascendancy-guide-how-to-build-powerful-permanent-mercenaries. Market confirms only Wrath+Zealotry (200c) and Swiftblade Rallying Cry T3 (20d+); Envy+Zealotry and Grace+Malevolence/Haste pairs are 1 to 5c.
- Warrant farming is a mainstream strat: "35 divines per hour" selling Cruel Mistress, Infamous Sniper, Combatant, Kineticist warrants (mmoexp, Aug 17) https://www.mmoexp.com/News/poe-3-29-mercenary-farming-guide-how-to-make-35-divines-per-hour-with-delirium-and-heist.html; "5 to 10 divines regularly, premium 25+" (mmoexp, Aug 10) https://www.mmoexp.com/News/path-of-exile-3-29-mercenary-farming-guide-best-boss-rush-strategy-for-fast-currency.html. Videos: Travic "STOP SELLING YOUR MERCENARIES TOO CHEAP" (Jul 28) https://www.youtube.com/watch?v=0htuC1oTI2o; TranePanos pricing guide linking the same ledger (Aug 11) https://www.youtube.com/watch?v=zruHV4812Gc; BawLoch "Infamous Mercenary Rushing, 100+ divines" https://www.youtube.com/watch?v=CRYQt7SPiso.
- Sells vs sits (asking-book proxy): deep books with a real floor are Manyshot VIS::Return+MA (n=10k at 30 to 100c), Combatant FB::Return+SS (n=5k at 20c), Fallen Reverend Wrath+Zealotry (n=1.5k at 200c), Kineticist KBoC+GKB (n=14k at 25 to 40c). Thin, high books (Kineticist Return+GMP n=74, Swiftblade n=37, Flamehand n=16) are where price-checks swing. Sniper/Thunderquiver/Blade Ambusher have 2 to 9d median asks over a 1c floor: those medians are unsold inventory.
- Early-league guide numbers ("10 to 50 chaos for a meta warrant", neonsect Aug 30 https://neonsect.com/path-of-exile/poe-329-mercenaries-guide/) are obsolete; use them only as a reminder that the floor was near-zero in week one.

## Why "median of cheapest listings" misleads

1. The 1-divine dump wall. 26.5% (Kineticist), 27% (Combatant), 30% (Manyshot), 30.5% (Fallen Reverend) of all level-83/84 listings sit at exactly 1d regardless of quality; the ledger notes "the median is 1 divine with or without any support, so it cannot discriminate at all" (merc-filter.js `mercSupportLift`). Use the skill-bound floor, not a family median.
2. Securable against online. Official trade on 2026-09-02, KBoC::Return: 9,747 securable against 124 online; Vaal Ice Shot::Return: 10,000+ securable against 316 online (`status.option`). The ledger reads Faustus tabs, so its floors are securable asks; an online-only check will show fewer and dearer listings. Data is "typically 10 minutes behind" https://xddbsns.com/mercenary-price-check.html.
3. Cap and mirror outliers. Asks are capped at 1,000d in the export; 0.5 to 0.8% of rows are priced in Mirrors (877 Kineticist, 1,140 Manyshot). Never let a mirror ask into a mean; use nth-cheapest only.
4. Level 84 is a different market. One L84 VIS::Return+MA listing at 225d vs an L83 median of 19d; two L84 FB::Return+SS at 2d/108d vs 20c. The ledger pools them "so a single rare 84 can set the floor for an 83's search"; the app should not.
5. Price-fixing. The classic pattern (several low asks that never sell, then a relist) is described for warrants by mmojugg (Jul 25) https://www.mmojugg.com/news/poe-mercenaries-guide.html: "Do not assign a fixed price from one listing." Require depth (5th-cheapest for pairs, 3rd for triples) and discard n≤3 books quoting 100d+ (research-3.29.md rule).
6. The Infamous premium is psychological. It is pooled in the data; "people may still pay a premium for Infamous due to lack of knowledge or vanity" https://xddbsns.com/mercenary-warrant.html. Show the flag, don't price it.
7. Anonymous trade queries are capped. One mercenary group with skill + 2 supports is complexity 37 (OK); two groups fail with "Query is too complex ... Logging in will increase this limit" (observed 2026-09-02). A support inside a `not` group is not skill-bound, and `not:true` inside a mercenary group is silently ignored (merc-filter.js).

## Collector recommendations

Query these as skill-bound (support on the named skill) packages at level 83 only, plus a separate L84 pass. Beyond `market.rs` SCREENS:

- Kineticist (not in SCREENS): pair `KBoC + Greater Kinetic Blast` (Barrage pair is 5x cheaper); `KBoC::Return`; `::Return+GMP`; `::Return+Chain`; `::Return+Greater Fork`; `::Return+GMP+Greater EDWA`; each with and without `Haste`, `Inspiring Cry`. Bricks: `Kinetic Bolt`, `Kinetic Rain of Impact`. `KBoC::GMP` without Return as the control row.
- Manyshot: `Vaal Ice Shot::Return` with `Mirror Arrow` (this pair excludes Icicle Rain and Frigid Forkshot by pool construction; sportskeeda's Frigid Forkshot wish-list is contradicted by asks); `VIS::Return+Greater EDWA`; `+Greater Hypothermia`; `Ice Shot::Return+GMP` with `VIS::Return` (80c 5th on this snapshot, keep as a mid row); plain `Ice Shot::Return` as the 1c control. Vaal Ice Shot vs Ice Shot: Return on the Vaal gem is the gate; on the base gem it is worth nothing alone.
- Combatant: `Frost Blades + Static Strike`; `FB::Return`; `+Greater EDWA`; `+Chain`; `+Wrath`; `Wild Strike::Return` control.
- Stormhand: `Arc::Chain+Gilded Chain Distance` with `Ball Lightning of Static`; the Orb-of-Storms/Wrath variants (warrants.json 5th 40 to 75c) as mid rows.
- Swiftblade and Shattersword: T3-only `Rallying Cry::Greater CDR+Greater More Duration` (+Pride for Swiftblade); T2 pair as control.
- Fallen Reverend: `Wrath + Zealotry`; `+ Reinforce: Fallen Bishop/Emperor/Osseotitan`; `Battlemage's Cry` pairs as control.
- Cruel Mistress: `Soulrend of Reaping::GMP+Return`, and `Envy+Zealotry+Forbidden Rite Totem+SoR`. These replaced the Envy-led screen in `market.rs`.
- Bastion: add `Determination + Intimidating Cry` (49c floor) beside `Impenetrable Bastion::GCDR+GMoreDur`.
- Mid families: Flamehand `RM::GAPP+GMP`; Eruptor `FL::GEL(+GMoreDur/+GCDR)`; Mysterious Diver `FB::Return+GEDWA+GHypo`; Cardinal `CP::GC+GFA`; Earthshaker `MS::GCDR+GPDR`; Bloodletter `LS::GF+GFA`; Warpriest `HoP::GMD+GPulverise`; Flaming Charlatan `WoC::GAF+GCDR`.
- Demotions applied to `market.rs` on 2026-09-02: Sniper dropped `jackpot` and fell to base 15 with GMP at 10, Thunderquiver dropped `jackpot` and fell to base 15, Cruel Mistress lost the Envy-led screen, Shattersword lost the Lancing Steel bonus. See `docs/research-3.29.md`, "2026-09-02 revisions".
- Record per row: n, min, 3rd, 5th, 10th, median, share at exactly 1d, mirror-priced count, L84 count, and the online-only total from the official API for the top ~10 rows. Refresh at least daily; the divine rate moved 10% in 24h.
