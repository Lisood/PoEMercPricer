# UI design and performance contract

PoEMercPricer is a focused Windows utility, not a dashboard. The interface is designed around one decision: whether a scanned Mercenary Warrant is worth creating and price-checking.

## Design rules

- Put the decision, score, and action before supporting detail.
- Express the action as an explicit verb phrase. A conflicting build must read `SKIP: BRICKED` and name the exact bricking skill; colour alone is insufficient.
- Use the in-game mercenary panel as the visual reference: near-black inset surfaces, thin bronze double borders, warm ivory text, and restrained orange-gold emphasis. Colour supplements text labels; it never carries meaning alone.
- Keep the command bar and result actions stable while the result body scrolls.
- Keep the verdict fixed. Only the bounded skills pane scrolls; skill rows are collapsed summaries by default, while a bricking row is highlighted and expanded automatically.
- Draw status dots and disclosure chevrons as vector primitives rather than font glyphs, so missing-glyph squares and platform-dependent triangles cannot appear. Every skill row reserves fixed chevron and icon columns; non-expandable rows show `No supports` but retain the same name baseline.
- The bounded skills pane uses an always-visible, solid scrollbar with a minimum 36 px handle and an explicit, unboxed scroll cue. Support counts and tier distributions are primary row information and must remain readable at rest, rather than being hidden in low-contrast microcopy or decorative badges.
- Wrap every skill and support group. The minimum supported window width is 440 effective pixels, and no result should require horizontal scrolling.
- Show active-skill levels and support tiers next to their names. When identical artwork leaves several catalog-backed identities, show each viable tier-specific name as an inline selection button; applying a choice must immediately recompute the verdict and copied summary. Long candidates wrap instead of being clipped.
- Keep advanced diagnostics out of the normal scan path.
- Use existing 3.29 skill/support art beside readable names and tiers. Icons are loaded lazily, cached once, and never replace text.
- Use plain labels such as `Skip`, `Price-check`, and `Not scored`; avoid unexplained abbreviations, web-dashboard styling, gradients, glass effects, and decorative animation.
- Treat the game-derived serif as display type for names, verdicts, and section titles only. Operational guidance, actions, and helper text use the proportional UI face without decorative italics.
- Reserve coloured outlines and badges for semantic states such as `BRICK`; do not turn ordinary metadata, counts, or navigation hints into cards or pills. Bronze double keylines are retained only where they echo the in-game mercenary panel hierarchy.
- Use colons, periods, and short clauses in operational copy. Avoid ornamental dash-heavy phrasing and generic product language.
- Do not show a generic green dot or a synthetic `Ready` state. The status line states the current task or outcome directly, such as `Press Ctrl+M to scan a mercenary panel`, `Scan complete in 131 ms`, or `Scan failed: …`. Short scans use text rather than animated progress UI, and colour is never the only carrier of state.

The spacing, type hierarchy, fluid sizing, keyboard access, and contrast targets follow Microsoft guidance for Windows desktop applications:

- [Content layout and spacing](https://learn.microsoft.com/en-us/windows/apps/design/basics/content-basics)
- [Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography)
- [Keyboard interactions](https://learn.microsoft.com/en-us/windows/apps/develop/input/keyboard-interactions)
- [Accessible text requirements](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessible-text-requirements)
- [Progress controls](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/progress-controls)
- [WCAG: use of colour](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html)

## Resource budget

- The UI is reactive. It never requests a continuous repaint.
- Input, a completed background scan, or the global hotkey wakes the UI explicitly with `Context::request_repaint`.
- Animations, animated scrolling, video, remote fonts, and decorative image assets are disabled or absent.
- Only icons visible in the current result are decoded and uploaded as small cached textures.
- Collapsed skill rows do not render their support grids, reducing layout work and keeping six-skill warrants compact.
- OCR and capture stay off the UI thread, with only one scan allowed at a time.
- The UI does not retain a scan-history clone. Only the current result and its elapsed scan time remain in memory.
- Settings use the existing window rather than creating another native viewport or renderer.

This follows egui's reactive-mode guidance: event-driven repainting lets the computer rest when nothing changes. See the official [`Context::request_repaint` documentation](https://docs.rs/egui/latest/egui/struct.Context.html).

## Template-pattern audit

The interface is reviewed as a native game companion, not as a web landing page. It intentionally excludes gradients, glass surfaces, grain overlays, shadows, fade-in effects, cursor-following effects, icon-library decoration, hover fades, and animated scrolling. It also avoids generic feature-card grids and ornamental badges around ordinary metadata.

Two treatments are deliberate rather than template defaults:

- Bronze keylines and the serif display face echo Path of Exile's mercenary panel. The serif is limited to identity and hierarchy; normal controls and instructions use the proportional system face.
- Red, green, and gold verdict accents communicate scan state and resale action. Each colour is paired with explicit text, and ordinary support metadata remains unboxed.

Operational copy uses short, product-specific sentences. Navigation cues are plain labels with small vector marks, and text punctuation follows normal desktop UI conventions rather than ornamental dash-heavy marketing copy.

## Review checklist

Before release:

1. Test the empty state, a normal result, a jackpot, an unsupported family, and an OCR error.
2. Resize to 440 pixels wide and confirm all skill names, support names, tiers, and actions remain accessible.
3. Test 100%, 125%, and 150% Windows display scaling.
4. Navigate Scan, Clipboard, Settings, setting controls, Copy summary, and Search official trade by keyboard.
5. Confirm default text contrast is at least 4.5:1 and that every semantic colour has an accompanying text label.
6. Leave the app idle and confirm there is no continuous repaint or material CPU/GPU activity.
7. Run formatting, Clippy, the complete release test suite, and a release build.
