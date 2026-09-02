# Formula notes

The overlay uses the **screening scores** (`QK`, `QM`, `QC`) as a stop/skip filter.

A later audit of those weights:

| Piece | Status |
|-------|--------|
| Tier factor T1=0.6 / T2=0.8 / T3=1.0 | Heuristic. Real supports have their own progressions. |
| Point weights (+25 KBoC, −40 Icicle Rain, …) | Heuristic. Not derived from sales or DPS tests. |
| Bands 50 / 65 / 80 / 90 | Unverified cutoffs. Useful as a UI, not as a valuation. |
| AI share ≈ 1/(1+n competing skills) | Invalid as a general rule. |
| Cooldown `CD / (1+CDR)` | Confirmed. |
| Faster Attacks is *increased* attack speed | Confirmed. |
| Crit expected multiplier `1 + C(CM−1)` | Confirmed. |
| Static Strike duration `3 × more-duration` | Confirmed. |
| Return ≈ 2× damage | Only if the returning hit actually connects. Frost Blades becomes practically consistent around 150% increased projectile speed and remains geometry-dependent. |

Real support values (not used in the 0–100 score):

| Support | I | II | III |
|---------|---|----|-----|
| Elemental Damage with Attacks | 15% more | 20% more | 30% more |
| Hypothermia (chilled enemy) | 15% more | 20% more | 30% more |
| Faster Attacks | 15% increased | 25% increased | 35% increased |
| Cooldown Recovery | 20% increased | 30% increased | 40% increased |
| More Duration | 15% more | 25% more | 40% more |
| Critical Damage | +30% multiplier | +45% | +70% |
| Critical Chance | 80% increased | 120% increased | 200% increased |

Projectile behaviour order: Split → Pierce → Fork → Chain → Return.

The app does not emit a sale-price quote. Price-check comparable current listings using class + main skill + supports bound to that skill + secondary skill + brick exclusions. Any resulting figure is an asking-price signal, not a confirmed sale.

The current 3.29.3 listing audit and exact skill-bound gates are documented in [`research-3.29.md`](research-3.29.md).
