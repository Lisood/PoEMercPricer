# PoEMercPricer UI guide

How to add or change anything in the overlay without losing the look. Companion to `docs/ui-design.md` (the rules) and `src/app.rs` (the code).

Order of work for any UI change: decide the layout on paper, check it against the tokens and rules in this file, then write it in egui in `app.rs`. Update this file in the same commit whenever a token, size, breakpoint or rule moves.

## 1. Principles

1. Decision before detail. Verdict, score and action come first; identity, market and skills follow.
2. Text carries meaning; colour repeats it. Every coloured state has a label (`SKIP: BRICKED`, `thin market`, `BRICK`).
3. Flat and still. No gradients, shadows, glass, blur, animation, hover fades or icon-library decoration. Solid fills, 1 px keylines, 2 px radius.
4. Nothing hides silently. If content is clipped, a solid scrollbar and a text cue appear. If it fits, neither does.
5. Status is a sentence, not a dot. The footer states the current task or outcome.
6. Serif is display only. Georgia for names, verdicts, section titles, scores. The proportional UI face for everything operational. Faces: Georgia is read from `%WINDIR%\Fonts` at startup; Ubuntu-Light is the only embedded font and the fallback for both, so every non-ASCII character in the UI must exist in it (`tests/samples.rs` checks this).
7. One action per surface. No modals, toasts, countdowns or auto-restarts.
8. Works at 440 × 560. Everything wraps; nothing scrolls horizontally.

## 2. Themes

Three palettes plus Follow Windows (picks Dark or Light from the system app mode; falls back to Standard when unknown). Define each as one struct in `app.rs` and never mix tokens across themes.

| Token | Role | Standard | Dark | Light |
|---|---|---|---|---|
| `bg` | window, card inner | `#0A0908` | `#000000` | `#FFFFFF` |
| `surface` | bars, rows, fields | `#13110E` | `#0E0E0E` | `#F6F6F7` |
| `raised` | card ring, buttons | `#1D1914` | `#181818` | `#FFFFFF` |
| `border` | keylines, button strokes | `#4F402B` | `#333330` | `#D0D0D5` |
| `border_dark` | inner keylines, separators | `#2C241A` | `#222220` | `#ECECEF` |
| `text` | body | `#E0D6C3` | `#ECEBE8` | `#111114` |
| `muted` | secondary | `#B0A189` | `#A8A69F` | `#6B6B72` |
| `subtle` | tertiary, legal | `#9A8B72` | `#8C8A83` | `#707077` |
| `gold` | brand, display accents, links, focus | `#C7A05B` | `#D4A857` | `#A16207` |
| `gold_fill` | primary button fill | `#B39052` | `#C7A05B` | `#F0B429` |
| `gold_fill_text` | text on primary button | `#0A0908` | `#000000` | `#111114` |
| `orange` | action line, ambiguity, caveat words | `#D36A28` | `#E07A3A` | `#C2410C` |
| `red` | skip, brick, errors | `#E87670` | `#EF7A74` | `#DA2323` |
| `green` | positive breakdown points | `#82C889` | `#82C889` | `#15803D` |
| `take_green` | take-item verdicts | `#60E678` | `#60E678` | `#15803D` |
| `check` | price-check band | `#B2B24E` | `#B2B24E` | `#7A6A0A` |
| `brick_row` | bricking skill row fill | `#22100E` | `#241010` | `#FEF2F2` |
| `hovered` | widget hover and pressed fill (stroke turns `gold`) | `#352A1B` | `#242424` | `#EDEDEF` |

Two Light values are darker than the palette they came from because the contrast test rejected the originals on one surface each: `subtle` was `#86868E` (3.61:1 on `raised`) and `red` was `#DC2626` (4.47:1 on `surface`). Both were darkened at the same hue.

Verdict box fill is the accent at low alpha: take 30 % (Light 10 %), skip 12 % (Light 8 %), check 12 % (Light 9 %), everything else including unsupported 12 % of `muted` (Light 10 %). Error block: `red` at 10 % (Light 7 %).

Rules:
- Standard keeps the in-game bronze double keyline (`raised` ring, 3 px padding, `bg` inner with `border_dark`). In Light, `raised` equals `bg`, so the same markup renders as a single keyline. Do not add a second ring for Light or Dark.
- Every text token must reach 4.5:1 on `bg`, `surface` and `raised`. The contrast test in `app.rs` (`every_text_tone_meets_normal_text_contrast_on_every_surface`) must cover all three themes; add a new colour there before using it.
- Never introduce a fourth accent. New semantics reuse `orange` (caveat), `red` (stop), `take_green` (go), `gold` (identity).

## 3. Type

| Use | Face | Size |
|---|---|---|
| App title | Georgia | 18 |
| Verdict title | Georgia | 20, line-height 1.15, wraps |
| Score number | Georgia | 30, `of 100` at 11 muted |
| Mercenary name, class | Georgia | 16 class, 14 muted name/level |
| Section titles (`Mercenary skills`, Settings sections) | Georgia | 15 to 16 gold |
| Skill name | Georgia | 15, level 12 muted |
| Market range, tier `T3` | Georgia | 15 / 12 |
| Body, buttons, checkboxes | UI face, weight 300 | 14 |
| Status, action line, highlights | UI face | 13 |
| Metadata, detail lines, hints | UI face | 12, never below 11.5 |

No bold anywhere; hierarchy comes from face, size and colour. No italics. No all-caps except the verdict titles the code already emits.

## 4. Spacing and shape

- Panel margins 12 × 8 (bars), 12 (body). Card inner 14 × 10. Verdict box 10 × 8. Rows 8 × 5, min height 42.
- Item gap 8. Section gap 10. Keyline 1 px. Radius 2 (1 inside cards).
- Buttons 30 px tall, 10 px side padding, single line (`white-space: nowrap` / no wrap in egui). Small in-pane buttons 26 px.
- Checkbox and radio 14 px. Text field 26 px in Settings, full-width when it is the only control.
- Score bar 6 px, ticks at 50 / 65 / 80 / 90.
- Icons: skill 30 px in a 1 px `border` frame, support 22 px in the grid, 18 px in the collapsed strip. Always from `assets/icons`, never redrawn, never without a text label beside them.
- App identity: `assets/branding/app-icon-master.png` is the transparent source artwork. The generated PNG family lives under `assets/branding/icons`; the multi-resolution ICO is embedded in the Windows executable, while the 256 px PNG supplies the egui window, taskbar and Alt-Tab icon.

## 5. Layout of the main window

```
command bar   surface, border-bottom
  title · version (yields when space is short)
  Scan (primary, hotkey hint yields) · Clipboard · [Update to X] · Settings
body          bg, padding 12
  [error block]                 only after a failed scan
  score card                    fixed
    identity row  class · Lvl · name
    verdict box   title + action | score
                  score bar
                  highlights (max 2)
                  market row + detail
  Mercenary skills (n)           [Scroll for all skills]  cue only on overflow
  skills pane                   the only thing that scrolls; min 140
    skill rows (collapsed unless brick)
    Score breakdown disclosure  inside the pane
footer        surface, border-top
  status (max 2 lines, full text on hover) · Copy summary · Search official trade
```

Width rules (measure with `ui.available_width()`):
- ≥ 560: two support columns, icon strip on collapsed rows, hotkey hint on Scan, version tag.
- < 560: one support column.
- < 520 or an update button present: drop the version tag and the hotkey hint.
- < 500: drop the collapsed icon strip; the `n supports · 3×T3 · 1×T2` text stays.
- Verdict title, skill names and market detail wrap. Status clamps to two lines.

Scanning state: while a scan runs, Scan, Clipboard, Search official trade and Restart to update are disabled and the status line is gold. A scan is only counted as running for 30 s (`SCAN_WATCHDOG`): the OS OCR call and the window capture can block forever, so past that the buttons come back and the next Scan starts a fresh worker, saying `Previous scan was still running and was abandoned.` before the new status. The abandoned worker's result is discarded rather than replacing the newer scan. Nothing is restarted or killed, and nothing polls: the UI asks for a single repaint at the 30 s mark.

Height: the skills pane takes `available_height().max(140)`. When content exceeds it, show the 10 px solid scrollbar (handle ≥ 36 px) and the cue in the heading. Everything else is fixed.

## 6. Settings pane

Full-window pane replacing the body; command bar and footer stay. Header: `Back` button, `Settings` in Georgia 18 gold, `Open config folder` link. Footer status: `Changes apply immediately and save when you go back.` plus a gold `Done`. No Save button: edits apply live, closing saves, an invalid hotkey keeps the pane open with the error line under the field.

Section order, always: Appearance, Scanning, Window, Trade, Updates, Advanced (collapsed). The layout is the Ledger: one row per setting, label left, control right, section titles Georgia 15 gold over a `border_dark` rule. Helper text 11.5 `subtle`, one line, only when the label cannot carry it.

Adding a setting:
1. Add the field to `AppConfig` with a default; document it in `README.md` Settings row.
2. Pick the section. If none fits, it goes in Advanced, not a new section.
3. Control by type: boolean → checkbox; two to four choices → segmented buttons; free text → 26 px field; a list that can grow (leagues) → dropdown with a custom fallback field; action → 26 px button on the row's right.
4. Label is a short noun phrase. Consequence goes in the helper line (`Applies after restart.`).
5. Check the Ledger still fits at 780 tall; if not, move something to Advanced or accept the scrollbar (never clip).

Hotkey control: text field plus `Record`. While recording the field shows `Ctrl+Shift+…` in gold with a `Cancel` button; Escape cancels. Reject chords `parse_hotkey` cannot express and show `Invalid hotkey: …` in red under the field.

Updates section shows exactly two lines from `update_copy()` plus the one button for the state (`Check now`, `Update to X`, `Restart to update`, or `Check now` + `Open releases page` after a failure). Never add progress bars or spinners; the line of text is the progress.

## 7. States that must be designed for every change

Empty, normal result, jackpot, bricked skip, not scored, ambiguous support, scan error, abandoned scan (past the watchdog), update available, update ready, update failed, minimum window 440 × 560, all three themes. Screenshot each one you touch with `scripts/screenshot-window.ps1` and keep the list here in step with `docs/ui-design.md`'s review checklist.

## 8. Copy

- Short clauses, colons and periods. No dashes as ornament, no exclamation marks, no product marketing.
- Verbs for actions (`Scan`, `Copy summary`, `Search official trade`, `Restart to update`).
- Verdicts are explicit and name the cause (`SKIP: BRICKED`, `BRICKED BY: Kinetic Bolt`).
- Status lines state what happened and, if useful, the next step (`Scan failed: Clipboard does not contain Warrant text or an image`).
- Numbers keep their units and dates. Say `thin market`, `stale asks`, `sample data` in words. The market row and its detail line read:

  ```
  ≈ 4.5–9.0 div
  12 listed · level 83+ · typical ask 6.8 div · median listing age 4 d · Allflame snapshot 2026-09-02
  ```

- Plain labels only: `Skip`, `Price-check`, `Not scored`. No unexplained abbreviations beyond `KBoC`, `GMP`, `EDWA`, `T3`, which the game uses.

## 9. Do not

- Add cards, pills, badges or coloured outlines around ordinary metadata.
- Use emoji, icon fonts or drawn glyphs; chevrons, ticks, radios and the scroll cue are painted 1.25 to 1.5 px vector primitives.
- Introduce opacity on text (egui cannot); pick a token instead.
- Put anything in a `<img src>` or texture path that might not exist; icons load lazily and fail to text.
- Request continuous repaints. Every new UI state repaints once on change.
- Let anything overflow without a scrollbar and cue, or wrap a button label.

## 10. Checklist before merging a UI change

1. Screenshot the change in Standard, Dark and Light.
2. Resize to 440 × 560 and to 900 × 1200; nothing clips silently, nothing scrolls sideways, buttons stay single-line.
3. Every new colour is in the contrast test for all three themes.
4. Every coloured state has a text label. Every icon has a name beside it.
5. Keyboard: Tab reaches the new control; Escape still closes Settings; focus ring is the 1 px gold rect.
6. Idle: no repaint loop (watch GPU in Task Manager for ten seconds).
7. `cargo fmt`, `cargo clippy`, `cargo test`, release build.
8. Refresh `docs/ui-design.md` if a rule changed.
