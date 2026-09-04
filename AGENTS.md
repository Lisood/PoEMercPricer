# AGENTS.md

Instructions for automated coding agents and contributors working in
this repository. Read this before making changes.

## 1. What this is

PoEMercPricer is a Windows-only Rust egui overlay that screens Path of Exile
3.29 Mercenary Warrants, either from a pasted clipboard item or from an OCR
scan of the game window. It builds to one unsigned exe, targets Rust 1.88+,
and has no server component. Running `cargo run`, `poemercpricer --fixture`,
or `poemercpricer --scan FILE` opens the GUI window and, in a release build,
starts a hotkey listener and an update check; never run these in automation
or CI. Safe to run non-interactively: `--help`, `--version`, `score`,
`clipboard`, and the `dump-*` subcommands (`dump-scan`, `dump-trade-query`,
`dump-clipboard-scan`, `dump-window-scan`) all print to stdout and exit.

## 2. Checks before any commit

Run, in order: `cargo fmt`, `cargo clippy --all-targets --locked -- -D
warnings`, `cargo test --locked`. Live-network tests are opt-in and not part
of the normal loop: `cargo test --test update_live -- --ignored`.
Size and memory gates are opt-in too (they measure this machine): after
`cargo build --release`, run `cargo test --release --test resource_budgets --
--ignored --test-threads=1` and the ignored latency test in
`scan_screenshots`; see `docs/performance.md`, "Binary size and memory".

## 3. How to ship an update (release)

Preconditions: you are on `main`, the working tree is clean, `gh auth status`
succeeds, and CI is green on `main`. The script also refuses to run against a
private repo, because the updater 404s for every user while the repo is
private, and refuses a version that already has a tag locally or on origin.
It pushes `main` and the tag together, atomically.

Command: `.\scripts\release.ps1 -Version X.Y.Z`. Rehearse first with
`.\scripts\release.ps1 -Version X.Y.Z -DryRun`, which runs the local checks
and build for real but never commits, tags, or pushes.

What the script does:
0. Greps the tracked tree for secret-shaped text and for tracked debug or log
   files, and refuses to go on if either turns up.
1. Rewrites the version line in `Cargo.toml`.
2. Runs `cargo update -p poemercpricer` and `cargo build --release --locked`,
   then fails if the release exe is over 11,000,000 bytes. `release.yml` runs
   the same size gate on the CI build; see `docs/performance.md`, "Binary size
   and memory".
3. Runs `cargo fmt --check`.
4. Runs `cargo clippy --all-targets --locked -- -D warnings`.
5. Runs `cargo test --locked`.
6. Commits `Release X.Y.Z`.
7. Tags `vX.Y.Z`.
8. Pushes `main` and the tag together.
9. Watches the `release.yml` run for that tag. The job first checks the
   tagged commit is on `main` and waits for that commit's `ci.yml` run to go
   green, then builds and uploads the three assets to a draft release, then
   publishes it.
10. Verifies `poemercpricer-windows-x64.exe` (and its `.exe.gz` sibling) was uploaded with a sha256
    digest, that `THIRD_PARTY_NOTICES.html` is there too, and prints the
    release URL.

`release.yml` builds the exe, gzips it, then runs `cargo about` over the
tagged `Cargo.lock` to produce `THIRD_PARTY_NOTICES.html` and uploads all
three to a draft release before publishing it. The notices file is
generated, never committed; `about.toml` and `about.hbs` at the repo root
configure it.

A published release is immutable: its tag and assets can't be changed or
deleted. Fix a bad published release by shipping a new patch version, not by
touching the old one. A failed workflow leaves at most a draft behind, since
publishing is the last step; `gh release delete vX.Y.Z --yes` removes the
draft before you delete the tag.

Success looks like the script printing the release URL and a line like
`poemercpricer-windows-x64.exe 12345678 sha256:<hex>`. Every running copy of
the app sees the update on its next launch.

What to do when something fails:
- The tag-mismatch check in `release.yml` fails: the tag does not match
  `Cargo.toml`'s version. Bump `Cargo.toml` and cut a new tag; do not retag
  the same version.
- The workflow itself fails: `gh run view <id> --log-failed`, fix the cause,
  delete the bad tag both locally and on origin (`git tag -d vX.Y.Z` and
  `git push origin :refs/tags/vX.Y.Z`), delete the draft release if one was
  created (`gh release delete vX.Y.Z --yes`), and rerun the script.
- An uploaded asset has no digest: GitHub computes it asynchronously, so step
  10 polls for up to 3 minutes before failing. If it still fails, wait a
  minute more and re-check with `gh release view vX.Y.Z --json assets`.

What must never reach a release, and what enforces it:
- Session cookies, tokens, keys. `POESESSID` is the realistic one for a PoE
  tool: never paste it into a script, test, or fixture. The CI "No secrets"
  step and `release.ps1` step 0 grep the tracked tree and fail on it.
- Your own machine. A locally built exe embeds `C:\Users\<you>\.cargo\...`
  paths from dependency panic locations (measured: 117 copies of the Windows
  username in a local release build). Only the CI-built exe is published;
  never upload a local build to a release by hand. The PDB is not uploaded.
- Screenshots with chat, friends list, or account names. Crop samples to the
  mercenary panel; the bug template says the same to reporters.
- Debug captures and logs. They go to `%APPDATA%\PoEMercPricer\...\debug`,
  never into the repo, and `.gitignore` blocks `debug/` and `*.log`.
- Commit messages: `--generate-notes` quotes them on the public release page,
  so keep names, emails, and account details out of them.
- Trade snapshots under `assets/` carry prices and counts only, no seller
  or account names; keep it that way when refreshing them.
The repo is public, so keep GitHub secret scanning and push protection on
under Settings > Code security. Both are free for public repos and they catch
what the grep above misses.

Semver: patch for a catalog, price, or bugfix change; minor for a feature;
major has never been needed. The updater only ever sees tags shaped
`vX.Y.Z` on non-prerelease releases; prereleases are invisible to it.

## 4. How the updater works

See `docs/updater.md` sections 3-6 for the full design. In short: the app does
one `GET releases/latest` at startup and every 6 hours and looks for an asset
named exactly `poemercpricer-windows-x64.exe`. It downloads the smaller
`.exe.gz` sibling when the release has one and inflates it, falling back to the
exe asset on any failure. Either way the bytes are checked against the exe
asset's size and GitHub's published SHA-256 digest before anything is written.
The running exe is then replaced in place and a "Restart to update" button
appears; nothing restarts on its own. The `REPO` and `ASSET` constants in
`src/update.rs` are load-bearing: renaming the asset or moving the repo breaks
every installed copy's ability to update, so never do either without a plan for
existing users.

## 5. Rules

1. No AI attribution in commits, PR bodies, or code comments. The
   `commit-msg` hook rejects `Co-Authored-By` and similar trailers; do not
   bypass it with `--no-verify`.
2. Do not add process injection, packet sniffing, or input playback.
3. Do not add network calls beyond the updater's GitHub Releases check.
4. Windows is the only supported platform.
5. Keep changes small and scoped.
6. Update the relevant file under `docs/` in the same change whenever you
   change behaviour.
7. Ask before pushing a tag. A tag is a release: once `release.yml` builds
   and uploads the exe, users' apps will offer it as an update.
