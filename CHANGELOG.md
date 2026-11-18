# Changelog

All notable changes to PoEMercPricer are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow [Semantic Versioning](https://semver.org/). The release workflow publishes the section for the tagged version as the top of the GitHub release notes, so write each section for players.

## [0.2.1] - TODO(date)

Path of Exile 3.30.0 (Covens of Umbra) support.

### Added

- 3.30 catalog: 37 builds, 272 active skills and 152 support arts. The new build is Chaplain of the Well. Five skills are newly recognised: Abyssal Spray, Abyssal Spray Detonation, Power of the Depths, Vaal Smite and Steelskin Ally (Steelskin Ally was missing from the 3.29 catalog too, and only rolls on Swiftblade). Support art is unchanged, and no build or skill was removed.
- A scoring screen for Chaplain of the Well.
- The overlay label and `--help` now show the patch the catalog was built for.

### Changed

- The default league is Covens. A saved Allflame league is moved to its Covens equivalent the first time you start this version, and the settings file is rewritten, so your trade searches keep working without a visit to Settings.
- Warrants are scored and priced again from a 3.30 trade-site market survey. 3.30 changed Mercenary scaling and several supports (for example Kinetic Blast's Arcane Might is now 150%, down from 200%), so the 3.29 gates and prices were rebuilt rather than carried over. Scores, verdicts and market ask ranges will differ from 0.2.0:
  - Cruel Mistress is now a jackpot family. Sniper and Thunderquiver move up to premium.
  - Swiftblade and Shattersword lose their jackpot screens.
  - Stormhand is scored by the same market screen as the other families instead of its own scorer.
  - Cardinal, Withertouch, Frosthand, Bloodletter, Ripper and Warpriest no longer have a screen of their own and use the generic estimate.
  - Manyshot's Icicle Rain penalty and Combatant's Frost Blades + Wild Strike penalty are gone, because the market pays for those packages. Manyshot still cannot jackpot with Icicle Rain. Fallen Reverend gets a Frostblink brick.
  - Details and evidence are in `docs/market-3.30.md`.
- Market data now comes from the official trade site only. Trade stats, warrant types, listing counts and prices are collected there, and a listing count is the site's search total at item level 83+, instant buyout. This replaces the third-party ledger counts used in 3.29.
- A package with no listings falls back to the next package or shows "no listings data" instead of a price of 0c.
- Active-skill levels worked out from the mercenary level are now shown as estimates with a tilde, such as `Lvl ~26`. The formula was measured in 3.29.3 and could not be checked for 3.30, because the 3.30 trade data has no skill levels. A level read from the panel is shown as before.

### Fixed

- Trade searches use the right Herald of Ash stat for each build. 3.30 added a second stat with the same name: one is for Chaplain of the Well, and Flaming Charlatan and Eruptor keep the old one. Vaal Smite's Gilded Extra Targets searches use Smite's stat.
- Warrants copied from the game no longer confuse the parser. `Limit`, `Portal`, `Area Level` and `Item Tier` lines are ignored wherever they appear.
- Screen scans no longer read an `Area Level` line as the Mercenary level, which would have given the wrong skill level.
- Screen scans recognise skills that the OCR pools were missing: Puncture, Frenzy and Haste on Sniper, Proximity Shield on Stormhand, and the nine Chaplain of the Well skills.
- The generic estimate is capped at its family's price tier (72 for high, 62 for mid, 52 for floor), so a family without a screen cannot outscore one with a screen.
- Settings are saved atomically, so a crash or power loss while saving cannot leave a half-written settings file. Only the whole word "Allflame" triggers the league migration.

### Notes

- 3.30 warrants open a Proving Grounds portal and are no longer used with a Map.
- Releases are still unsigned, and the app still makes no network request other than the update check.

## [0.2.0] - 2026-09-05

### Added

- A per-user Windows installer, `poemercpricer-setup-windows-x64.exe`. It installs under `%LOCALAPPDATA%\Programs\PoEMercPricer`, adds a Start menu entry and an optional desktop shortcut, and does not need administrator rights. The portable exe is still published. Portable copies share your settings with the installed app.

### Changed

- In-app updates are stricter. The download is checked against the size and SHA-256 digest GitHub publishes before anything is replaced, the running copy is swapped in place, and a competing copy cannot start a second update at the same time. A failed download or check leaves the installed app untouched.
- Each release now ships its third-party notices, and updates keep a copy beside the app.

## [0.1.0] - 2026-09-04

First public release.

- In-game overlay that screens Path of Exile 3.29 Mercenary Warrants from a copied item, a clipboard image or a screen scan of the inspect panel.
- Reads active skills with Windows OCR and support gems with icon recognition, then gives a 0-100 jackpot or skip verdict with the reasons.
- Market ask range from a bundled trade-site snapshot, a pre-filled official trade search, a copyable summary, and a hotkey (`Ctrl+Shift+M` by default).
- Checks GitHub Releases for updates at startup and every 6 hours.
