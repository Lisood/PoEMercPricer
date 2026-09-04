# 3.29 OCR and market audit

Audit date: 2026-09-01. Game-data target: Path of Exile 3.29.3, Allflame.

## Sources and scope

- [PoEDB's 3.29 Mercenary data](https://poedb.tw/us/Mercenaries) supplies the current build, skill, support-pool, and icon vocabulary extracted from the game data.
- [Perandus Ledger's warrant price checker](https://xddbsns.com/mercenary-price-check.html) describes its public-stash methodology and limitations. The static export I used was generated at `2026-09-01T08:36:26Z`: 1,075,504 listed Allflame warrants and a 361.04 chaos/Divine conversion.
- [Current 3.29.3 gem-level datamine and tests](https://www.reddit.com/r/pathofexile/comments/1vu5px8/some_minor_clarifying_datadetails_on_mercenary/) support level 26 skills on level-83 mercenaries and level 27 on level-84 mercenaries. PoEDB exposes the level 1-26 progression on level-83 build pages such as [Combatant](https://poedb.tw/us/Combatant).
- [PoB templates for all 36 Mercenary types](https://www.reddit.com/r/PathOfExileBuilds/comments/1vsebbi/here_are_pob_templates_for_all_36_mercenary_types/) give an independent build/active-skill cross-check. The post's 36 names exactly match the bundled catalog, and its author reports comparing 8-9 templates with in-game stat sheets. It leaves supports out on purpose, so support art and tiers are still verified from current PoEDB data and screenshots here.
- [Nerotox's tested carry-mercenary guide index](https://www.reddit.com/r/PathOfExileBuilds/comments/1vita7a/guide_on_how_to_search_for_carry_mercs/) independently picks Manyshot, Combatant, and Kineticist as the three meta carry families, and documents the competing-skill/AI constraints.

The market numbers below are asking-price floors, not completed sales. They are evidence for which exact skill-bound combinations the market distinguishes. They are not a promise that a warrant will sell at that price.

## Proven skill-bound money gates

I decoded the listing rows using the public export's six-slot encoding and filtered with each support bound to its owning skill. That matters: a flat search for a support anywhere on the warrant is not the same query.

| Family | Required skill package | Support gate | Current observed market signal |
|---|---|---|---|
| Kineticist | Kinetic Blast of Clustering + Greater Kinetic Blast | Return on KBoC | 5d floor; five-deep floor 6d |
| Kineticist | same | Return + GMP on KBoC | 140d floor; five-deep floor 230d |
| Kineticist | same | Return + GMP + Greater EDWA on KBoC | four listings; 500d floor (very thin) |
| Kineticist | KBoC + Barrage | Return on KBoC | 1d floor; five-deep floor 2d |
| Manyshot | Vaal Ice Shot + Mirror Arrow | Return on Vaal Ice Shot | 100c floor |
| Manyshot | same | Return + Greater EDWA on Vaal Ice Shot | five-deep floor 1d |
| Manyshot | same | Return + Greater EDWA + Greater Hypothermia on Vaal Ice Shot | 8d floor; five-deep floor 10d |
| Manyshot | same, plus Ice Shot | GMP + Return on Ice Shot with the Vaal gate | 50d floor; five-deep floor 99d |
| Combatant | Frost Blades + Static Strike | Return on Frost Blades | 15c floor |
| Combatant | same | Return + Greater EDWA on Frost Blades | five-deep floor 140c |
| Combatant | same | Return + Greater EDWA + Chain on Frost Blades | 10d floor; five-deep floor 20d |
| Stormhand (sleeper) | Arc + Ball Lightning of Static | Chain + Gilded Chain Distance on Arc | 2d floor; five-deep floor 5d |

So the app treats skill + attached support + tier as the recognition unit. Seeing Return somewhere else on the mercenary is not enough.

Stormhand is labelled a market sleeper rather than a proven top carry. The exact Arc package has a measurable listing premium, but current combat-testing reports disagree about its boss performance.

## Frost Blades Return geometry

Return stays market-positive even when equipped projectile speed is unknown. Current testing puts roughly 135% increased projectile speed in the partial/position-dependent region, with about 150% as the practical consistency target. Walls and arena geometry can make it connect below that point. The screen keeps its resale value but doesn't describe it as an unconditional 2x damage multiplier.

Evidence: [135% partial observation](https://www.reddit.com/r/PathOfExileBuilds/comments/1vcb3kn/), [smooth behavior closer to 155%](https://www.reddit.com/r/PathOfExileBuilds/comments/1ve5bbu/), and [open-arena limitations](https://www.reddit.com/r/PathOfExileBuilds/comments/1vayehq/).

## OCR implementation consequences

- The bundled catalog contains all 36 currently listed builds and 267 current skill names. Its build list exactly matches the independent 36-template PoB index, so valuable skills and their brick competitors share the same OCR vocabulary.
- Skill art and support art are generated from current PoEDB CDN paths, with one exception. PoEDB still serves the pre-3.26 swirl art for Return, the player Returning Projectiles gem, and it does not match the green boomerang the 3.29 client draws for `Return III`. So `assets/icons/supports/return__returnprojectiles.webp` is hand-pinned in `$pinnedIcons` in `scripts/fetch-icons.ps1` and is skipped even by `-RefreshCatalog`. Replace it by hand from a screenshot if the art ever changes.
- Active skill level is derived from Mercenary Level for the level-83/84 resale market. Support power is read separately from the visible Roman tier I/II/III.
- Recognition has to preserve the row relationship between an active skill and its attached supports. Scores never use a support recognized on another row.
- Keeping OCR small is a maintained constraint: the native Windows API receives only the panel text column, while 152 support identities collapse to 65 cached pixel templates. See [performance.md](performance.md).

## Shared-art accuracy audit

The catalog generator records every support's compatible active skills from the
36 current PoEDB build pages. The reproducible 3.29.3 audit evaluated 4,832
distinct build + skill + source-art + tier contexts:

- Skill + source art alone resolves 4,073 contexts.
- Adding the visible Roman tier resolves 4,691 contexts (97.08%), an extra 618 exact results.
- Adding the recognised build identity resolves 0 more contexts. Wherever a skill occurs in several current builds, PoEDB exposes the same support pool for it.
- 141 contexts (2.92%) stay physically ambiguous because several eligible supports use byte-identical source art at the same tier.
- The largest irreducible groups are Minion Damage / Minion Life (48 contexts), Cooldown Recovery / DoT Multiplier (30), Throwing Speed / Trigger Radius (25), Combustion / Ignite Chance (7), and Ironwood / Physical as Extra (6).

All 72 effects sharing the generic MercGold art become exact after context
filtering. Static-image OCR has to keep every surviving candidate for the 141
irreducible contexts. It must not emit `gem`, silently drop the cell, or promote
a probability to a financially consequential exact result.

Holy Flame Totem shows the hard limit. Its current
[Flaming Charlatan page](https://poedb.tw/us/Flaming_Charlatan) lists both
Ironwood and Physical as Extra at tiers I, II, and III, and all six entries use
the same `MercSilverStrIntSupportGem` CDN artwork. Skill, build, tier,
and icon pixels can't distinguish the two. A visible or hovered tooltip title
resolves the instance definitively; without tooltip text, the honest result is
the two-name ambiguity.

Audit source: [PoEDB Mercenaries](https://poedb.tw/us/Mercenaries) and its 36
current per-build pages, fetched 2026-09-02. Reproduce the census with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\audit-support-context.ps1 -SummaryOnly
```

The PoB-template post explicitly omits supports, so it is an active-skill
cross-check and not support-name evidence. Support compatibility comes from
PoEDB's current per-skill support pools.

For Flaming Charlatan specifically, PoEDB's 3.29 mercenary table lists the
wrapped active skill as `Wave of Conviction of Trarthus` and confirms its tier
III supports use the exact labels `Greater Added Fire III` and
`Greater Area of Effect III`. The scanner keeps stable canonical ids for
scoring, while its UI uses those tier-specific in-game labels.

Source: https://poedb.tw/us/Flaming_Charlatan

### Wrapped and house-specific active skills

The current 3.29.3 mercenary loadout table displays 11 `of Trarthus` active
skills. Azadi, Bardiya, Cyaxan, and Keita use bespoke skill names rather than an
invented `of <House>` suffix, so recognition works off the complete catalog
name instead of a hard-coded house-name rule. The scanner also covers long
transfigured, trap, Empire, spectre, and reinforcement names through the same
catalog-backed wrapped-line path.

PoEDB's Mercenaries footer still contains the pre-3.29 name `Dark Pact of
Trarthus`; 3.29 renamed that reward gem to `Dark Bargain of Trarthus`. Neither
suffix form appears in a current mercenary loadout (Cruel Mistress displays base
`Dark Bargain`), so it is excluded from the screen-scanner catalog on purpose.
Sources: [current mercenary loadouts](https://poedb.tw/us/Mercenaries),
[current compatibility data](https://poedb.tw/us/Spellblade_Support), and the
[3.29 rename announcement mirror](https://store.steampowered.com/news/posts/?enddate=1784679286&feed=steam_community_announcements).

The [official live trade-stat API](https://www.pathofexile.com/api/trade/data/stats) currently exposes 155 normalized Mercenary
support filter names. The attainable 36-build PoEDB pools contain 151 names.
`Area of Effect` is an alias of `Increased Area of Effect`, while Excommunicate,
Gilded Malediction, and Gilded Onslaught on Cry exist as trade filters but don't
occur in any current attainable build/skill pool. They are tracked as reserved
filters rather than assigned to visible artwork without evidence.

Regenerate the catalog and the artwork with:

```powershell
.\scripts\fetch-icons.ps1 -RefreshCatalog
```

That rewrites `assets/catalog-3.29.json` and refetches every icon under
`assets/icons` except the pinned Return template. Without `-RefreshCatalog` the
script only fills in files that are missing. Rebuild afterwards: `build.rs`
minifies the catalog and embeds every icon into the exe, so nothing is read
from `assets/` at run time.

## Non-meta family market screens (2026-09-01 snapshot)

The per-family screens in `src/scoring/market.rs` were mined from the Perandus
Ledger static exports (`xddbsns.com/data/allflame/mercenary-warrants.json` and
per-build `mercenary-build-<slug>.json`; the six-slot listing encoding matches the
site's `js/merc-filter.js`). Snapshot 2026-09-01T12:06:54Z, 1,075,743 listings,
divine = 355.4c. Floors are asking prices, not sales. Depth rule: 5th-cheapest
for pairs, 3rd for triples; n=3 floors quoting 100d+ were discarded as cap-priced
asks. Cross-checked against 3.29 tier lists (aoeah 4710, akrpg 1076, odealo
Luminary, sportskeeda). Two recurring patterns are encoded: the Gilded-III support on the
family's signature skill is the near-universal money gate, and utility packages
(Rallying Cry CDR+Duration, dual auras, Leap Slam + Gilded Frenzy, Molten Shell +
Gilded PDR) outvalue DPS packages on most non-meta families. Warpriest of the
Ruckus has one cap-priced listing and no screen, so it falls back to the generic estimate.
Re-mine after major market shifts and rebalance the tables.

## 2026-09-02 revisions from ledger cross-check

See `docs/market-3.29.md` for the data. Scorer changes made on this date:

- Sniper and Thunderquiver lose `jackpot` and drop to floor weights: Tornado Shot::GMP is 1c/1c (n=8,383) and Lightning Arrow::Return+GMP 1c/1c (n=3,404); their multi-divine median asks are unsold inventory.
- Cruel Mistress re-centred on Soulrend of Reaping::GMP+Return (10c/20c, n=1,513); Envy+Zealotry alone is 1c/3c, so the aura pair is now a 5+5 presence bonus.
- Shattersword's Lancing Steel bonus removed (Gilded Scattershot 1c/1c, n=267).
- Manyshot: Ice Shot::Return+GMP beside Vaal Return no longer counts as a jackpot path (50d floor on 09-01, 10c/80c on 09-02). Vaal Ice Shot::Return + T3 EDWA + T3 Hypothermia with Mirror Arrow remains the premium row.
