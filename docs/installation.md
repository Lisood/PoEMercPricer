# Windows installation

Version 0.2.0 adds the per-user Windows installer. Existing v0.1.0 portable
copies can update through the app or migrate using setup.

## Install and use

Run `poemercpricer-setup-windows-x64.exe`. Setup installs for the current
Windows account at `%LOCALAPPDATA%\Programs\PoEMercPricer`. The folder stays
in place when the download is deleted. Open the app from the Start menu; a
desktop shortcut is optional. Launching at the end of setup is opt-in and is
always suppressed for silent installs.

The dark wizard uses the app's existing gold icon, Segoe UI, native controls
and DPI scaling. Windows high-contrast mode disables custom styling. There
is no webview, extra runtime, service, background installer or network request
during setup. Windows 10/11 x64 and x64-compatible Windows are accepted.

An English Windows OCR pack is still needed for screen scans. Setup does not
install language packs or GPU drivers; see the README requirements. Clipboard
Warrant text does not require OCR.

Portable and installed copies share the app's existing config directory under
`%APPDATA%\PoEMercPricer`; setup never edits it. To migrate, close the portable
copy, install, and use the Start menu shortcut. Check that your settings are
present before deleting the old executable and shortcut yourself. The
installer does not move or delete a download it did not create.

## Updates, repair and removal

The updater still downloads `poemercpricer-windows-x64.exe.gz` or the raw
`poemercpricer-windows-x64.exe` from the same GitHub repository. The setup asset
has a different fixed name and is never used for self-replacement. Old
portable versions continue using the same release protocol.

Updates verify the executable and any supplied third-party notices before
replacing the app. Notices are stored as `THIRD_PARTY_NOTICES-X.Y.Z.html` so
each file clearly identifies its version. Older listings without notices
still work. A failed notice download or checksum aborts before the swap. A
failure to save already-verified notices after replacement is logged; fetch
the matching notices from the release page if needed. The version in Windows
Installed apps is refreshed after replacement, even before you choose Restart.
Metadata errors never undo or misreport a successful binary update.

The shortcut target stays the same. The previous executable is kept as
`poemercpricer-previous.exe`. Nothing restarts automatically. To roll back,
close the app and replace `poemercpricer.exe` with that backup. Disable update
checks temporarily if you need to remain on the older version; Windows' listed
version reflects the last successful update until setup or another update
refreshes it.

Run the same setup again to repair missing app files. Run a newer setup to
upgrade. Setup compares the actual installed executable's Windows version,
so an old installer cannot overwrite a copy that already updated itself.
It also refuses a different destination while this account has an existing
registration, avoiding an orphaned second installation. Close running copies
before installing, repairing or uninstalling. Setup and Uninstall both check
the mutex held throughout the new app's GUI lifetime, including its update.

Remove the app through Windows Settings > Apps > Installed apps. Uninstall
removes installed files, shortcuts, the registration, rollback exe, stale
updater downloads and versioned notice files. It leaves the shared settings
directory, debug captures and unrelated files in the install folder. There
is deliberately no recursive deletion of the installation directory.

Silent install and uninstall:

```powershell
Start-Process .\poemercpricer-setup-windows-x64.exe -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait -PassThru
Start-Process "$env:LOCALAPPDATA\Programs\PoEMercPricer\unins000.exe" -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait -PassThru
```

These are executable arguments; when scripting GUI-subsystem programs in
PowerShell use `Start-Process -Wait -PassThru` and inspect the exit code.
`/DIR="path"` chooses a writable destination on first install; `/TASKS=desktopicon`
requests the desktop shortcut. Silent setup never opens the overlay. Logs
are available with `/LOG="path"`. A running app or downgrade attempt causes a
nonzero exit code, without a forced close or reboot.

## Build and test

```powershell
cargo build --release --locked
cargo install cargo-about --version 0.9.2 --locked --features cli
cargo about generate --locked --fail about.hbs -o THIRD_PARTY_NOTICES.html
pwsh scripts/build-installer.ps1
pwsh scripts/test-installer.ps1
```

Output: `target/installer/poemercpricer-setup-windows-x64.exe`. The compiler
bootstrap pins Inno Setup 6.7.3 to SHA-256
`9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732`
and verifies its Pyrsys B.V. signature. Build-time downloads stay in ignored
`target/installer-tools`. No dependency is added to the app.

The smoke script uses a fresh GUID for its app identity, registry key and
shortcuts, and a temporary folder containing spaces and a non-ASCII character.
It installs the actual payload, inspects shortcuts and notices, runs only
`--version`, repairs, upgrades an older versioned PE, swaps an installed child through the real updater over
loopback, rejects a newer-PE downgrade and a conflicting destination, and
uninstalls while retaining a user sentinel file. It also verifies both running
app guards and cleanup of rollback/download/notice files. Logs and timing
results go to `target/installer-tests`. It also checks a fresh installation at
the default per-user Programs location and removes it. Final asynchronous
uninstaller cleanup is polled with a five-second bound.

`-Setup target/installer/poemercpricer-setup-windows-x64.exe` tests a completed
production package and its real registry integration. It refuses to run if a
production registration or shortcut already exists.
The release workflow runs this against the final package before publishing.

Normal Rust tests cover registration isolation, Unicode paths, checksums,
notice failures, gzip fallback, portable compatibility and shared installer
identifiers. CI's `installer` job builds and exercises the setup on each PR
and main push. Live network checks remain separate so GitHub outages do not
make the offline unit suite flaky:

```powershell
cargo test --locked --test update_live -- --ignored --nocapture --test-threads=1
cargo test --locked --test update_install live_published_app_installs_and_runs_its_version_command -- --ignored --exact --nocapture
```

`update-live.yml` runs these weekly and on manual dispatch. The second test
downloads the real published exe and notices, verifies and installs them into
an isolated child, then runs the downloaded app's `--version`. It never opens
the overlay. A future-version installer asset cannot be tested against GitHub
until one is published; its local lifecycle is exercised now.

### Local verification, 2026-09-05

After removing the signing service integration, the unsigned package passed
both the GUID-isolated lifecycle and the final production-package lifecycle.
These exercised install, repair, upgrade, running-app protection, relocation
and downgrade rejection, an installed self-update, and uninstall with user
data retained. The GUID run also verified the default permanent location.
The final-package run verified that updating refreshes Windows' registered
version and that the replaced executable still runs `--version`.

All 223 normal Rust tests passed, including app worker decisions, update-state
text, debug-build protection and recheck scheduling. The four app worker tests
also passed in release mode and now run in CI's installer job. The four live
network tests and the live published-app
replacement test passed separately. The latter used the actual published
v0.1.0 payload and notices; local newer-release tests use a simulated release.
Formatting, Clippy, workflow validation, PowerShell parsing, release dry-run
restoration tests and all four release-asset checks also passed. No release,
tag or push was made. The user subsequently authorized GUI testing explicitly; the results below
include actual installation, update-button and restart interaction.

## GUI verification, 2026-09-05

The real installer was stepped through welcome, destination, shortcuts, ready,
install and completion. Text and artwork rendered without clipping at the
current desktop scaling. Desktop and launch options were unchecked by default.
GUI uninstall completed and preserved the existing user configuration.

For live update checks, a separate copy of the current source was built as
v0.0.0 with a private configuration directory. Only those two fixture settings
changed; the updater and app worker code were unchanged. A registered test
installation then ran this older fixture against the real GitHub release.
Manual Check now, Update to 0.1.0, automatic startup updating, Settings restart
and command-bar restart all succeeded. The restarted GUI displayed v0.1.0.
The executable and notices matched GitHub's published hashes, the previous
executable was v0.0.0, and Windows' registered version became v0.1.0.

The computer-use launcher's first installer run wrote files and reported
success, but its registration was not visible from the shell. Repeating the
GUI install through a normal Windows process launch produced and retained the
expected registration; that run passed automatic updating and metadata checks.
The launcher-context discrepancy is retained in the evidence, not counted as
a successful registration test. Both test installations were uninstalled.

Screenshots, transcripts and hash evidence are under
`target/verification-20260905`. They are local and ignored by git. High-contrast
and 200% scaling remain separate visual checks; this session did not change
Windows display or accessibility settings. See [the review](installer-updater-review.md).

## Release

The app, installer and uninstaller are unsigned. Only GitHub Actions builds
may be published. No signing service, token or signing approval is required.
The updater continues to verify downloads against GitHub's size and SHA-256
metadata before replacing the executable.

`verify-release-assets.ps1` checks all four files and verifies the gzip expands
to the exact release exe. The final installer and its installed updater are
tested before all assets are uploaded to a draft and the release is published.
`release.ps1` rehearses installer build/tests locally and checks all four
published digests. A dry run restores both Cargo files byte for byte even if
a check fails. Publishing requires explicit authorization.

## Research and tradeoffs

Inno Setup was chosen for a small, offline installer that works with the
existing self-updater. Windows documents per-user program storage separately
from roaming settings in [KNOWNFOLDERID](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid).
Inno's [lowest privilege mode](https://jrsoftware.org/ishelp/topic_setup_privilegesrequired.htm)
supports installation without elevation. Installing under Program Files would
make the current in-place updater require administrator access.

MSIX owns its package directory and update lifecycle, which would require a
different deployment model; see Microsoft's [packaged desktop app behavior](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes).
Using an MSI repair database alongside an independently replaced executable
would also need additional coordination. Inno fits this single-executable
app without adding another updater service.

The [native dark wizard](https://jrsoftware.org/ishelp/topic_setup_wizardstyle.htm)
adds roughly 220 KB according to Inno's documentation and retains a
high-contrast fallback. The existing icon is reused; there are no animations
or new raster assets. [Wizard image sizing](https://jrsoftware.org/ishelp/topic_setup_wizardimagefile.htm)
and native layout keep artwork sharp at higher DPI.
[AppMutex](https://jrsoftware.org/ishelp/topic_setup_appmutex.htm) protects both
installation and removal even when a loaded exe is renamed.
