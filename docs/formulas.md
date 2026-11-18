# Formula notes

The overlay uses the screening scores (`QK`, `QM`, `QC`) as a stop/skip filter.

I went back over those weights later. What held up and what didn't:

| Piece | Status |
|-------|--------|
| Tier factor T1=0.6 / T2=0.8 / T3=1.0 (`SupportTier::factor`) | Heuristic. Real supports have their own progressions. |
| Point weights (+25 KBoC, -20 Kinetic Bolt, and the rest) | Heuristic. Not derived from sales or DPS tests. |
| Bands skip <50 / common <55 (Combatant <60) / check <65 / good <80 / very valuable <90 / jackpot >=90 (`models::interpret_score`) | Unverified cutoffs. Useful as a UI, not as a valuation. A market-proven support package overrides the band to "jackpot" at any score. |
| AI share ≈ 1/(1+n competing skills) | Invalid as a general rule. |
| Cooldown `CD / (1+CDR)` | Confirmed. |
| Faster Attacks is *increased* attack speed | Confirmed. |
| Crit expected multiplier `1 + C(CM-1)` | Confirmed. |
| Static Strike duration `3 × more-duration` | Confirmed. |
| Return ≈ 2× damage | Only if the returning hit actually connects. Frost Blades becomes practically consistent around 150% increased projectile speed and stays geometry-dependent. |

Real support values, which the 0-100 score does not use:

| Support | I | II | III |
|---------|---|----|-----|
| Elemental Damage with Attacks | 15% more | 20% more | 30% more |
| Hypothermia (chilled enemy) | 15% more | 20% more | 30% more |
| Faster Attacks | 15% increased | 25% increased | 35% increased |
| Cooldown Recovery | 20% increased | 30% increased | 40% increased |
| More Duration | 15% more | 25% more | 40% more |
| Critical Damage | +30% multiplier | +45% | +70% |
| Critical Chance | 80% increased | 120% increased | 200% increased |

Projectile behaviour order: Split, Pierce, Fork, Chain, Return.

The app never emits a sale-price quote; the market row is a dated ask range, described below. Price-check comparable current listings yourself using class + main skill + supports bound to that skill + secondary skill + brick exclusions. Whatever number falls out of that is an asking-price signal, not a confirmed sale.

The current 3.30 market survey and the exact skill-bound gates are in [`market-3.30.md`](market-3.30.md); the patch facts and data sources are in [`research-3.30.md`](research-3.30.md). The 3.29.3 listing audit is kept in the [3.29 archive](research-3.29.md).

How the 3.30 screens differ from 3.29, with the evidence in [`market-3.30.md`](market-3.30.md):

- Stormhand is not a bespoke scorer any more. Its jackpot failed (it is a 10d family and the Arc + Ball Lightning of Static + Chain + Gilded Chain Distance package has n=1), so it is an ordinary `market::SCREENS` entry.
- Jackpot holds only for a premium family whose gate package has n >= 8, a median of at least 50d and top-share >= 0.95. That keeps Kineticist, Manyshot and Combatant and adds Cruel Mistress. Swiftblade and Shattersword lose it.
- Six families (Cardinal, Withertouch, Frosthand, Bloodletter, Ripper, Warpriest) have no screen and use `extra::score_generic`. Its estimate is capped at the family's price tier: 72 for high, 62 for mid, 52 for floor. Premium families and families with no measured tier keep the 79 cap.
- Penalties the market contradicted are removed: Icicle Rain on Manyshot (it still cannot jackpot, because it never sits beside Mirror Arrow) and Frost Blades + Wild Strike on Combatant. Fallen Reverend gains a Frostblink brick.

## Market ask estimate

`src/pricing.rs` reads `assets/warrant-prices-3.30.json`, a snapshot of the cheapest instant-buyout asks on the official trade site, collected per family and support package by `scripts/fetch-warrant-prices.ps1` (one search plus a fetch of the ten cheapest listings per row, level 83 and above only). It is compiled into the binary by `build.rs` and never refreshed at run time; the snapshot date and level floor are printed next to every estimate. The bundled snapshot was generated 2026-11-18T05:49:05Z for Covens: 193 rows (base 65, money 64, money_gates 64, of which 23 are partner-swept), at 95.29 chaos per divine (poe.ninja, 2026-11-18 05:49Z). This is the rate the market row uses. The scoring survey in `market-3.30.md` converted divine asks at 101.5 chaos per divine (poe.ninja, 2026-11-18 02:57Z) and only fed the score weights and tiers, never the displayed prices. Mysterious Diver and Warpriest of the Ruckus have 0 listings and show "no listings data". A package with no listings falls back to the next package, never to 0c. Listing counts are the trade site's search totals at ilvl 83+, instant buyout.

Refresh with `pwsh scripts/fetch-warrant-prices.ps1` and rebuild. The run takes about an hour: the script keeps every request at least 2.5 s apart and holds itself to half of every rate-limit window the trade API reports in its `X-Rate-Limit-Ip` headers, so a full sweep spends most of its time waiting on the 6-hour search window. A run that dies part way resumes from its checkpoint file.

Row selection for a scanned mercenary:

1. Keep rows whose `family` matches (case-insensitive) and whose `infamous` is null or equals the mercenary's.
2. Drop rows with `listings == 0`.
3. Take the most specific package the mercenary satisfies: `money_gates` needs the row's money skill with every gate support on that skill at the gate tier or higher; `money` needs the money skill; `base` always matches.
4. Among rows of the same package, the one with the highest median wins.

A mercenary the scorer flagged as bricked is priced off the `base` row alone (`estimate_for(merc, true)`): the brick is exactly what destroys the money package, so quoting the money row would overstate it.

What is shown: `lowest_chaos` to `p75_chaos` as the range, `median_chaos` as the typical ask, `listings`, and the median listing age. Ten divines and up are shown as whole divines, half a divine up to ten as divines to one decimal, anything below that in chaos. `thin market` is fewer than 5 listings, counting the partner-filtered matches when the row has them; `stale asks` is a median listing age over 14 days. When the snapshot carries `"placeholder": true` the UI and summary say `sample data`.

These are asks at the snapshot date, not sales. They do not feed the 0-100 score.
