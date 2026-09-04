# UI design notes

PoEMercPricer is a focused Windows utility. The interface is built around one decision: whether a scanned Mercenary Warrant is worth creating and price-checking.

## Design rules

- Put the decision, score, and action before supporting detail.
- Express the action as an explicit verb phrase. A conflicting build must read `SKIP: BRICKED` and name the exact bricking skill. Colour alone is not enough.
- Use the in-game mercenary panel as the visual reference: near-black inset surfaces, thin bronze double borders, warm ivory text, restrained orange-gold emphasis. Colour supplements text labels and never carries meaning alone.
- Three palettes (Standard, Dark, Light) plus Follow Windows live in one `Palette` struct in `app.rs`; tokens, type sizes, spacing and breakpoints are specified in `docs/ui-guide.md`, and the contrast test covers every palette.
- Keep the command bar and result actions stable while the result body scrolls.
- Keep the verdict fixed. Only the bounded skills pane scrolls. Skill rows are collapsed summaries by default, and a bricking row is highlighted and expanded automatically.
- Draw status dots and disclosure chevrons as vector primitives rather than font glyphs, so missing-glyph squares and platform-dependent triangles can't appear. Every skill row reserves fixed chevron and icon columns; non-expandable rows show `No supports` but keep the same name baseline.
- The bounded skills pane uses a solid scrollbar with a minimum 36 px handle and an explicit, unboxed scroll cue. Both appear only when the skills actually overflow the pane; nothing implies hidden content when everything is visible. Support counts and tier distributions are primary row information and have to stay readable at rest, not hidden in low-contrast microcopy or decorative badges.
- Wrap every skill and support group. The minimum supported window width is 440 effective pixels, and no result should need horizontal scrolling.
- Show active-skill levels and support tiers next to their names. When identical artwork leaves several catalog-backed identities, show each viable tier-specific name as an inline selection button; applying a choice has to immediately recompute the verdict and copied summary. Long candidates wrap instead of getting clipped.
- Keep advanced diagnostics out of the normal scan path.
- Use existing 3.29 skill/support art beside readable names and tiers. Icons load lazily, cache once, and never replace text.
- Use plain labels such as `Skip`, `Price-check`, and `Not scored`. Avoid unexplained abbreviations, web-dashboard styling, gradients, glass effects, and decorative animation.
- Treat the game-derived serif as display type for names, verdicts, and section titles only. Operational guidance, actions, and helper text use the proportional UI face without decorative italics.
- Reserve coloured outlines and badges for semantic states such as `BRICK`. Don't turn ordinary metadata, counts, or navigation hints into cards or pills. Bronze double keylines are kept only where they echo the in-game mercenary panel hierarchy.
- Use colons, periods, and short clauses in operational copy. Avoid ornamental dash-heavy phrasing and generic product language.
- The market row inside the verdict box shows a dated ask range from the bundled snapshot, with `thin market`, `stale asks`, or `sample data` spelled out in text. The range takes the verdict accent only on take-item bands; otherwise it uses the normal text colour, so colour never rates a price on its own. When the snapshot has no row for the build, the row says so instead of disappearing.
- Don't show a generic green dot or a synthetic `Ready` state. The status line states the current task or outcome directly, such as `Press Ctrl+Shift+M to scan a mercenary panel`, `Scan complete (clipboard) in 131 ms`, or `Scan failed: ...`. Short scans use text rather than animated progress UI, and colour is never the only carrier of state.
- A scan counts as running for 30 seconds (`SCAN_WATCHDOG`). The OS OCR call and the window capture can block forever, so past that the disabled buttons come back, the status says the previous scan was abandoned, and the next Scan starts a fresh worker. Nothing is killed and nothing polls: the UI asks for one repaint at the deadline.
- No modal, no countdown, and no toast for updates: the command-bar button and the Settings > Updates section are the whole surface.
- Never restart automatically and never install on exit.
- "Install updates automatically" decides only whether an available update proceeds to install without a click; it never decides when the restart happens.
- The update buttons are ordinary buttons in tab order, and Escape still closes Settings.
- `Restart to update` and `Update to X` use the same gold-on-fill pair as `Scan`; update state text uses the normal text and muted colours, and the Failed line uses red with wording that says `Could not update`.
- The update worker requests exactly one repaint per state change. The "Checked N min ago" text is computed at paint time from an elapsed timer and never schedules its own repaint.
- Fixture mode behaves like a normal launch; the startup update check is already skipped by the debug-build rule during development.
- The 6-hour re-check uses `request_repaint_after`, never a timer thread or per-frame polling. A background result writes the status line only when there is something to do (`installed`, `available`); a failed background check goes to the console.

Spacing, type hierarchy, fluid sizing, keyboard access, and contrast targets follow Microsoft guidance for Windows desktop applications:

- [Content layout and spacing](https://learn.microsoft.com/en-us/windows/apps/design/basics/content-basics)
- [Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography)
- [Keyboard interactions](https://learn.microsoft.com/en-us/windows/apps/develop/input/keyboard-interactions)
- [Accessible text requirements](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessible-text-requirements)
- [Progress controls](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/progress-controls)
- [WCAG: use of colour](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html)

## Resource use

- The UI is reactive. It never requests a continuous repaint.
- Input, a completed background scan, or the global hotkey wakes the UI explicitly with `Context::request_repaint`.
- Animations, animated scrolling, video, remote fonts, and decorative image assets are disabled or absent.
- Only icons visible in the current result are decoded and uploaded as small cached textures.
- Collapsed skill rows don't render their support grids, which cuts layout work and keeps six-skill warrants compact.
- OCR and capture stay off the UI thread, with only one scan allowed at a time.
- The UI keeps no scan-history clone. Only the current result and its elapsed scan time stay in memory.
- Settings reuse the existing window instead of creating another native viewport or renderer.

That follows egui's reactive-mode guidance: event-driven repainting lets the computer rest when nothing changes. See the official [`Context::request_repaint` documentation](https://docs.rs/egui/latest/egui/struct.Context.html).

## Template-pattern audit

I review the interface as a native game companion, not as a web landing page. It deliberately excludes gradients, glass surfaces, grain overlays, shadows, fade-in effects, cursor-following effects, icon-library decoration, hover fades, and animated scrolling. It also avoids generic feature-card grids and ornamental badges around ordinary metadata.

Two treatments are deliberate rather than template defaults:

- Bronze keylines and the serif display face echo Path of Exile's mercenary panel. The serif is limited to identity and hierarchy; normal controls and instructions use the proportional system face.
- Red, green, and gold verdict accents communicate scan state and resale action. Each colour is paired with explicit text, and ordinary support metadata stays unboxed.

Operational copy uses short, product-specific sentences. Navigation cues are plain labels with small vector marks, and text punctuation follows normal desktop UI conventions.

## Review checklist

Before release:

1. Test the empty state, a normal result, a jackpot, an unsupported family, and an OCR error.
2. Resize to 440 pixels wide and confirm all skill names, support names, tiers, and actions stay accessible.
3. Test 100%, 125%, and 150% Windows display scaling.
4. Navigate Scan, Clipboard, Settings, setting controls, Copy summary, and Search official trade by keyboard.
5. Confirm default text contrast is at least 4.5:1 and that every semantic colour has an accompanying text label.
6. Leave the app idle and confirm there is no continuous repaint or material CPU/GPU activity.
7. Run formatting, Clippy, the complete release test suite, and a release build.
8. Test Updates: not checked, up to date, available with auto-install off, downloading, ready, and failed offline.
