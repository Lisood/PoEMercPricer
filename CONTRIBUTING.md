# Contributing

PoEMercPricer is a small Windows-only Rust overlay. Keep changes scoped: one
concern per pull request.

## Checks before you push

Run these three, in order. CI runs the same ones and fails on any of them.

```powershell
cargo fmt
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Rust 1.88 is the minimum supported version, and CI checks it separately.

Additional suites are opt-in because they need a network, a desktop, or this
particular machine:

```powershell
cargo test --test update_live -- --ignored
cargo test --locked --test update_install live_published_app_installs_and_runs_its_version_command -- --ignored --exact
cargo build --release
cargo test --release --test resource_budgets -- --ignored --test-threads=1
```

The size and memory budgets and the scan-latency test in `scan_screenshots`
measure the machine they run on. `docs/performance.md`, under "Binary size and
memory", says what the numbers mean.

Do not run the GUI in automation. `cargo run`, `--fixture` and `--scan FILE`
all open a window and, in a release build, register the hotkey and start an
update check. `--help`, `--version`, `score`, `clipboard` and the `dump-*`
subcommands print to stdout and exit.

## Where things live

Installer/updater changes also require `scripts/test-installer.ps1` after a
release build and notice generation, plus `scripts/test-release-dryrun.ps1`.
The installer script uses isolated registrations and never opens the overlay.
CI runs the lifecycle; `update-live.yml` runs real GitHub checks weekly and
on manual dispatch. Build/release details are in `docs/installation.md`.

- Scoring math: `src/scoring/`. A formula change needs a unit test in
  `tests/scoring.rs`.
- Overlay UX: `src/app.rs`. Layout and accessibility rules are in
  `docs/ui-design.md` and `docs/ui-guide.md`.
- Capture, OCR and icon recognition: `src/capture.rs`, `src/winocr.rs`,
  `src/vision.rs`, `src/scan.rs`.
- Updater: `src/update.rs`, designed in `docs/updater.md`.
- Installer and Windows integration: `installer/`, `src/installation.rs`,
  `scripts/build-installer.ps1`, `scripts/test-installer.ps1`.
- Market data and its provenance: `assets/`, `docs/market-3.29.md`,
  `docs/research-3.29.md`.

Behaviour changes update the matching file under `docs/` in the same commit.

## Rules

1. No process injection, packet sniffing, or input playback.
2. No network calls beyond the updater's GitHub Releases check.
3. Windows is the only supported platform.
4. No AI attribution in commits, pull request bodies, or code comments. The
   `commit-msg` hook rejects those trailers; do not bypass it with
   `--no-verify`.

## Releases

`AGENTS.md` section 3 is the procedure: preconditions, what
`.\scripts\release.ps1 -Version X.Y.Z` does step by step, how to rehearse it
with `-DryRun`, what must never reach a release, and how to recover from each
way it can fail. Ask before pushing a tag; a tag is a release, and every
installed copy offers it as an update on its next launch.

## Reporting bugs

Use the bug report issue template. Crop screenshots to the mercenary panel and
leave out chat, the friends list, and account names. Issues are public.

Security issues go through GitHub's private vulnerability reporting instead.
See [SECURITY.md](SECURITY.md).
