# Formula notes

The overlay uses the screening scores (`QK`, `QM`, `QC`) as a stop/skip filter.

I went back over those weights later. What held up and what didn't:

| Piece | Status |
|-------|--------|
| Tier factor T1=0.6 / T2=0.8 / T3=1.0 (`SupportTier::factor`) | Heuristic. Real supports have their own progressions. |
| Point weights (+25 KBoC, -40 Icicle Rain, and the rest) | Heuristic. Not derived from sales or DPS tests. |
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

The current 3.29.3 listing audit and the exact skill-bound gates are in [`research-3.29.md`](research-3.29.md).

## Market ask estimate

`src/pricing.rs` reads `assets/warrant-prices-3.29.json`, a snapshot of the cheapest instant-buyout asks on the official trade site, collected per family and support package by `scripts/fetch-warrant-prices.ps1` (one search plus a fetch of the ten cheapest listings per row, level 83 and above only). It is compiled into the binary by `build.rs` and never refreshed at run time; the snapshot date and level floor are printed next to every estimate. The bundled snapshot was generated 2026-09-02T12:59:33Z for Allflame, 192 rows, at 298.1 chaos per divine.

Refresh with `pwsh scripts/fetch-warrant-prices.ps1` and rebuild. The run takes about an hour: the script keeps every request at least 2.5 s apart and holds itself to half of every rate-limit window the trade API reports in its `X-Rate-Limit-Ip` headers, so a full sweep spends most of its time waiting on the 6-hour search window. A run that dies part way resumes from its checkpoint file.

Row selection for a scanned mercenary:

1. Keep rows whose `family` matches (case-insensitive) and whose `infamous` is null or equals the mercenary's.
2. Drop rows with `listings == 0`.
3. Take the most specific package the mercenary satisfies: `money_gates` needs the row's money skill with every gate support on that skill at the gate tier or higher; `money` needs the money skill; `base` always matches.
4. Among rows of the same package, the one with the highest median wins.

A mercenary the scorer flagged as bricked is priced off the `base` row alone (`estimate_for(merc, true)`): the brick is exactly what destroys the money package, so quoting the money row would overstate it.

What is shown: `lowest_chaos` to `p75_chaos` as the range, `median_chaos` as the typical ask, `listings`, and the median listing age. Ten divines and up are shown as whole divines, half a divine up to ten as divines to one decimal, anything below that in chaos. `thin market` is fewer than 5 listings, counting the partner-filtered matches when the row has them; `stale asks` is a median listing age over 14 days. When the snapshot carries `"placeholder": true` the UI and summary say `sample data`.

These are asks at the snapshot date, not sales. They do not feed the 0-100 score.
