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
  of `poemercpricer-windows-x64.exe`, or its `.gz` sibling, only when a check
  finds a newer release and there is something to install.

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

The PowerShell scripts under `scripts/` do make outbound requests, to PoEDB,
the official trade site and API, poe.ninja (currency rates), and the Perandus
Ledger (`xddbsns.com`, build list). They are maintainer tools for refreshing
the bundled snapshots. The app never runs them.

## Local footprint

The binary can: read and write the clipboard, take screenshots, register one
global hotkey, read `%WINDIR%\Fonts\georgia.ttf`, read and write its own
config file under `%APPDATA%\PoEMercPricer`, replace its own executable, and
open your browser or a File Explorer window when you click a button. It
writes nothing to the registry, installs no service, and touches no Path of
Exile file.

It replaces its own executable only after the downloaded bytes match both the
size and the SHA-256 digest GitHub publishes for the asset, and it keeps the
executable it replaced as `poemercpricer-previous.exe`. A failed verification
changes nothing on disk.

It never reads or writes game memory and sends no input to the game, so ToS
and anti-cheat concerns are out of scope for a security report. They are
welcome as normal issues.

## Releases

Release executables are built only by GitHub Actions from a tagged commit and
uploaded with GitHub's SHA-256 digest. A locally built exe embeds the
builder's `C:\Users\<name>\.cargo` paths and is never published. CI and the
release script grep the tracked tree for session cookies, tokens and private
keys before anything is tagged. The full checklist is in `AGENTS.md`.

The exe is unsigned. That is a real gap: a signed binary would let Windows
and antivirus vendors verify the publisher instead of the digest alone.
Certificates cost money this project does not have, and the SHA-256 check
against GitHub's published digest is what stands in for it.
