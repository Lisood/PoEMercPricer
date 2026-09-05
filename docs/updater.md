# Auto-updater design

Version 0.2.0 adds installer integration, versioned notices and bounded,
serialized update downloads. See [installation.md](installation.md).
Written 2026-09-03 before the first release, implemented the
same day, released as v0.1.0 on 2026-09-04. Sections 1 to 7 describe the updater as it
runs today. Sections 8, 9 and 12 are the original plan and its verification
log, kept as history. Section 13 records what the first published release
carried.

PoEMercPricer ships as one `poemercpricer.exe` (unsigned; see section 4). Every Path of Exile patch changes the catalog, so users who fall behind get wrong verdicts. The updater keeps them current with the least machinery that actually works: GitHub Releases as the free host, one HTTPS request at startup and every 6 hours after, a verified download, an in-place swap of the running exe, and a visible "Restart to update" button. Nothing installs on exit and nothing restarts on its own.

## 1. What Awakened PoE Trade does

Read from `main/src/AppUpdater.ts`, `renderer/src/web/settings/about.vue` and `ipc/types.ts` on master (2026-09-03):

- `electron-updater` against GitHub Releases (`latest.yml` plus the setup exe). Checks once at startup and every 16 hours.
- The installer build downloads automatically. The portable build and macOS never download; they show "download manually".
- `--no-updates` turns auto-download off but still checks.
- States: `initial`, `checking-for-update`, `update-available` (with `noDownloadReason`), `update-downloaded`, `update-not-available`, `error`. The UI lives only in Settings > About: two lines of text and one button.
- After download it says "will be installed on exit". Users found that confusing and it fails when Windows shuts the app down (issue #846, closed as not planned). `--no-updates` confusion is discussion #1726.

What I keep: startup check, GitHub Releases, a `--no-updates` flag, an About-style panel with two lines and one button, no modal dialogs.

What I change: install immediately after verification instead of on exit, put the restart prompt in the command bar where the user looks, never show a red error for a failed background check, re-check every 6 hours instead of 16 (one GET is cheap and PoE patches land mid-session), and say what happens next in plain words: the new version runs on the next start, or now if you press Restart.

## 2. Hosting: GitHub Releases

Free, no server, no account beyond the repo. Verified on 2026-09-03 with the WinRT HTTP client from a plain Rust binary:

| Step | Endpoint | Observed |
|---|---|---|
| Latest release | `GET https://api.github.com/repos/Lisood/PoEMercPricer/releases/latest` | 412 ms for `cli/cli`; excludes drafts and prereleases by itself |
| Asset download | `assets[].browser_download_url` (302 to `objects.githubusercontent.com`, followed automatically) | 15.3 MB in 498 ms |
| Integrity | `assets[].digest` is `"sha256:<hex>"`, computed by GitHub for every uploaded asset | Local SHA-256 matched |

Constraints to plan around:

- The repo has to stay public. Unauthenticated calls to a private repo return 404 (confirmed against this repo while it was still private), so every installed copy loses updates the moment `Lisood/PoEMercPricer` goes private. There is no free token-less alternative for a private repo, and shipping a token inside the exe is not an option.
- Unauthenticated API limit is 60 requests per hour per IP. One request per launch is far under that; a 403 or 429 is reported as "GitHub rate limit, try again in an hour".
- GitHub rejects requests without a `User-Agent`. Send `PoEMercPricer/<version>` and `Accept: application/vnd.github+json`.
- The asset must have a fixed name: `poemercpricer-windows-x64.exe`. The updater looks for that exact name and ignores everything else, so extra assets (a zip, a PDB) never break it. One optional sibling is recognised: `poemercpricer-windows-x64.exe.gz`, a gzip of the same bytes (46% of the size: v0.1.0 is 8,779,776 bytes as an exe and 4,007,868 gzipped). The updater downloads it when present, inflates it with a bound of `size + 1`, and verifies the result against the exe asset's own `size` and `digest`, so it adds no trust; any failure falls back to the exe asset.

## 3. Backend

### 3.1 Dependencies

Everything except the exe swap already ships in the binary:

| Need | Choice | Cost |
|---|---|---|
| HTTPS | `windows::Web::Http::HttpClient` (WinRT, already a dependency; OS trust store, system proxy, redirects) | 4 more `windows` features: `Web_Http`, `Web_Http_Headers`, `Win32_Security_Cryptography`, `Win32_System_Com` (`Foundation`, `Storage_Streams` and `Win32_System_WinRT` were already on for OCR) |
| SHA-256 | `windows::Win32::Security::Cryptography::BCryptHash` against the `BCRYPT_SHA256_ALG_HANDLE` pseudo-handle (plain Win32 CNG, no COM/WinRT apartment) | none; 12 ms for 15 MB |
| Version compare | `semver` 1.0.28, already in `Cargo.lock` transitively | direct dependency, no new crate |
| Replace the running exe | `self-replace` 1.5.0 (Apache-2.0, by mitsuhiko, used by rustup/uv) | +1 crate; its deps `tempfile`, `fastrand`, `windows-sys 0.52` are already locked |
| Inflate the gzip asset | `flate2::read::GzDecoder` | none; `flate2` is already linked for PNG |

Rejected: `self_update` (pulls reqwest + tokio, interactive defaults, blocks on stdin), `ureq` (fine, but WinRT already does the job with zero new crates), `MOVEFILE_DELAY_UNTIL_REBOOT` (needs a reboot), batch-file swaps (racy).

### 3.2 Module `src/update.rs`

Pure functions and one blocking pipeline; the UI thread never calls it directly.

```rust
pub const REPO: &str = "Lisood/PoEMercPricer";
pub const ASSET: &str = "poemercpricer-windows-x64.exe";
pub const COMPRESSED_ASSET: &str = "poemercpricer-windows-x64.exe.gz";
pub const RELEASES_URL: &str = "https://github.com/Lisood/PoEMercPricer/releases";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

const JSON_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Debug)]
pub struct Release { pub version: semver::Version, pub page: String, pub download: String, pub compressed: Option<String>, pub size: u64, pub sha256: String, pub notices: Option<NoticeAsset> }

#[derive(Clone, Debug)]
pub enum UpdateState {
    Idle,                                   // disabled, or not checked yet
    Checking,
    UpToDate { checked: Instant },
    Available(Release),                     // notify-only mode, or before install
    Downloading(Release),
    Ready { version: String },              // new exe is on disk; restart pending
    Failed { message: String },             // manual fallback: open RELEASES_URL
}

/// One GET. Ok(None) = already current. Tags must parse as `vX.Y.Z`.
pub fn check() -> anyhow::Result<Option<Release>>;
/// Download next to the exe, verify size and sha256, keep the old exe as
/// `poemercpricer-previous.exe`, then self_replace. Blocking; 30-120 s worst case.
pub fn install(release: &Release, exe: &Path) -> anyhow::Result<()>;
/// Parsing helpers, unit-tested without network.
pub fn parse_release(json: &[u8], current: &str) -> anyhow::Result<Option<Release>>;
pub fn is_newer(tag: &str, current: &str) -> anyhow::Result<bool>;
pub fn sha256_hex(bytes: &[u8]) -> anyhow::Result<String>;
/// Size and sha256 of the exe asset must both match; nothing else is trusted.
pub fn verify(bytes: &[u8], release: &Release) -> anyhow::Result<()>;
/// Inflate a downloaded `COMPRESSED_ASSET` and verify it like the exe asset.
pub fn inflate_verified(gz: &[u8], release: &Release) -> anyhow::Result<Vec<u8>>;
pub fn gunzip(gz: &[u8], expected_len: u64) -> anyhow::Result<Vec<u8>>;
/// One blocking GET; public so the live tests can hit any URL. Maps 403 and 404 to user-facing reasons.
pub fn fetch(url: &str, timeout: Duration) -> anyhow::Result<Vec<u8>>;
```

HTTP helper. WinRT is initialised per thread with `RoInitialize(RO_INIT_MULTITHREADED)`, tolerating `RPC_E_CHANGED_MODE`, and deliberately never released, so the updater keeps a thread-local "initialised" flag instead of the RAII guard `winocr.rs` uses. `RoInitialize` alone ties the process MTA's lifetime to whichever threads called it: when the last one exits, the MTA tears down mid-process and the activation factories `windows` caches in statics dangle, faulting the next WinRT call from a fresh thread (reproduced by `cargo test --test update -- --test-threads=1`). `winrt()` also calls `CoIncrementMTAUsage` once per process, behind a `std::sync::Once`, to pin the MTA for good with a cookie that is deliberately never released.

The HTTP helper requests headers first and reads the body in 64 KiB chunks.
Both `Content-Length` and the actual streamed byte count are checked: JSON is
limited to 5 MB, executable and notice bodies to their published sizes, and
compressed downloads to 100 MB. Empty executable assets are rejected. One
deadline covers the headers, stream acquisition and body reads, including a
server that sends headers and then stalls. An oversized response is closed
without buffering the rest. See `fetch_bounded` in `src/update.rs`.

The implementation follows Microsoft's [HTTP completion options](https://learn.microsoft.com/en-us/uwp/api/windows.web.http.httpcompletionoption)
and [input-stream reading API](https://learn.microsoft.com/en-us/uwp/api/windows.storage.streams.iinputstream.readasync).

Timeouts: 15 s for the JSON, 120 s for the download. A 15 MB asset took 0.5 s on the test machine and the v0.1.0 exe is 8.8 MB, so 120 s is there to cover a slow connection without hanging forever.

`install` first obtains an exclusive, delete-on-close file handle for
`poemercpricer-update.lock` in the executable folder. The handle covers the
download, rollback copy, replacement and metadata update, so another new
client cannot race that transaction. It is released on return or process
termination. This uses Windows' [file sharing rules](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.share_mode).
Older clients do not know about this lock; close extra copies when migrating.

`install` step by step:

1. If the release lists `poemercpricer-windows-x64.exe.gz`, `fetch` it (4.0 MB) and gunzip in memory with a `size + 1` read bound; on any error, or if the result fails step 2, `fetch(download)` the exe asset instead (8.8 MB, fine). Either way the bytes checked in step 2 are the inflated exe.
2. `bytes.len() == size` and `sha256_hex(&bytes) == sha256`, else bail with "checksum mismatch".
3. Write to `<exe dir>/poemercpricer.exe.<pid>.update` (same volume, so the swap is a rename, and Explorer never sees a half-written `poemercpricer.exe`). The pid keeps two running instances out of each other's temp file.
4. `fs::copy(exe, exe_dir/poemercpricer-previous.exe)`: one line of rollback insurance. README tells the user to rename it back if the new version fails to start.
5. `self_replace::self_replace(temp)`. The temp file is removed afterwards whether or not steps 4 and 5 succeeded.

The new installer uses the same executable update pipeline. Before step 3,
any listed `THIRD_PARTY_NOTICES.html` is downloaded and checked against its
own size and digest (declared size: 1 byte to 5 MB); failure aborts before the
swap. After a successful swap, verified notices are saved under the versioned
name `THIRD_PARTY_NOTICES-X.Y.Z.html`, and the Installed apps `DisplayVersion`
is refreshed only for the registered executable. Metadata or notice-write
failures are logged without reporting the completed binary update as failed.
Older listings without notices remain supported. Setup is never an update
payload. See [installation.md](installation.md) for installation and release checks.

A `PermissionDenied` from any of those writes is reported as "permission denied writing `<dir>`; move poemercpricer.exe to a folder you can write to"; other IO errors carry the directory in their context.

Measured: `self_replace` on a running exe took 177 ms, the old file was gone after the process exited, the new bytes were in place under the original name, and the process kept running. The swap happens at install time, so the next launch from any shortcut is the new version whether or not the user presses Restart.

`exe` is `std::env::current_exe()` captured once at startup and stored in `PricerApp`. Windows reports the load-time path even after the file is renamed underneath, but capturing it early costs nothing and removes the question.

### 3.3 Threading and messages

Same pattern as scanning: `thread::spawn`, `catch_unwind`, send on the existing channel, `ctx.request_repaint()`. Add one variant, `ScanMsg::Update(UpdateState)`, rather than a second channel. The worker sends `Checking`, then one terminal state; during install it sends `Downloading` first so the panel can say so.

Startup: `PricerApp::new` spawns the check after the first frame (send it from `update()` when a `first_frame` bool flips) so the window is visible before any network activity. Conditions for the automatic check, all required:

- release build (`!cfg!(debug_assertions)`), so `cargo run` in a checkout never replaces `target/release/poemercpricer.exe` under the developer;
- `cfg.check_updates` is true;
- `--no-updates` was not passed.

The Check now button ignores all three: it is how a user retries after an offline launch, or checks once with automatic checks turned off. It is disabled only while a check or download is in flight.

If `cfg.install_updates_automatically` is true and `check` returns a release, the same worker continues straight into `install`. Otherwise it stops at `Available` and waits for the user.

Periodic re-check, without a timer thread: `last_update_check: Option<Instant>` is set whenever a check starts, and a pure `recheck_in(last, state, enabled) -> Option<Duration>` says how long until the next one (`UPDATE_RECHECK` is 6 h). It returns `None` when checks are disabled or the state is `Checking`, `Downloading` or `Ready` (an installed update waits for the restart and is never downloaded over). In `update()` the app either starts the check when the duration is zero or calls `ctx.request_repaint_after(remaining)`; eframe turns the last frame's deadline into a `WaitUntil`, so this adds no polling and no continuous repaint. A hidden window gets no frame, so a re-check that falls due while hidden runs when the window is next shown. A periodic check follows the same auto-install rule as the startup check.

### 3.4 Restart

`PricerApp::restart(&mut self)`:

1. `self._hotkeys.take()`: dropping `GlobalHotKeyManager` calls `DestroyWindow` on its hidden window, which frees the `RegisterHotKey` binding, so the new process can register `Ctrl+Shift+M` without a race.
2. `Command::new(&self.exe).args(std::env::args_os().skip(1)).spawn()`. Elevation is inherited, so an elevated instance (needed when PoE is elevated) restarts elevated with no extra UAC prompt.
3. `ctx.send_viewport_cmd(ViewportCommand::Close)`.

If the spawn fails, this process keeps running, so it re-registers the hotkey it just gave up and reports `Could not restart: <error>`.

Never restart on the app's own initiative. A scan may be in flight or the user may be mid-trade with the overlay open.

### 3.5 Config and CLI

`AppConfig` gains two fields; `#[serde(default)]` on the struct already makes old config files pick up the new defaults:

```rust
pub check_updates: bool,                  // default true
pub install_updates_automatically: bool,  // default true
```

CLI: `--no-updates` (skip the startup and periodic checks, same name as APT) and `--version` (prints `CARGO_PKG_VERSION`, for bug reports). Move the ad-hoc argument loop in `main.rs` into `fn parse_args(&[String]) -> Result<Args>` so both flags get a unit test.

## 4. Release pipeline

`.github/workflows/release.yml` builds and publishes every `v*` tag:

```yaml
name: release
on:
  push:
    tags: ["v*"]
permissions:
  contents: write
  actions: read
jobs:
  build:
    runs-on: windows-latest
    steps:
      # Actions pinned to commit SHAs (tag in a trailing comment): this job
      # holds the write token and every running copy auto-installs its output.
      - uses: actions/checkout@<sha>        # v4
      # Reuses the ci run's fmt, test, clippy, audit and MSRV gates instead of
      # duplicating them; a tag on a commit that never reached main can't publish.
      - name: Tagged commit must be on main with a green ci run   # polls gh run list, then gh run watch --exit-status
      - uses: dtolnay/rust-toolchain@<sha>  # stable
        with:
          toolchain: stable               # required when the ref is not a channel name
      - uses: Swatinem/rust-cache@<sha>     # v2
        with:
          cache-all-crates: true   # keeps the cargo-about install below a no-op
      - name: Tag must match Cargo.toml
        shell: pwsh
        run: |
          $v = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).packages[0].version
          if ("v$v" -ne $env:GITHUB_REF_NAME) { throw "tag $env:GITHUB_REF_NAME does not match Cargo.toml version $v" }
      - run: cargo build --release --locked
      - name: Size budget          # throws above 11,000,000 bytes; docs/performance.md
      - run: Copy-Item target/release/poemercpricer.exe poemercpricer-windows-x64.exe
      - name: Gzip sibling asset   # .NET GZipStream at SmallestSize
      - run: cargo install cargo-about --version 0.9.2 --locked --features cli
      - name: Third-party notices from the tagged Cargo.lock
        run: cargo about generate --locked --fail about.hbs -o THIRD_PARTY_NOTICES.html
      - run: ./scripts/test-installer.ps1 -Payload poemercpricer-windows-x64.exe
      - run: ./scripts/build-installer.ps1 -Payload poemercpricer-windows-x64.exe
      - run: ./scripts/verify-release-assets.ps1
      - run: ./scripts/test-installer.ps1 -Payload poemercpricer-windows-x64.exe -Setup target/installer/poemercpricer-setup-windows-x64.exe
      # Immutable releases lock the tag and assets at publish, so everything
      # is uploaded to the draft first.
      - run: gh release create $env:GITHUB_REF_NAME poemercpricer-windows-x64.exe poemercpricer-windows-x64.exe.gz THIRD_PARTY_NOTICES.html target/installer/poemercpricer-setup-windows-x64.exe --draft --title $env:GITHUB_REF_NAME --generate-notes
        env:
          GH_TOKEN: ${{ github.token }}
      - run: gh release edit $env:GITHUB_REF_NAME --draft=false
        env:
          GH_TOKEN: ${{ github.token }}
```

The bodies of the PowerShell steps above are elided; `release.yml` is the source of truth.

The app, installer and uninstaller are unsigned. The updater trusts the size and SHA-256 digest published by GitHub; it does not depend on a signing service or publisher certificate.

`gh` is preinstalled on the runner, so no marketplace action is needed. CI already tests every push to `main`; tags are cut from `main`, so the release job reuses those checks and additionally tests the final installer. The job still requires the tagged commit to be on `main` with a green `ci.yml` run before it builds anything, since the tag push and the `main` push race each other. The notices file is generated from the tagged `Cargo.lock` and never committed; `about.toml` and `about.hbs` at the repo root configure it. GitHub adds the `digest` itself, asynchronously, which is why the release script polls for it. The release is created as a draft, all four assets are uploaded to it, and only then is it published; once published, immutable releases mean the tag and assets can never be changed or deleted, so a bad release is fixed by shipping a new patch version, not by editing the old one.

Cutting a release is one command, `.\scripts\release.ps1 -Version X.Y.Z` (rehearse with `-DryRun`). It refuses to run on a private repo, on a dirty tree, off `main`, or with secret-shaped text tracked. Then it bumps `Cargo.toml`, refreshes the lock, builds, checks the exe against the 11,000,000 byte budget, runs fmt, clippy and the tests, commits `Release X.Y.Z`, tags, pushes, watches the workflow run, and polls for up to three minutes until the uploaded exe carries a sha256 digest, requiring valid digests for the exe, `.gz`, notices and installer. It also builds/tests the installer locally; see `docs/installation.md` for the installer build and final-package tests. The runbook with failure recovery is `AGENTS.md` at the repo root.

The tag check makes a mismatched tag fail loudly instead of publishing an exe that reports the wrong version and then re-updates itself forever.

## 5. UI and UX

Follows `docs/ui-design.md`: text carries state, colour supplements it, verbs on buttons, no modal, no animation, reactive repaint only.

### 5.1 Where it shows

Command bar. One extra button appears only when there is something to do, placed left of Settings. The examples below run 0.1.0 with 0.1.1 published:

| State | Button | Tooltip |
|---|---|---|
| `Ready` | Restart to update (gold text on the normal button fill; disabled while a scan runs) | "0.1.1 is installed and runs on the next start. Restarting now takes about a second." |
| `Available` (notify-only mode) | Update to 0.1.1 | "Downloads 9 MB from GitHub, verifies it, and replaces poemercpricer.exe. You choose when to restart." |
| anything else | nothing | |

This fixes the APT complaint: the user sees the pending restart every time they look at the overlay, and the button says exactly what will happen.

Status line. For actions the user started (Check now, Update, Restart) it mirrors every state. A background check (startup or periodic) writes the status line only when there is something to do: `Ready` gives "0.1.1 is installed. It runs on the next start, or use Restart to update." and `Available` gives "0.1.1 is available. Update from the command bar." A failed background check goes to the console log, never to a red status line, because "offline at launch" is not an error the user needs to act on.

Settings > Updates. The section sits between Trade and Advanced:

```
Updates
Version 0.1.0                         Release notes
<line 1: state>
<line 2: detail>
[ Check now ]  or the state's button
[x] Check for updates at startup and every 6 hours
[x] Install updates automatically
```

`Release notes` opens `RELEASES_URL` in the browser (the overlay does not render markdown). The two lines and the button are one match per state, same shape as APT's About panel. `app.rs`'s `update_copy_matches_the_spec_table_for_every_state` test asserts this table:

| State | Line 1 | Line 2 | Button |
|---|---|---|---|
| `Idle`, checks enabled | `Not checked yet.` | `Checks when the app starts and every 6 hours.` | Check now |
| `Idle`, checks disabled | `Update checks are off.` | `Off with --no-updates.` or `Off in Settings.` | Check now |
| `Checking` | `Checking GitHub for a newer release…` | | none (Check now disabled) |
| `UpToDate` | `0.1.0 is the latest release.` | `Checked 3 min ago. Checks again in about 6 h.` | Check now |
| `Available` | `0.1.1 is available.` | `Install updates automatically is off. Update to 0.1.1 downloads it now; it runs after a restart.` | Update to 0.1.1 |
| `Downloading` | `Downloading 0.1.1 (9 MB)…` | `The current version keeps working until you restart.` | none |
| `Ready` | `0.1.1 is installed.` | `It runs the next time you start PoEMercPricer, or restart now. Until then 0.1.0 keeps running.` | Restart to update |
| `Failed` | `Could not update: <short reason>.` | `Nothing was changed. You can download it yourself.` | Check now, Open releases page |

Short reasons come from the backend and are complete sentences: `no internet connection`, `GitHub rate limit, try again in an hour`, `the release has no Windows download`, `checksum mismatch`, `permission denied writing C:\Program Files\...` (with the hint `move poemercpricer.exe to a folder you can write to`).

### 5.2 Rules

- No modal, no countdown, no toast. The command-bar button and the Settings section are the whole surface.
- Never auto-restart, never install on exit.
- "Install automatically" does exactly one thing: it decides whether `Available` proceeds to `install` without a click. It never decides when the restart happens.
- Keyboard: both buttons are ordinary egui buttons in tab order. Escape still closes Settings.
- Contrast: gold on the button fill is the same pair already used for Scan; state text uses `TEXT` and `MUTED`; the Failed line uses `RED` with the text saying "Could not update".
- Reactive: the worker requests exactly one repaint per state change. The "Checked 3 min ago" text is computed at paint time from an `Instant` and does not schedule repaints.
- Fixture mode (`--fixture`) behaves like a normal launch; the check is skipped by the debug-build rule during development anyway.

## 6. Failure modes

| Situation | Behaviour |
|---|---|
| Offline at startup | Silent; `update check skipped: <error>` on stderr, visible only when the exe was started from a terminal. Settings shows `Failed` with the reason. |
| Repo made private, or no releases yet (404) | Same as offline: `Failed: no release published yet`. This was the state before v0.1.0 was published. |
| Rate limited (403) | `Failed`, reason names the limit. |
| Tag not `vX.Y.Z` | `Failed: release tag "x" is not a version`; protects against a hand-made tag. |
| Asset missing | `Failed: the release has no Windows download`. |
| Asset over 100 MB | `Failed: the release download is implausibly large (<n> bytes)`, before anything is fetched. |
| Release has no `digest` | `Failed: the release has no usable sha256 checksum`. A digest that is not 64 hex characters is rejected the same way. |
| Compressed sibling missing or corrupt | Silent: the exe asset is downloaded and verified instead. |
| Size or checksum mismatch | `Failed`, temp file deleted, exe untouched. |
| Exe folder not writable (Program Files, read-only share) | `Failed` with the folder path and the move hint. |
| Antivirus quarantines the swap | Surfaces as an OS error in `Failed`; `poemercpricer-previous.exe` is still there. |
| New version crashes at start | User renames `poemercpricer-previous.exe` back. Documented in README. |
| Restart while a scan is running | Restart to update is disabled while a scan is running, like Scan is. A scan past the 30 s watchdog stops counting as running. |
| Two instances updating one folder | An exclusive `poemercpricer-update.lock` handle covers download, verification, rollback and replacement. A competing copy reports that another update is running and can retry. Windows deletes the lock on handle close, including process exit. A later attempt skips the swap when the target digest is already installed, preserving rollback. Older published clients do not participate in this lock. |
| Downgrade on GitHub (latest tag lower than running) | `is_newer` is strict `>`, so nothing happens. |

The app and installer are unsigned. Windows or antivirus software may block a downloaded executable or a self-update; the updater reports filesystem failures and does not bypass those protections.

## 7. Tests

Offline by default; CI has no network guarantees. The live tests are `#[ignore]` and run on demand.

- `tests/update.rs` (offline)
  - `parse_release` on `tests/fixtures/release-latest.json`, a `releases/latest` body trimmed to three assets (the exe, its `.gz` sibling and a PDB, so the "ignore everything else" rule is exercised).
  - The `.gz` sibling: `compressed` holds its URL when present and is `None` when absent, while `size`/`sha256` always describe the exe asset. `gunzip` round-trips, stops at `size + 1` on an over-long stream, and errors on non-gzip input. `inflate_verified` (Windows, uses `BCryptHash`) accepts the exact bytes and rejects a flipped bit, a short stream, a long stream and garbage.
  - `is_newer`: `v0.3.0 > 0.2.0`, `v0.2.0 == 0.2.0` is false, `v0.2.0-rc1` is false, `3.29` errors.
  - `sha256_hex(b"abc")` equals the FIPS 180 vector `ba7816bf…f20015ad` (exercises the `BCryptHash` wrapper; Windows-only test like the OCR ones).
  - Missing `digest` or missing exe asset error with "checksum" / "Windows download" in the message; a `size` over 100 MB errors with "implausibly large".
  - `winrt_survives_a_previous_updater_thread_exiting` spawns an updater thread, joins it, then hashes from a fresh thread: without the `CoIncrementMTAUsage` pin this faults on the dangling activation factory. Run the file single-threaded to reproduce the old behaviour.
- `tests/update_http.rs` checks exact-length responses, oversized Content-Length, chunked and headerless overflows, and a body stalled after headers.
- `tests/update_install.rs` also verifies that a competing process cannot touch the executable while another owner holds the update lock, and that retry succeeds after the owner releases it.
- `tests/update_install.rs` (offline, Windows): `install()` end to end. A `std::net::TcpListener` on a random loopback port plays GitHub, and the test copies its own binary into a temp folder and runs that copy as a child process (`<copy> install_child --exact`, release passed in `PMP_INSTALL_*` env vars), so `self_replace` swaps the copy and never the running harness. The child writes `ok` or the error to a result file; the parent then reads the folder. Covered: the gzip sibling is used when it is the only route that answers; a missing (404) or corrupt `.gz` falls back to the exe; a release without a sibling installs from the exe; after a success `poemercpricer.exe` holds the served bytes, `poemercpricer-previous.exe` holds the old exe, and no `.update` temp file is left; a digest mismatch, a size mismatch, a 404 on the exe and a refused connection all leave the exe untouched with no `previous.exe` and no temp file, and the refused connection reads `no internet connection`; an exe that already holds the release (another instance got there first) is left alone with no `previous.exe` written.
- `tests/update_live.rs` (network, `cargo test --test update_live -- --ignored`)
  - `check()` against the real repo: `Ok(Some)` when the tag is newer than the running build, `Ok(None)` when it is not, and an error only if it reads `no release published yet`.
  - `fetch` the `cli/cli` latest release, download its smallest digested asset through the 302, and assert the length and the `BCryptHash` SHA-256 match GitHub's `digest`. This is the spike from section 12 kept as a test.
  - The published `.exe.gz` of the latest real release (written by .NET `GZipStream` in `release.yml`) downloads and `inflate_verified` accepts it against the exe asset's size and digest, so the gzip route every installed copy tries first is known to work with `flate2`.
  - `fetch` of a repo that does not exist returns the mapped `no release published yet`, not a raw HRESULT. A repo that does not exist and a private one answer 404 alike, which is what the mapping is for.
  - `install()` is never called in-process here or anywhere else: `self_replace` would overwrite the running test harness. `tests/update_install.rs` runs it in a child process instead.
- `src/main.rs` unit tests: `parse_args(["--no-updates"])`, `["--fixture", "--no-updates"]`, unknown flag still errors, `--scan` without a path errors.
- `src/app.rs` unit tests: `update_copy` for every `UpdateState`, `checked_ago` at 5 s, 3 min, 2 h, and `recheck_in` (disabled, Ready and Checking give `None`; never checked and 7 h ago are due now; 1 h ago waits about 5 h).
- App worker tests exercise the same check/install orchestration used by the overlay: automatic mode emits Downloading before installation, manual mode leaves the release Available, success preserves the version for Restart, and check errors, install errors and worker panics remain retryable. These use injected network/install operations without opening a native window. Debug builds reject manual replacement as well as automatic replacement; Check now still works, and release builds retain both installation paths.
- `tests/cli.rs`: `--version` prints `CARGO_PKG_VERSION` and exits 0.
- `tests/config_roundtrip.rs`: an old config without the two new keys loads with both true.
- Live run of the release exe, once per release: start it with the check enabled, confirm the overlay comes up with no red status and that a failed check only writes `update check skipped: <reason>` to stderr (start it from a terminal to see it), screenshot with `scripts/screenshot-window.ps1`. Start it again with `--no-updates` and confirm no log line.
- Manual self-update check, once per release: build locally with `version` set one patch below the published tag, run that exe, confirm it goes `Checking`, `Downloading`, `Ready`, press Restart, confirm Settings shows the published version and `poemercpricer-previous.exe` exists. Delete that file, turn off "Check for updates at startup and every 6 hours", relaunch, and confirm nothing happens.

## 8. Documentation changes (the plan; all of these landed in v0.1.0)

- `README.md`
  - Install: add "Download (Windows)" above "From source": get `poemercpricer-windows-x64.exe` from the Releases page, rename it to `poemercpricer.exe` if you like, and put it in a folder you can write to (not Program Files) so updates can replace it. Mention the SmartScreen prompt on first run.
  - New Updates section: one request to `api.github.com` at startup, what gets downloaded, verification by SHA-256 against GitHub's digest, the two Settings checkboxes, `--no-updates`, the restart button, and `poemercpricer-previous.exe` as the rollback.
  - Features list: "Checks GitHub Releases for a newer version at startup, installs it after verifying the checksum, and asks you to restart. Off with `--no-updates` or in Settings."
  - Usage table: add Restart to update and the Settings entry for Updates; CLI block gains `--version` and `--no-updates`.
  - Safety / ToS: add "One HTTPS request to GitHub at startup to look for a new version; nothing else leaves the machine."
- `SECURITY.md` scope: add outbound HTTPS to `api.github.com` and `github.com` for release downloads, that the app replaces its own executable only after the SHA-256 matches GitHub's published digest, and that `--no-updates` removes all network activity.
- `CONTRIBUTING.md`: add a Releases section with the procedure from section 4 and the rule that the tag must equal `v` + `Cargo.toml` version.
- `docs/ui-design.md`: add the rules from section 5.2 to Design rules and "Test Updates: not checked, up to date, available with auto-install off, downloading, ready, failed offline" to the review checklist.
- `THIRD_PARTY_NOTICES.md`, Rust crates paragraph: add `self-replace` (Apache-2.0).
- `.github/ISSUE_TEMPLATE/bug_report.md`: change the version line to "PoEMercPricer version (Settings > Updates, or `poemercpricer --version`):".
- `src/main.rs` `print_help`: two new lines for `--version` and `--no-updates`.

## 9. Implementation order (as planned on 2026-09-03; all steps done)

1. Make the repo public and confirm `gh api repos/Lisood/PoEMercPricer/releases/latest` returns 404 for "no releases" rather than for "private". Nothing below is testable end to end before this.
2. `Cargo.toml`: `semver = "1"`, `self-replace = "1"`, the four `windows` features. `cargo build` to confirm the lock only gains `self-replace`.
3. `src/update.rs` with `parse_release`, `is_newer`, `sha256_hex`, `fetch`, `check`, `install`. Unit tests and the fixture JSON.
4. `AppConfig` fields, `parse_args` with `--no-updates` and `--version`, `print_help`.
5. `PricerApp`: `update_state`, `exe`, `no_updates`, `first_frame`; `ScanMsg::Update`; `start_update_check`, `start_install`, `restart`; command-bar button; Settings section.
6. `.github/workflows/release.yml`.
7. Documentation from section 8.
8. `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, release build, the manual walkthrough from section 7.
9. Tag `v0.1.0`. The first real self-update is verified when `v0.1.1` ships; until then the local-build-one-patch-below trick covers it.

Estimated size: `update.rs` about 180 lines, `app.rs` about 120 lines, `main.rs` about 30 lines, workflow 25 lines.

## 10. Deliberately left out

- A timer thread for the periodic re-check. `request_repaint_after` already gives a wake-up at the right time with no thread and no polling.
- "Install on exit". The swap has already happened by the time the user is told, so the next start is the new version whether the app is closed by the user, by Windows shutdown, or by a crash. That is what APT's "installed on exit" promise fails to deliver (issue #846).
- Delta or block-map downloads. The exe is 8.8 MB, 4.0 MB gzipped, and downloads in about a second. Section "Binary size and memory" in `docs/performance.md` has the `bidiff` note.
- Release notes inside the overlay. A link to the release page does the job without a markdown renderer.
- A "skip this version" option. Turn off automatic install, or `--no-updates`.
- Code signing. Releases are unsigned; download integrity is checked against GitHub's release metadata.
- Prerelease or beta channel. `releases/latest` ignores prereleases, which is the behaviour I want; a beta can still be published as a prerelease for people who download it by hand.
- Portable uninstall. Delete the exe; delete the config folder only if you want to remove settings too. Installed copies use Windows Installed apps; see [installation.md](installation.md).

## 11. What a release must not contain

Audited 2026-09-03 on the working tree and a local release build:

| Risk | Finding | Guard |
|---|---|---|
| Local username in the exe | 117 occurrences of the Windows username in a locally built `poemercpricer.exe`, from `.cargo\registry` panic-location paths of dependencies | Only CI builds are published; `release.ps1` never uploads anything, `release.yml` builds on the runner |
| Debug symbols | `debug = "line-tables-only"` writes a PDB next to the exe | The workflow uploads four named files: the exe, its `.gz` sibling, installer and `THIRD_PARTY_NOTICES.html` |
| Session cookies and tokens | none tracked (`POESESSID`, `ghp_`, `github_pat_`, `AKIA`, private keys) | CI "No secrets" step and `release.ps1` step 0 |
| Account names in data | `assets/*.json` hold prices and counts only | Refresh scripts request no seller fields |
| Screenshots | samples show the mercenary panel and game UI, no chat or account names | Bug template asks reporters to crop |
| Debug captures and logs | written under `%APPDATA%`, never in the repo | `.gitignore` blocks `debug/` and `*.log`; `release.ps1` refuses tracked ones |
| Commit messages | quoted by `--generate-notes` on the release page | Rule in `AGENTS.md` |
| Email | none in the exe | `authors` is the GitHub handle |

Not done on purpose: `--remap-path-prefix` for local builds (they are never published), SBOM or `cargo deny` (dependabot plus `cargo audit` already run in CI), and a git-history rewrite (nothing to remove).

## 12. Verification log, 2026-09-03

Spike at a throwaway crate with `windows 0.58` (features above), `self-replace 1.5.0`, `serde_json`; release build 17 s.

- `GET api.github.com/repos/cli/cli/releases/latest`: 200 in 412 ms; `assets[].digest` present as `sha256:<hex>`.
- Asset download through the 302: 15,347,712 bytes in 498 ms; length equal to `size`.
- `BCryptHash` SHA-256 of the download: 12 ms; equal to the digest.
- The HTTP helper from section 3.2, then named `get` (`Status()` poll, `Cancel()`, `GetResults()`), compiled against `windows 0.58` and ran both requests above unchanged.
- `gh release create --generate-notes --title` flags exist on the installed `gh`; the `cargo metadata` one-liner in the workflow prints `0.1.0` for this repo.
- `GET api.github.com/repos/Lisood/PoEMercPricer/releases/latest` (private, no releases): 404 surfaced by `EnsureSuccessStatusCode` as `0x80190194`.
- `self_replace` of a running exe with a byte-different copy: 177 ms, process kept running, file under the original name had the new bytes, no leftover temp file 3 s after exit.
- `Cargo.lock` already holds `semver 1.0.28`, `tempfile 3.27.0`, `fastrand 2.5.0`, `windows-sys 0.52.0`, `once_cell 1.21.4`; `self-replace` is the only new crate.
- `global-hotkey 0.6.4` Windows `Drop` is `DestroyWindow(self.hwnd)`, which releases the hotkey for the restarted process.
- APT sources cited in section 1 were read from master the same day.

Implementation verification, same day, after the code landed:

- `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`: clean. `cargo test --locked`: 176 tests over 15 binaries, 0 failures; 3 live tests ignored by default.
- `cargo test --test update_live -- --ignored`: 3 passed in 0.9 s (real `check()` against the private repo maps to `no release published yet`; a 1950-byte `cli/cli` asset downloaded through the 302 and its `BCryptHash` SHA-256 matched GitHub's digest).
- Release exe started with the check on: console printed `update check skipped: no release published yet`, the overlay opened with the normal grey prompt and no red status. Settings showed Version 0.1.0, the red `Could not update` line, `Check now`, `Open releases page`, and both checkboxes with the 6-hour label (screenshots taken with `scripts/screenshot-window.ps1`).
- Same exe with `--no-updates`: window came up, console stayed empty.
- `scripts/release.ps1 -Version 0.1.0 -DryRun`: bump, `cargo update -p poemercpricer`, release build, fmt, clippy, full tests, then restore; `Cargo.toml` byte-identical afterwards. A version whose tag already exists and `-Version 1.2` fail on the right precondition. The secret patterns match four synthetic positives and nothing in the tree.
- The workflow's tag check passes for `v0.1.0` and throws for `v9.9.9` when run locally with `GITHUB_REF_NAME` set.

## 13. First release, 2026-09-04

The repo went public and `v0.1.0` was tagged and built by `release.yml`. What the release carries:

| Asset | Bytes |
|---|---|
| `poemercpricer-windows-x64.exe` | 8,779,776 |
| `poemercpricer-windows-x64.exe.gz` | 4,007,868 |
| `THIRD_PARTY_NOTICES.html` | 222,749 |

Sha256 digests for each asset are on the release page. The `.exe.gz` digest is
published but unused by the updater (the exe's digest is what the inflated
bytes are checked against); `THIRD_PARTY_NOTICES.html` is generated on the
runner from the tagged `Cargo.lock`.

The release is neither a draft nor a prerelease, so `releases/latest` returns it and every installed copy can see it. `check()` against the real repo returns `Ok(None)` for a 0.1.0 build, where a private repo gave the mapped 404.

Still unverified: an update installed from one published release to the next. That needs `v0.1.1`; until then `tests/update_install.rs` (the swap, against a loopback server) and `live_gzip_asset_inflates_to_the_exe_digest` (the published `.gz` against the real digest) together cover every step of `install` except the GitHub `releases/latest` lookup itself, which `live_check_against_real_repo` exercises.
