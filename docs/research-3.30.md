# 3.30 patch facts and data sources

Game-data target: Path of Exile 3.30.0, Covens of Umbra, launched 2026-11-13. Allflame (3.29) has ended. The 3.29 research is kept as the [3.29 archive](research-3.29.md) and is not updated; the 3.29 data files were deleted from `assets/`. The market survey and the scoring it drives are in [`market-3.30.md`](market-3.30.md).

## Sources

- [3.30.0 patch notes](https://www.pathofexile.com/forum/view-thread/4013680): launch facts and the Mercenary changes quoted below.
- [PoEDB Version 3.30.0](https://poedb.tw/us/Version_3.30.0): the patch's data changes.
- [PoEDB Mercenary Warrant](https://poedb.tw/us/Mercenary_Warrant): warrant item layout and Mercenary data.
- [Trade league list](https://www.pathofexile.com/api/trade/data/leagues): league ids.
- [Trade stats](https://www.pathofexile.com/api/trade/data/stats): Mercenary stat ids for skills, supports and tiers.

## What changed in 3.30

### League

The trade league ids are `Covens`, `Hardcore Covens`, `Ruthless Covens` and `HC Ruthless Covens`. The default league in the app is `Covens`. A saved league containing the whole word "Allflame" is migrated to its Covens equivalent and the config file is rewritten (saved atomically). A private league such as "Allflamers (PL123)" is left alone.

### Mercenary changes

From the patch notes, verbatim:

- "Hired Mercenaries now have lower Attack Damage scaling at high levels, dealing up to 20% less Attack Damage at level 83."
- "Hired Mercenaries now have lower Damage Reduction scaling at high levels."
- "Hired Mercenaries now get more additional base Life scaling with level, and more increased Maximum Life scaling with level."
- "They receive -30% to all Resistances in level 46-67 Areas, and -60% to all Resistances in level 68+ Areas and Hideouts."

### Skill and support changes that affect Mercenaries

- Kinetic Blast: Arcane Might is now 150% (was 200%).
- Values changed for Ailment Damage, Brutality, Greater Brutality, Pulverise, Lesser Pulverise, Trap and Mine Damage, and Greater Trap and Mine Damage.

These were read as context only. The scoring was recalibrated from a 3.30 trade-site market survey rather than patched by hand. See [`market-3.30.md`](market-3.30.md).

## Warrant item layout

PoEDB shows new lines on the 3.30 warrant: `Limit: 1`, `Portal: The Proving Grounds` and `Area Level: 83` (warrants open a Proving Grounds portal and are no longer used with a Map). A real 3.30 Ctrl+C, taken from trade listing text, has none of them. The copied layout is:

1. Header: `Item Class: Map Fragments`, `Rarity: Normal`, `Mercenary Warrant`.
2. The mercenary name section.
3. `Build` and `Mercenary Level`.
4. One section per skill, with `Name (Tier: N)` support lines.
5. Usage text and `Note`.

The trade JSON reports item level 0, but the trade site's ilvl filter selects the Mercenary Level. The clipboard parser ignores `Limit`, `Portal`, `Area Level` and `Item Tier` wherever they appear, so it handles either form. OCR never reads an `Area Level` line as the Mercenary level. The Mercenary level still sets the active-skill level (83 gives 26, 84 gives 27).

## Catalog

`assets/catalog-3.30.json`: 37 builds, 272 skills, 152 support arts.

- New build: Chaplain of the Well.
- New skills: Abyssal Spray, Abyssal Spray Detonation, Power of the Depths and Vaal Smite. Steelskin Ally was added too; it was missing from the 3.29 catalog as well, and it only rolls on Swiftblade.
- Support art is byte-identical to 3.29. No build or skill was removed.
- No 3.30 patch note changes Mercenary gem levels, so the skill-level progression measured in 3.29.3 (`src/models.rs`) is kept. Its source thread can no longer be read and the 3.30 trade data has no skill levels, so it is unverified for 3.30: inferred levels are flagged as estimates and shown with a tilde (`Lvl ~26`).
- PoEDB's scraper broke on a markup change (`decoding="async"` on the support images) and was fixed.

### Support-context audit

`pwsh scripts/audit-support-context.ps1 -SummaryOnly -OutputPath <path>` (PoEDB only) covered 37 of 37 build pages. 4,773 of 4,914 contexts resolve exactly (97.13%); 141 (2.87%) stay ambiguous because the art is byte-identical. The largest ambiguity sets are Minion Damage / Minion Life (48), Cooldown Recovery / DoT Multiplier (30) and Throwing Speed / Trigger Radius (25).

### OCR pool audit

The active-skill pools used to read skill names were audited against the trade survey and PoEDB. Both sources agree on every change:

- Sniper gained Puncture, Frenzy and Haste.
- Stormhand gained Proximity Shield.
- Chaplain of the Well has a new pool of 9 skills (the survey and PoEDB match exactly).

## Trade stats

`assets/trade-stats-3.30.json` has 539 entries. Five skill stats were added since 3.29 and none were removed: Abyssal Spray, Abyssal Spray Detonation, Power of the Depths, Vaal Smite, and a second Herald of Ash.

- Herald of Ash is not a new skill. 3.30 added a second stat with the same name: 31201 is on Chaplain of the Well, and 44334 stays on Flaming Charlatan and Eruptor. Both were verified live on the trade site. Trade searches pick the right one per build.
- Vaal Smite's Gilded Extra Targets support shares Smite's stat, 37259. Verified live.

Refresh with `pwsh scripts/fetch-trade-stats.ps1 -League Covens -Patch 3.30 -Out assets/trade-stats-3.30.json`.

## Warrant types

65 warrant types: 64 unchanged from 3.29, plus `AbyssalTemplar` (Chaplain of the Well), which exists as an ordinary type only (no Infamous variant).

## Data-collection method

Trade stats, warrant types, listing counts and prices come from the official trade site. The third-party ledger counts used in 3.29 are gone: a listing count is the trade site's search total at item level 83 and above, instant buyout (`securable`).

- Rate limits: the trade API allows 600 searches per 6 hours. The scripts share one HTTP helper (`scripts/trade-http.ps1`) that keeps requests apart and stays inside the windows the API reports in its rate-limit headers. The 3.30 run used about 450 of 600 searches in the window and saw no HTTP 429.
- Filter: item level 83+ only.
- Privacy: only prices, counts and listing ages are kept. Requests do not ask for seller or account fields, and no seller names are written to `assets/` or the repo.
- Scripts: `scripts/fetch-trade-stats.ps1` (stat ids), `scripts/fetch-warrant-prices.ps1` (prices and counts), `scripts/fetch-icons.ps1 -Patch 3.30.0 -League Covens -Out assets/catalog-3.30.json` (catalog and icons).

### Price snapshot

`assets/warrant-prices-3.30.json`, refreshed 2026-11-24: 193 rows (base 65, money 64, money_gates 64), 21 of them partner-swept. Rows with no listings fall back to the next package or show "no listings data", never 0c. Mysterious Diver and Warpriest of the Ruckus have 0 listings.

### Currency rates

Two poe.ninja rates are in use, and they feed different things:

| Rate | Read | Feeds |
|---|---|---|
| 140.4 chaos per divine | poe.ninja, 2026-11-24 09:58Z | the bundled price snapshot and the market row in the overlay (95.29 in the 2026-11-18 snapshot) |
| 101.5 chaos per divine | poe.ninja, 2026-11-18 02:57Z | the scoring survey in `market-3.30.md` (converting divine asks to chaos for medians and tiers) |

The survey rate is not a price shown to users. Snapshot prices are stored in chaos, so the displayed ranges only depend on the snapshot rate.

## Next league port checklist

Each of these names the league or the patch and has to change when the next league lands:

1. `src/config.rs`: the default `trade_league`, and the retired-league word in `retired_league_successor` (the whole word to migrate from, and its successor).
2. `src/main.rs`: the `dump-trade-query` default league.
3. Asset filenames, which carry the patch: `build.rs`, `src/catalog.rs`, `src/trade.rs` and `src/pricing.rs` all name `catalog-`, `trade-stats-` and `warrant-prices-<patch>.json`.
4. Script defaults: the `-Patch` and `-League` parameters of `scripts/fetch-icons.ps1`, `fetch-trade-stats.ps1` and `fetch-warrant-prices.ps1`, the previous-catalog path in `fetch-icons.ps1`, and `scripts/audit-support-context.ps1`.
5. Hardcoded test counts: the build count in `src/catalog.rs` and `tests/samples.rs`, and the stat entry and skill counts in `src/trade.rs` tests.
6. Docs: this file, `market-3.30.md`, the README and the version strings. The old patch's research and market files stay as an archive.
