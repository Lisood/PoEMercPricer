# Installer and updater review — 2026-09-05

The installer, updater, app worker integration, registration handling, tests
and release scripts were reviewed locally. Nothing was committed, tagged,
pushed or published. The final local installer is 5,747,108 bytes; its app is
8,805,376 bytes, within the existing 8 MB and 11 MB packaging gates.

## Findings fixed

| Finding | Impact | Fix and regression evidence |
|---|---|---|
| Competing update transactions could race between checking the installed hash and copying rollback | A second instance could damage the rollback copy despite separate download filenames | An exclusive, delete-on-close Windows file handle covers the transaction. A child-process test verifies refusal while another owner holds the lock and successful retry afterwards. |
| HTTP responses were buffered before their actual size was checked | An oversized response could consume excessive memory before checksum rejection | Header-first streaming enforces byte ceilings for JSON, executables, notices and gzip data. Tests cover oversized declared lengths, chunked bodies and responses without Content-Length. |
| Download completion needed coverage beyond response headers | A streaming implementation must not leave the UI downloading indefinitely when the body stalls | One deadline covers all HTTP phases. A loopback server sends headers and partial content, then stalls; the test confirms cancellation and a timeout error. |
| Zero-byte executable metadata was accepted | A malformed release could offer an empty executable | Release parsing rejects empty executable assets, with a regression test. |

The earlier app-worker review also fixed manual self-replacement in debug
builds and added tests for automatic/manual decisions, restart readiness,
errors and panics. Release builds retain automatic and manual installation.

The locking design follows Rust's [Windows sharing-mode API](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.share_mode).
The HTTP design follows Microsoft's [completion options](https://learn.microsoft.com/en-us/uwp/api/windows.web.http.httpcompletionoption)
and [stream reads](https://learn.microsoft.com/en-us/uwp/api/windows.storage.streams.iinputstream.readasync).

## Executed verification

| Check | Result | Local evidence under `target/verification-20260905` |
|---|---|---|
| Full Rust suite | 223 passed; opt-in tests run separately | `rust-checks.log` |
| Formatting and Clippy | Passed, warnings denied | Local command results |
| Final installer lifecycle, both GUID and production identities | Passed; install, repair, upgrade, running-app guards, relocation/downgrade rejection, updater, uninstall; default folder tested with GUID identity | `installer-after-review.log` |
| Live network tests | All four passed against real GitHub endpoints | `live-after-review.log` |
| Live executable replacement | Published app and notices verified, installed and executed in an isolated child | `live-after-review.log` |
| GUI installer and uninstaller | Welcome, destination, tasks, ready, installation, completion and removal passed | `gui-installer.log`, `gui-normal-launch-install.log`, `gui-normal-uninstall.log`, `screenshots/installer-*.png`, `screenshots/uninstall-*.png` |
| GUI manual update | Check now → Available → Update → Downloading → Restart → published GUI v0.1.0 | `screenshots/app-*.png` |
| GUI automatic update | Startup installed the real release; command-bar restart opened v0.1.0 | `gui-auto-update.stderr.log`, `screenshots/app-automatic-*.png` |
| GUI update integrity and registration | Exe/notices matched published SHA-256; rollback was v0.0.0; registered version became v0.1.0 | `gui-live-verification.json` |
| Memory and latency | Warm scan 48.97 ms; peak scan memory 40.3 MiB; idle GUI 126.8 MiB, within 1 s / 96 MiB / 160 MiB gates | `performance.log` |
| Release assets | Exe, gzip, notices and setup passed size/version/consistency checks | `artifacts.log` |
| Workflow and PowerShell validation | Passed | `release-checks.log` |
| Release dry-run tests | Both success and simulated failure preserved original Cargo files, without real git/GitHub writes | `release-checks.log` |

The GUI update fixture was built from a separate copy of the current source
as v0.0.0, with a private settings directory. Only the version and settings
location differed. It contacted the real published v0.1.0 release through
the unchanged updater code. Both test installations were removed and the
user's original configuration remained byte-for-byte unchanged.

The first GUI installer launch through the computer-use launcher reported
success but its registration was not visible to the shell. That result was
not counted as a registration pass. A second GUI run through normal Windows
process launch created and retained the expected key, passed automatic
updating and metadata verification, then removed the key on uninstall. The
evidence suggests a launcher-context difference; its exact cause is unproven.

## Security and remaining limits

The raw dependency audit is retained in `dependency-audit.json`. It reports
`RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` for `quick-xml`. Neither quick-xml
version appears in `cargo tree --target x86_64-pc-windows-msvc`; these are
non-Windows dependencies already excluded by CI's documented audit policy.
The audit also reports maintenance warnings for
[paste](https://rustsec.org/advisories/RUSTSEC-2024-0436) and
[ttf-parser](https://rustsec.org/advisories/RUSTSEC-2026-0192), which remain
transitive dependencies of the UI stack. These warnings were not suppressed
or described as resolved vulnerabilities.

The app and installer remain unsigned. Download integrity trusts GitHub's
release metadata, not an independent publisher certificate. Older published
clients do not use the new update lock. Close other copies during migration.
Filesystem failures and abrupt power loss remain operating-system failure
cases; a rollback copy is recovery support, not a guarantee against hardware
failure. High contrast and 200% scaling were not exercised in this session;
the visible GUI checks used the current desktop scaling.

No additional blocking defect was identified in the reviewed paths after
the fixes and tests above. This is scoped verification, not a claim that all
possible future regressions or security issues have been eliminated.
