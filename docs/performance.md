# Lightweight OCR contract

PoEMercPricer must remain a small companion overlay. OCR is on-demand work, never a resident service.

## Enforced design limits

- **No child OCR process:** `src/winocr.rs` calls `Windows.Media.Ocr` through the Rust Windows binding. It does not launch PowerShell, write a temporary PNG, or ship a neural-model runtime.
- **Bounded OCR image:** screenshot OCR receives only the left three-quarters of the inspect panel, wide enough to preserve centered mercenary names at their native scale while excluding the surrounding world HUD. The bundled 850×1189 panel becomes 637×1189 (about 2.9 MiB RGBA), and a 2000×1123 fullscreen capture becomes a 630×1123 panel column. This is one temporary bitmap and one OCR call; there is no resident allocation or second pass.
- **Bounded template memory:** all 152 support identities collapse to 65 unique pixel arts. They are cached once at 32×32 plus 16×16, approximately 0.32 MiB of raw pixels after duplicates are collapsed.
- **No missing or speculative generic-art result:** silver/gold identities retain their complete PoEDB skill-and-tier compatibility metadata. Unique compatible candidates receive an exact name; physically indistinguishable survivors receive an explicit candidate-set label and tier instead of `gem` or a guessed name.
- **Compact eligibility data:** the 1,660 compatibility records are stored once as `Skill|tiers` strings in the 199 KiB catalog, replacing the former skill-only list rather than adding a second index. The visible tier is checked inside the existing catalog pass, with no extra capture, OCR, template, or resident service.
- **Zero-extra-pass tooltip proof:** when identical art still leaves several candidates, an exact support title already present in the bounded OCR result resolves one unique ambiguous cell. Descriptions, longer names, multiple titles, and duplicate eligible cells are rejected rather than guessed. No second screenshot or OCR call is made.
- **Event-only manual resolution:** unresolved identical-art candidates render as ordinary inline buttons. A click updates the in-memory canonical support and recomputes the score once; it adds no OCR, polling, animation, or background work.
- **DPI-resilient icon normalization:** every detected cell is compared at 85%, 100%, and 115% crop scale. Exact matches use a conservative confidence/margin gate; degraded captures retain the best catalog-backed candidate set rather than emitting `Unresolved support`.
- **No idle OCR:** capture, OCR, and template matching run only after a Scan/hotkey action and execute off the UI thread.
- **No idle repaint loop:** the global hotkey handler and scan worker wake the UI only
  for real events. There is no periodic 100 ms repaint/poll timer, which keeps idle
  CPU use low and avoids repeatedly presenting the transparent overlay window.
- **Clipboard images stay in-process:** copied bitmap pixels are converted directly
  to RGBA. Explorer-copied PNG/JPEG/WebP files are decoded with the already-bundled
  lightweight image codecs; neither path starts a helper process or resident service.
- **Catalog matching is pre-normalized:** OCR parsing normalizes the 3.29 skill
  catalog once per scan and reuses compiled regular expressions. Fuzzy matching
  never recompiles patterns or renormalizes all 267 skills inside its inner loop.
- **Stable interaction frames:** button labels keep a fixed layout during scans and
  hover animations are disabled, avoiding animation-only redraw bursts on Windows.

The vision and `gem_acceptance` tests verify all 16 support cells on the real Alara panel, require exact catalog identities wherever the pixels permit them, and permit an ambiguity only when every displayed candidate is catalog-backed and byte-identical. `scan::tests::ocr_region_is_bounded_to_the_skill_text_column` enforces the OCR crop sizes. The fullscreen Danalla regression fixture is intentionally adversarial: an equipment tooltip overlaps its panel and the world HUD surrounds it. It must still read Danalla at level 83 with exactly the six visible skill rows and no off-panel support icons. A separate unobstructed cropped-Danalla test protects the normal screenshot/clipboard path without pretending the two independently captured images are pixel-equivalent.

Wrapped active-skill recognition is generated-test backed rather than sampled:
every multiword catalog name is reconstructed at every word boundary in all six
skill slots with left-, centre-, and right-aligned continuation text (6,084
cases), while 1,690 adjacent-row cases prove fragments cannot be joined across
different skills. Reversed and interleaved Windows OCR order is covered without
adding another OCR, capture, or image-matching pass.

## Benchmark and acceptance check

Always benchmark an optimized build because image correlation and Windows bindings are intentionally optimized for release:

```powershell
cargo test --release --test scan_screenshots scan_manyshot_alara_reads_skills_and_gem_tiers -- --nocapture --test-threads=1
cargo test --release --test scan_screenshots fullscreen_danalla -- --nocapture --test-threads=1
cargo test --release --test scan_screenshots warm_fullscreen_scan_stays_within_interactive_latency_budget -- --nocapture --test-threads=1
```

The test must identify the Manyshot family, level 83, all six visible skills at skill level 26, all 16 support cells, and the exact Ice Shot sequence `AoE II / Cold Penetration II / Fork III / Hypothermia III`.

End-to-end time includes the OS OCR engine and varies by Windows language pack, CPU, and first-run cache. The first scan is therefore reported separately from repeat scans. The release acceptance ceiling for a warm fullscreen `scan_rgba` call is **1 second**; debug builds use an 8-second ceiling because their image-correlation loops are intentionally unoptimized. The user-facing target is **under 1 second from Scan click to verdict** on the reference machine. Capture and UI dispatch should consume only the remaining portion of that target and must be profiled separately when a click is slower than the pure-image gate.

Local release verification on 2026-09-01 scanned `manyshot_alara.png` five times in
**140–158 ms end to end** after parser optimization. Native Windows OCR took about
40–50 ms; the remaining time covered catalog parsing and all support-icon/tier
matching. The command returned the exact six active skills at level 26 and all 16
support tiers. These figures are measurements, not cross-machine pass/fail thresholds.

The live 2560×1440 Scan path was also measured twice in one persistent release
process with a real Path of Exile Warrant panel open. Pass one took **216.4 ms**
(61.4 ms capture + 155.0 ms scan); pass two took **154.6 ms** (53.8 ms capture +
100.8 ms scan), with all six skills detected on both passes. Direct monitor capture
was not adopted: at about 61 ms it was no faster than window capture and could include
PoEMercPricer itself when the companion window has focus. Exact game-title matches and
non-minimized windows are preferred; monitor capture is only a failure fallback.

## Official trade searches

Trade-query construction is offline and allocation-bounded. A 49 KB snapshot of the
official 3.29 mercenary skill/support stat IDs is bundled with the executable and parsed
once on first use. The app never polls or scrapes the trade service, fetches results, or
retries in the background. A user click creates one encoded official search URL and one
normal browser navigation, keeping both resource use and Path of Exile trade traffic low.
PoE's anonymous complexity limit permits one exact mercenary stat group. Searches use
the exact warrant type and level, then the most-supported skill and up to five supports
bound to that skill. The UI names the selected skill and reports that one of the scanned
skills was used. A two-second button guard prevents accidental duplicate browser tabs.

The app itself never submits the encoded query to the API. For release verification,
[`scripts/verify-trade-search.ps1`](../scripts/verify-trade-search.ps1) prints and validates
the payload locally by default. Its explicit `-Live` mode performs exactly one search
POST, has no retry loop, and does not fetch listing details. On 2026-09-02 the Grynnelle
fixture's type + level + six-filter Storm Call query was accepted by the official
Allflame endpoint with HTTP 200 and one matching result (`RJGD3PRZh7`).
