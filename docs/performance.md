# Performance and footprint

PoEMercPricer should stay a small companion overlay. OCR is on-demand work and never a resident service. This file covers the scan path, the binary and memory budgets, and the trade-search path.

## Installer addition, 2026-09-05 (v0.2.0 preparation)

The installer uses native Inno Setup controls, the existing gold icon and
LZMA2/normal compression. It adds no webview, service, startup task or polling
to the app. GUI startup creates one named mutex; registration metadata is
accessed only after an update. Notice downloads add about 224 KB for the
current build, with a 5 MB declared-size ceiling.

| Measurement | Observed | Gate |
|---|---:|---:|
| Release app | 8,805,376 bytes | 11,000,000 bytes |
| Unsigned installer including app and uninstaller | 5,747,108 bytes | 8,000,000 bytes |
| Gzip app payload | 4,015,542 bytes | Must be smaller than the exe |
| Silent install / repair | About 0.9 seconds each | 60-second smoke-test timeout |
| Warm fullscreen scan | 48.97 ms | 1 second |
| Fullscreen scan peak working set | 40.3 MiB | 96 MiB |
| Idle GUI working set | 126.8 MiB | 160 MiB |

The app is 25,600 bytes larger than the published v0.1.0 executable. Timing
depends on disk and antivirus activity. `scripts/test-installer.ps1` records
each lifecycle timing and launcher peak working set (about 16 MiB here).
That launcher measurement excludes the separate Inno worker and is not a
total installer memory measurement. The user explicitly authorized GUI tests,
so the idle-GUI memory test was run too. The scan-memory and latency tests
run without a GUI. The updater now streams bounded HTTP bodies in 64 KiB
chunks and holds a file lock only during installation; it adds no idle polling.

The size gates run during local packaging and before release publication,
with the final package. Full installation and release details are in
[installation.md](installation.md).

## Design limits

- No child OCR process. `src/winocr.rs` calls `Windows.Media.Ocr` through the Rust Windows binding. It doesn't launch PowerShell, write a temporary PNG, or ship a neural-model runtime.
- The OCR image is bounded: screenshot OCR receives only the left three-quarters of the inspect panel, wide enough to keep centered mercenary names at their native scale while excluding the surrounding world HUD. The bundled 850x1189 panel becomes 637x1189 (about 2.9 MiB RGBA), and a 2000x1123 fullscreen capture becomes a 630x1123 panel column. That is one temporary bitmap and one OCR call, with no resident allocation and no second pass.
- Choosing between those two crops costs no extra work: `vision::is_fullscreen_capture` classifies an image as a full-screen game capture from its dimensions alone, when it is at least 1024 px wide and at least 1.2 times as wide as it is tall (`width * 5 >= height * 6`). That admits 4:3 (1024x768, 1152x864) and 5:4 (1280x1024) monitors; a tighter aspect floor sent them down the cropped-panel path, which OCR'd the whole screen including the world HUD. Cropped panel screenshots and clipboard bitmaps are portrait or tiny, so they stay on the cropped path.
- Template memory is bounded too. All 152 support identities collapse to 65 unique pixel arts, cached once at 32x32 plus 16x16, roughly 0.32 MiB of raw pixels after duplicates are collapsed.
- Generic art never produces a missing or speculative result. Silver/gold identities keep their complete PoEDB skill-and-tier compatibility metadata. Unique compatible candidates get an exact name; physically indistinguishable survivors get an explicit candidate-set label and tier instead of `gem` or a guessed name.
- Eligibility data is compact. The 1,653 support compatibility records are stored once as `Skill|tiers` strings inside the 206 KiB catalog (70 KiB after the build-time minify), not as a second index beside a skill-only list. The visible tier is checked inside the existing catalog pass, with no extra capture, OCR, template, or resident service.
- Tooltip proof costs no extra pass. When identical art still leaves several candidates, an exact support title already present in the bounded OCR result resolves one unique ambiguous cell. Descriptions, longer names, multiple titles, and duplicate eligible cells are rejected rather than guessed. No second screenshot or OCR call happens.
- Manual resolution is event-only. Unresolved identical-art candidates render as ordinary inline buttons. A click updates the in-memory canonical support and recomputes the score once. It adds no OCR, polling, animation, or background work.
- Icon normalization survives DPI changes: every detected cell is compared at 85%, 100%, and 115% crop scale. Exact matches use a conservative confidence/margin gate, and degraded captures keep the best catalog-backed candidate set rather than emitting `Unresolved support`.
- Cells and templates are resampled to 32x32 with the `image` crate's area-weighted Triangle filter, not nearest neighbour. Nearest neighbour discarded a third of the client's 48 px pixels and held correct matches at 0.55-0.70, which reported the Swift Affliction and Infused Channelling cells as unidentified "ring-and-hook" gems. No new dependency: the filter ships with the already-bundled codec crate.
- Shared art is resolved by row uniqueness before anything is displayed. A support cannot be socketed twice in one skill row, so an identical-art candidate already held exactly elsewhere in the row is eliminated, and N cells sharing one N-identity set are all present (their order is unknown; scoring and trade searches are order-free, and the `raw` field records the resolution). Below-floor fallbacks carry no candidate set and are never resolved this way.
- The six skill rows run their icon vision on scoped `std::thread` workers and join before the mercenary is assembled. There is no thread pool, no `rayon`, and no work outside a Scan.
- No idle OCR. Capture, OCR, and template matching run only after a Scan or hotkey action, off the UI thread.
- No idle repaint loop. The global hotkey handler and scan worker wake the UI only
  for real events. There is no periodic 100 ms repaint/poll timer, which keeps idle
  CPU use low and avoids repeatedly presenting the transparent overlay window.
- Clipboard images stay in-process. Copied bitmap pixels are converted directly
  to RGBA. Explorer-copied PNG/JPEG/WebP files are decoded with the already-bundled
  image codecs. Neither path starts a helper process or resident service.
- Catalog matching is pre-normalized. OCR parsing normalizes the 3.29 skill
  catalog once per scan and reuses compiled regular expressions. Fuzzy matching
  never recompiles patterns or renormalizes all 267 skills inside its inner loop.
- Interaction frames stay stable. Button labels keep a fixed layout during scans and
  hover animations are off, which avoids animation-only redraw bursts on Windows.

The vision and `gem_acceptance` tests check all 16 support cells on the real Alara panel, require exact catalog identities wherever the pixels permit them, and allow an ambiguity only when every displayed candidate is catalog-backed and byte-identical. `scan::tests::ocr_region_is_bounded_to_the_skill_text_column` enforces the OCR crop sizes. The fullscreen Danalla regression fixture is deliberately nasty: an equipment tooltip overlaps its panel and the world HUD surrounds it. It must still read Danalla at level 83 with exactly the six visible skill rows and no off-panel support icons. A separate unobstructed cropped-Danalla test protects the normal screenshot/clipboard path without pretending the two independently captured images are pixel-equivalent.

Wrapped active-skill recognition is generated-test backed rather than sampled.
Every multiword catalog name is reconstructed at every word boundary in all six
skill slots with left-, centre-, and right-aligned continuation text (6,084
cases), while 1,690 adjacent-row cases prove fragments cannot be joined across
different skills. Reversed and interleaved Windows OCR order is covered without
another OCR, capture, or image-matching pass.

## Benchmark and acceptance check

Always benchmark an optimized build. Image correlation and the Windows bindings are tuned for release:

```powershell
cargo test --release --test scan_screenshots scan_manyshot_alara_reads_skills_and_gem_tiers -- --nocapture --test-threads=1
cargo test --release --test scan_screenshots fullscreen_danalla -- --nocapture --test-threads=1
cargo test --release --test scan_screenshots warm_fullscreen_scan_stays_within_interactive_latency_budget -- --nocapture --test-threads=1
```

The test must identify the Manyshot family, level 83, all six visible skills at skill level 26, all 16 support cells, and the exact Ice Shot sequence `AoE II / Cold Penetration II / Fork III / Hypothermia III`.

End-to-end time includes the OS OCR engine and varies by Windows language pack, CPU, and first-run cache, so the first scan is reported separately from repeat scans. The release acceptance ceiling for a warm fullscreen `scan_rgba` call is 1 second. Debug builds use an 8-second ceiling because their image-correlation loops are unoptimized on purpose. The user-facing target is under 1 second from Scan click to verdict on the reference machine. Capture and UI dispatch should only consume the remaining portion of that target, and need profiling separately when a click is slower than the pure-image gate.

### Detection quality harness (2026-09-03)

`POEMERC_PROFILE_OCR=1` prints one `result` line per support cell. Running the
`scan_screenshots` and `gem_acceptance` suites single-threaded with it set gives
208 cells across the eleven fixtures plus the 125%/150% DPI variants. Warm timings
are the ignored latency tests, median of seven runs, release build.

| Change | Exact cells / 208 | Ambiguous | Mean exact score | Cells with margin < 0.05 | Warm fullscreen (Danalla) | Warm Alara |
|---|---|---|---|---|---|---|
| Before | 181 (87.0%) | 27 | 0.705 | 23 | 65.6 ms | 44.8 ms |
| Triangle resampling | 193 (92.8%) | 15 | 0.844 | 12 | 64.0 ms | 49.1 ms |
| + row uniqueness | 196 (94.2%) | 12 | 0.846 | 12 | | |
| + threaded rows | 196 (94.2%) | 12 | 0.846 | 12 | 56.6 ms | 38.7 ms |

Net: ambiguity down 56% (27 to 12), the weakest-margin cell count halved, and the
warm scan 14% faster. Every newly exact cell was checked: Swift Affliction and
Infused Channelling by eye against the fixture crops, Shock Chance and the Throwing
Speed / Trigger Radius pair by the row rule. The 12 remaining ambiguities are
byte-identical catalog arts with no row evidence (Minion Damage / Minion Life,
Cooldown Recovery / DoT Multiplier, three Throwing Speed / Trigger Radius cells in
one Spectral Helix row) and two cells of the synthetic 150% DPI Alara variant.

Rejected after measurement, each on the same 208 cells: choosing the fine-pass
crop scale from the coarse pass (175 exact), a brightness-gain colour term (181,
slower), rg-chromaticity colour (183, slower), gradient-magnitude NCC at 0.30 weight
(broke fixtures), Catmull-Rom resampling (191), letting the observed gold pitch
grow the cell beyond the OCR estimate (190, but broke Orvan), and measuring the gold
frame height directly (mis-tiered Alara). Perceptual-hash prefilters, HOG features,
and a Hungarian assignment crate were considered and not needed: the coarse NCC
shortlist already costs under a millisecond per cell, and row uniqueness only ever
has to eliminate or consume a set, which is a few lines without a solver.

Local release verification on 2026-09-01 scanned `manyshot_alara.png` five times in
140-158 ms end to end after parser optimization. Native Windows OCR took about
40-50 ms; the rest covered catalog parsing and all support-icon/tier
matching. The command returned the exact six active skills at level 26 and all 16
support tiers. Those are measurements from one machine, not cross-machine pass/fail thresholds.

I also measured the live 2560x1440 Scan path twice in one persistent release
process with a real Path of Exile Warrant panel open. Pass one took 216.4 ms
(61.4 ms capture + 155.0 ms scan); pass two took 154.6 ms (53.8 ms capture +
100.8 ms scan), with all six skills detected on both passes. I didn't adopt direct monitor
capture: at about 61 ms it was no faster than window capture and could include
PoEMercPricer itself when the companion window has focus. Exact game-title matches and
non-minimized windows are preferred, and monitor capture is only a failure fallback.

## Binary size and memory

The app ships as one exe and every update downloads the whole file again, so
exe bytes are download bytes. Measured on 2026-09-04 (NVIDIA GPU, Windows 11,
release profile, `--no-updates`, idle window). "Private WS" is the column Task
Manager labels Memory.

| Build | Exe | Private WS | Working set | Commit | Warm scan |
|---|---|---|---|---|---|
| an earlier local build as committed (opt-level 3, default wgpu setup) | 12.66 MB | 220 MB | 273 MB | 307 MB | 48-54 ms |
| + `opt-level = "s"` | 9.96 MB | same | same | same | 49-51 ms |
| + JSON minified in build.rs | 9.77 MB | same | same | same | same |
| + Vulkan-only instance and `MemoryHints::MemoryUsage` | 9.77 MB | 81 MB | 131 MB | 155 MB | 49-51 ms |
| + regex without the five unused Unicode tables | 9.54 MB | same | same | same | 50-55 ms |
| + `MemoryHints::Manual { 2..64 MiB }` | 9.54 MB | 75 MB | 125 MB | 142 MB | same |
| + only Ubuntu-Light embedded (egui's other three fonts dropped) | 8.49 MB | same | same | same | same |
| + `opt-level = "z"` on renderer-setup crates | 8.22 MB | 75 MB | 125 MB | 142 MB | 50 ms |

That sweep ran before the icon art moved into the exe. The published v0.1.0
asset is 8,779,776 bytes, of which the 419 embedded webp icons are 878 KB and
the gzip decoder for the compressed update asset is 31 KB. A local release
build of the same commit lands within a few KB of that; only the CI build is
published.

For calibration, a wgpu "draw a triangle" window on Windows/Vulkan is reported
at about 145 MB working set and a trivial egui app at about 30 MB on a
software path, so 75 MB private on an NVIDIA Vulkan device is close to the
floor for this renderer; the rest is the driver.

What each change does:

- `opt-level = "s"` (Cargo.toml): 21% smaller code with no measurable change
  to the scan loop; the full test suite and the detection harness are
  unchanged.
- build.rs strips whitespace outside string literals from the three embedded
  JSON files (354 KB to 164 KB). Keys, order and numbers are untouched;
  `tests/samples.rs` asserts the embedded copy parses to the same value as
  the source file. A side effect is that the exe embeds the same bytes
  whatever the checkout's line endings.
- `wgpu_options()` in src/main.rs. eframe's default creates a Vulkan instance
  and an OpenGL (WGL) context it never uses, and wgpu-hal's Vulkan allocator
  under `MemoryHints::Performance` maps a 128 MB host-visible chunk on the
  first upload. The app already rendered on Vulkan (DX12 is not compiled in,
  since eframe disables wgpu's default features), so restricting the instance
  to Vulkan and asking for `MemoryUsage` changes nothing on screen. If no
  Vulkan adapter exists the instance falls back to Vulkan + GL exactly as
  before, and `WGPU_BACKEND` still overrides for debugging.

- regex keeps `std`, `perf`, `unicode-case` and `unicode-perl`. Every
  pattern in src uses only `\b \s \d (?i) (?m)`, named groups and ASCII
  classes; the dropped `unicode-age/bool/gencat/script/segment` tables only
  serve `\p{..}` classes, which nothing uses. A pattern that needed them would
  fail at its `LazyLock` in the existing tests. `perf` stays: dropping it falls
  back to the PikeVM, which is what made regex-lite slow.
- `MemoryHints::Manual` sets wgpu-hal's first Vulkan allocator chunks to 2 MiB
  instead of `MemoryUsage`'s 8 MiB. `desired_maximum_frame_latency: Some(1)`
  was measured alongside and gained nothing, so it is not set.
- Fonts: egui bundles Ubuntu-Light, Hack, NotoEmoji and emoji-icon-font
  (1.4 MB). The UI never uses `FontFamily::Monospace`, and every non-ASCII
  character it draws exists in Ubuntu-Light and Georgia, so only Ubuntu-Light
  is embedded (`assets/fonts`, UFL licence alongside). Latin text renders
  from the same face as before. `tests/samples.rs` walks `src/` and asserts
  Ubuntu-Light has a glyph for every non-ASCII character in a string literal,
  with a negative control, so new text that needs another face fails the
  suite instead of drawing a `?`.
- `[profile.release.package.*] opt-level = "z"` on naga, wgpu, wgpu-core,
  wgpu-hal, winit, egui-winit, accesskit_windows, accesskit_consumer and
  windows: shader translation,
  device setup and event plumbing, none on the scan path and negligible per
  frame. The app crate, image codecs, egui and epaint stay at `s`.

Where the remaining bytes are (cargo-bloat, `.text` 7.8 MiB of 12.1 MiB
before the changes): std 1.7 MiB, naga 1.0 MiB, wgpu core and hal 0.85 MiB,
the regex family 0.64 MiB, image codecs 0.8 MiB. `.rdata` holds 1.4 MiB of
egui's bundled fallback fonts. Vulkan and GL backends are compiled in because
wgpu 24 forces them on Windows; dropping them needs eframe 0.36 (wgpu 30),
roughly 1-1.5 MB, and is a migration rather than a quick win.

Rejected after measurement, same gates:

- `opt-level = "z"`: 9.27 MB but the warm scan slowed to 56-58 ms (loop
  vectorisation is off).
- `regex-lite` in place of `regex`: 1.0 MB smaller and every test green, but
  the OCR text parse took the warm scan to 60-99 ms.
- `panic = "abort"`: src/app.rs catches panics from the scan and update
  threads with `catch_unwind`.
- Dropping the webp or jpeg codec: the 419 embedded icons are webp and
  `--scan FILE` accepts both.
- Dropping egui's bundled fonts: they are the glyph fallback beside Georgia.
- DX12 (needs the wgpu `dx12` feature, +0.4 MB): working set 158 MB but
  commit 450 MB on the NVIDIA driver, and a different presentation path from
  the one the flicker fix was verified on.
- UPX: antivirus false positives, and it turns file-backed shareable code
  into private working set.
- Disabling the `accesskit` feature: 0.2-0.4 MB, but Narrator and UI
  Automation could no longer read the window.
- `desired_maximum_frame_latency: Some(1)`: no measurable memory change.
- Disabling Vulkan implicit layers (`VK_LOADER_LAYERS_DISABLE`): 0 MB here
  (Steam and Epic layers installed), and it would break OBS game capture and
  the NVIDIA Optimus layer on hybrid laptops.
- `K32EmptyWorkingSet` on hide: lowers Task Manager's number while hidden
  but not commit, and the next show pays the page faults back. Cosmetic.
- Dropping the wgpu surface while hidden: eframe 0.31 has no hook for it on
  the root viewport; it needs a vendored `wgpu_integration.rs`.
- Deflating the embedded font at startup: 190 KB for a heap copy plus a
  decode on every launch.
- `--remap-path-prefix`: 35 KB and strips local paths, but only CI builds
  ship and they carry runner paths already.
- Loading the window icon from the exe's own icon resource instead of a
  second embedded PNG: 75 KB for an unsafe resource lookup.

Download size: GitHub serves release assets uncompressed (verified by
headers), so `release.yml` uploads `poemercpricer-windows-x64.exe.gz` beside
the exe (4,007,868 bytes for v0.1.0, 46% of the exe, .NET GZipStream at
SmallestSize). The updater prefers it, inflates with a `size + 1` bound and
verifies the result against the exe asset's own size and digest, falling back
to the exe asset on any failure; the exe asset stays the contract for
installed copies. gzip was
chosen over zstd because `flate2` is already linked for PNG. Offline tests
cover parsing, bounding, round-trip and every rejection; the live path is
exercised by the manual release check in docs/updater.md section 7. A
`bidiff` delta from the previous tag would cut small releases further but
needs the previous exe in CI and a base-hash check on the client; deferred
until there are two public releases to diff.

Gates, run before and after any profile, dependency or renderer change:

```powershell
cargo build --release
cargo test --release --test scan_screenshots warm_fullscreen_scan_stays_within_interactive_latency_budget -- --ignored --nocapture --test-threads=1
cargo test --release --test resource_budgets -- --ignored --nocapture --test-threads=1
```

`resource_budgets` asserts the warm fullscreen scan peaks under 96 MB working
set (measured 40 MB) and the idle GUI stays under 160 MB working set
(measured 125 MB). Both are `#[ignore]` because they measure this machine's
GPU driver, like the latency test. `scripts/release.ps1` and `release.yml`
fail when the release exe exceeds 11,000,000 bytes (v0.1.0 measured
8,779,776). `tests/resource_budgets.rs` measures memory only, not exe size. The normal
`cargo test` also runs the embedded-JSON equivalence test and the font
glyph-coverage test, which guard the two build-time trims.

Not a size issue but found on the way: a release uploads the exe, its gzip
sibling and the notices file and nothing else, so the 419 skill and support
icons under `assets/icons` (878 KB of webp) are embedded by `build.rs` into
`OUT_DIR/icons.rs` and reached through `src/icons.rs`. Nothing reads the
directory at runtime. While the icons lived on disk only, a copy that was
just the downloaded exe had no templates and reported every support as
unresolved.

## Official trade searches

Trade-query construction is offline and allocation-bounded. A snapshot of the official
3.29 mercenary skill and support stat IDs ships with the executable (49 KB in
`assets/trade-stats-3.29.json`, 34 KB after the build-time minify) and is parsed once on
first use. The app never polls or scrapes the trade service, fetches results, or
retries in the background. A user click creates one encoded official search URL and one
normal browser navigation, which keeps both resource use and Path of Exile trade traffic low.
PoE's anonymous complexity limit permits one exact mercenary stat group of six filters;
a second group is rejected as too complex. Searches use the warrant type and a minimum
level, then the most-supported skill and up to five supports bound to that skill. The UI
names the selected skill and reports that one of the scanned skills was used. The Trade
setting "Search every skill" widens this to five groups (the money skill with its
supports, then four skill-only groups), which the trade site accepts only for a logged-in
session; it is off by default. A two-second button guard stops accidental duplicate
browser tabs.

The app itself never submits the encoded query to the API. For release verification,
[`scripts/verify-trade-search.ps1`](../scripts/verify-trade-search.ps1) prints and validates
the payload locally by default. Its explicit `-Live` mode performs exactly one search
POST, has no retry loop, and does not fetch listing details. On 2026-09-02 the Grynnelle
fixture's type + level + six-filter Storm Call query was accepted by the official
Allflame endpoint with HTTP 200 and one matching result (`RJGD3PRZh7`).
