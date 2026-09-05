# Security policy

## Supported versions

Only the latest release gets security fixes. The app updates itself, so an
older build is one restart away from being current.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting: the Security tab, then
Advisories, then Report a vulnerability. Do not open a public issue for a
security bug. Include the app version from `poemercpricer --version`, your
Windows version, and the steps you used.

## Network footprint

The app makes exactly two kinds of outbound request, both for updates, both
HTTPS:

- `GET https://api.github.com/repos/Lisood/PoEMercPricer/releases/latest`, once
  at startup and once every 6 hours while the window stays open.
- A download from `github.com` (redirected to `objects.githubusercontent.com`)
  of `poemercpricer-windows-x64.exe`, or its `.gz` sibling, and the release's
  `THIRD_PARTY_NOTICES.html` when present, only when installing a newer release.

Both carry a `PoEMercPricer/<version>` user agent and an
`Accept: application/vnd.github+json` header. No machine name, account,
league, hardware detail, or scan result is sent, and no request body exists.
There is no telemetry, crash reporting or analytics of any kind, and no
contact with the Path of Exile trade site or API from the app itself: the
Search official trade button builds a URL locally and hands it to your
browser.

Clearing "Check for updates at startup and every 6 hours" in Settings, or
passing `--no-updates`, stops both automatic requests. After that the app
makes no outbound connection unless you press Check now yourself. Debug
builds never check.

The installer itself works offline. Its build-time compiler bootstrap downloads
Inno Setup 6.7.3 from the official GitHub repository and verifies the pinned
SHA-256 and Pyrsys B.V. Authenticode signature before running it.

The PowerShell scripts under `scripts/` do make outbound requests, to PoEDB,
the official trade site and API, poe.ninja (currency rates), and the Perandus
Ledger (`xddbsns.com`, build list). They are maintainer tools for refreshing
the bundled snapshots. The app never runs them.

## Local footprint

The binary can: read and write the clipboard, take screenshots, register one
global hotkey, read `%WINDIR%\Fonts\georgia.ttf`, read and write its own
config file under `%APPDATA%\PoEMercPricer`, replace its own executable, and
open your browser or a File Explorer window when you click a button. It
installs no service and touches no Path of Exile file. Setup registers a per-user
uninstaller under `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\PoEMercPricer_is1`.
After a successful update, the app updates that entry's `DisplayVersion` only
when its executable matches the registered `InstallLocation`. A portable copy
never creates a registration or changes another copy's version.

Setup defaults to `%LOCALAPPDATA%\Programs\PoEMercPricer`. It creates shortcuts
only for the current user. It does not add a service, scheduled task, startup
entry, PATH entry, or firewall rule. Uninstall deletes the files it installed
and reserved updater files; it leaves settings, captures and unrelated files.
The running app holds a named mutex so Setup and Uninstall can refuse to change
an active installation even after the updater has renamed its image.

It replaces its own executable only after the downloaded bytes match both the
size and the SHA-256 digest GitHub publishes for the asset, and it keeps the
executable it replaced as `poemercpricer-previous.exe`. A failed verification
changes nothing on disk.

HTTP bodies are streamed with byte limits before verification, including
responses without Content-Length. A single request deadline also covers body
reads. An exclusive file handle serializes updates in the same folder and is
deleted on close, including process termination. Existing older releases do
not participate in that lock.

Notice downloads are also checked against their own GitHub size and digest
before the executable changes. Verified notices are saved under a versioned
filename after replacement. A failure to write notices or registry metadata
is logged without misreporting a completed binary update as failed; the
release page retains the notices for that version.

It never reads or writes game memory and sends no input to the game, so ToS
and anti-cheat concerns are out of scope for a security report. They are
welcome as normal issues.

## Releases

Release executables are built only by GitHub Actions from a tagged commit and
uploaded with GitHub's SHA-256 digest. A locally built exe embeds the
builder's `C:\Users\<name>\.cargo` paths and is never published. CI and the
release script grep the tracked tree for session cookies, tokens and private
keys before anything is tagged. The full checklist is in `AGENTS.md`.

The app, installer and uninstaller are unsigned. The updater checks size and
SHA-256 against GitHub release metadata before replacing an executable; it
does not require a publisher certificate. This verifies the download against
GitHub's metadata, not an independent publisher identity.

The final installer is silently installed in an isolated test folder, and its
installed updater is exercised before a draft release is created. All four
release assets must pass the size, version and gzip consistency checks. See
`docs/installation.md` for the build and local lifecycle tests.
