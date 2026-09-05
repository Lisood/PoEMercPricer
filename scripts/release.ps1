#Requires -Version 7
<#
.SYNOPSIS
    Cuts a PoEMercPricer release: bump, verify, commit, tag, push, watch the build.
.DESCRIPTION
    Implements CONTRIBUTING.md > Releases / docs/updater.md section 4: runs the
    checks CI runs, commits "Release X.Y.Z", tags vX.Y.Z, pushes both, and waits
    for release.yml to publish poemercpricer-windows-x64.exe with a sha256 digest.
.EXAMPLE
    pwsh scripts/release.ps1 -Version 0.1.0 -DryRun
    pwsh scripts/release.ps1 -Version 0.1.0
#>
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
function Step([string]$name) { Write-Host "==> $name" }

$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version '$Version' is not X.Y.Z. Fix: pass a bare semver, e.g. -Version 0.1.0." }
$branch = git branch --show-current
if ($branch -ne 'main') { throw "Current branch is '$branch', not main. Fix: git checkout main." }
gh auth status *>$null
if ($LASTEXITCODE -ne 0) { throw 'gh is not authenticated. Fix: gh auth login.' }
if ((gh repo view --json isPrivate --jq .isPrivate) -eq 'true') {
    $private = 'Repo is private, so the updater 404s for every user. Fix: make it public first (gh repo edit --visibility public --accept-visibility-change-consequences).'
    if ($DryRun) { Write-Warning $private } else { throw $private }
}
if (git tag --list "v$Version") { throw "Tag v$Version already exists locally. Fix: pick a new version, or delete the stray tag with git tag -d v$Version." }
if (git ls-remote --tags origin "refs/tags/v$Version") { throw "Tag v$Version already exists on origin. Fix: pick a new version; this one was already released." }
$cargoPath = Join-Path $repo 'Cargo.toml'
$versionLinePattern = [regex]'(?m)^version = "[^"]+"'
$cargoText = Get-Content -LiteralPath $cargoPath -Raw
$originalMatch = $versionLinePattern.Match($cargoText)
if (-not $originalMatch.Success) { throw 'Cargo.toml has no top-level version line. Fix: add a [package] version field first.' }
$originalVersionLine = $originalMatch.Value
$currentVersion = ($originalVersionLine -replace '^version = "|"$', '')
if ([version]$Version -le [version]$currentVersion) { throw "Version $Version is not greater than Cargo.toml's $currentVersion. Fix: bump higher (patch/minor per CONTRIBUTING.md)." }
$treeStatus = git status --porcelain
if ($treeStatus -and $DryRun) { Write-Warning 'Working tree is not clean; rehearsing anyway (a real run would fail here).' }
elseif ($treeStatus) { throw "Working tree is not clean. Fix: commit or stash your changes first.`n$treeStatus" }

# Nothing sensitive may reach the tag: the release notes quote commits and the
# tree is public. Same patterns as the "No secrets" step in ci.yml.
Step '0/10 secret scan of tracked files'
$leaks = git grep -I -n -E 'POESESSID[=: ]*[0-9a-f]{32}|ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22}|AKIA[0-9A-Z]{16}|BEGIN [A-Z ]*PRIVATE KEY' -- . ':!Cargo.lock'
if ($leaks) { throw "Secret-looking text is tracked. Fix: remove it and rewrite the commit before releasing.`n$($leaks -join "`n")" }
$stray = git ls-files | Select-String -Pattern '^debug/|\.log$|\.env$|config\.json$|last-capture'
if ($stray) { throw "Local files are tracked. Fix: git rm --cached them and add to .gitignore.`n$($stray -join "`n")" }

# Steps 1-5 run for real even under -DryRun.
$cargoOriginalBytes = [IO.File]::ReadAllBytes($cargoPath)
$lockPath = Join-Path $repo 'Cargo.lock'
$lockOriginalBytes = [IO.File]::ReadAllBytes($lockPath)
try {
Step '1/10 bump Cargo.toml version'
$newContent = $versionLinePattern.Replace($cargoText, "version = `"$Version`"", 1)
[System.IO.File]::WriteAllText($cargoPath, $newContent, [System.Text.UTF8Encoding]::new($false))
Step '2/10 cargo update -p poemercpricer && cargo build --release --locked'
cargo update -p poemercpricer
if ($LASTEXITCODE -ne 0) { throw 'cargo update -p poemercpricer failed.' }
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw 'cargo build --release --locked failed.' }
# Size budget (docs/performance.md, "Binary size and memory"): a regression to
# opt-level 3 or a new codec/backend pushes the exe past this.
$exeBytes = (Get-Item target/release/poemercpricer.exe).Length
if ($exeBytes -gt 11000000) { throw "poemercpricer.exe is $exeBytes bytes, above the 11000000 byte budget" }
Step '3/10 cargo fmt --check'
cargo fmt --check
if ($LASTEXITCODE -ne 0) { throw 'cargo fmt --check failed. Fix: run cargo fmt.' }
Step '4/10 cargo clippy --all-targets --locked -- -D warnings'
cargo clippy --all-targets --locked -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed.' }
Step '5/10 cargo test --locked'
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw 'cargo test failed.' }
Step '5/10 installer build and lifecycle checks'
cargo about generate --locked --fail about.hbs -o THIRD_PARTY_NOTICES.html
if ($LASTEXITCODE -ne 0) { throw 'Notice generation failed. Install cargo-about 0.9.2 with --locked --features cli first.' }
& "$PSScriptRoot/build-installer.ps1"
& "$PSScriptRoot/test-installer.ps1"
if ($DryRun) {
    Step 'dry-run: rehearsing steps 6-10 (nothing below is executed)'
    Write-Host "[dry-run] would run: git commit -am `"Release $Version`""
    Write-Host "[dry-run] would run: git tag v$Version"
    Write-Host "[dry-run] would run: git push --atomic origin main v$Version"
    Write-Host "[dry-run] would run: gh run watch --exit-status <release.yml run for v$Version>"
    Write-Host "[dry-run] would run: gh release view v$Version --json assets --jq '...' (check all four assets have sha256 digests)"
    Write-Host 'Dry run complete. No commit, tag, or push happened.'
    return
}
} finally {
    if ($DryRun) {
        Step 'dry-run: restoring Cargo.toml / Cargo.lock byte for byte, including on failure'
        [IO.File]::WriteAllBytes($cargoPath, $cargoOriginalBytes)
        [IO.File]::WriteAllBytes($lockPath, $lockOriginalBytes)
    }
}

# Steps 6-10 run only for a real release; a failure past here needs a manual undo.
Step '6/10 commit'
git commit -am "Release $Version"
if ($LASTEXITCODE -ne 0) { throw 'git commit failed.' }
$pushed = $false
try {
    Step '7/10 tag'
    git tag "v$Version"
    if ($LASTEXITCODE -ne 0) { throw 'git tag failed.' }
    Step '8/10 push'
    git push --atomic origin main "v$Version"
    if ($LASTEXITCODE -ne 0) { throw 'git push failed; nothing was pushed.' }
    $pushed = $true
    Step '9/10 watch the release workflow'
    $runId = $null
    $deadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $deadline -and -not $runId) {
        $runs = @(gh run list --workflow release.yml --branch "v$Version" --limit 1 --json databaseId | ConvertFrom-Json)
        if ($runs.Count -gt 0) { $runId = $runs[0].databaseId } else { Start-Sleep -Seconds 3 }
    }
    if (-not $runId) { throw "No release.yml run appeared for v$Version within 60s." }
    gh run watch $runId --exit-status
    if ($LASTEXITCODE -ne 0) { throw "release.yml run $runId failed. Fix: gh run view $runId --log-failed" }
    Step '10/10 verify the uploaded assets and their digests'
    # GitHub computes the digest asynchronously, often 30-90s after upload, so
    # poll instead of checking once.
    $exeLine = $null
    $assets = $null
    $pollDeadline = (Get-Date).AddMinutes(3)
    while ((Get-Date) -lt $pollDeadline) {
        $assets = gh release view "v$Version" --json assets --jq '.assets[] | .name + " " + (.size|tostring) + " " + (.digest // "no-digest")'
        if ($LASTEXITCODE -ne 0) { throw "gh release view v$Version failed." }
        $required = @('poemercpricer-windows-x64.exe', 'poemercpricer-windows-x64.exe.gz', 'poemercpricer-setup-windows-x64.exe', 'THIRD_PARTY_NOTICES.html')
        $complete = $true
        foreach ($name in $required) {
            $pattern = '^' + [regex]::Escape($name) + ' [1-9][0-9]* sha256:[a-fA-F0-9]{64}$'
            if (-not (@($assets) | Where-Object { $_ -match $pattern })) { $complete = $false }
        }
        if ($complete) { $exeLine = @($assets) | Where-Object { $_ -match '^poemercpricer-windows-x64\.exe ' }; break }
        Start-Sleep -Seconds 10
    }
    if (-not $exeLine) { throw "A required release asset is missing or has no sha256 digest yet: $assets" }
    Write-Host $exeLine
    Write-Host (@($assets) | Where-Object { $_ -match '^poemercpricer-setup-windows-x64\.exe ' })
    $gzLine = @($assets) | Where-Object { $_ -match '^poemercpricer-windows-x64\.exe\.gz ' }
    if (-not $gzLine) { throw "poemercpricer-windows-x64.exe.gz is missing (the updater would fall back to the exe, but the release is incomplete): $assets" }
    Write-Host $gzLine
    $noticesLine = @($assets) | Where-Object { $_ -match '^THIRD_PARTY_NOTICES\.html ' }
    if (-not $noticesLine) { throw "THIRD_PARTY_NOTICES.html is missing (the release ships crate code without its MIT/Apache attribution): $assets" }
    Write-Host $noticesLine
    $immutable = gh api "repos/Lisood/PoEMercPricer/releases/tags/v$Version" --jq .immutable
    if ($immutable -ne 'true') { throw "v$Version is not immutable; enable immutable releases under Settings > General > Releases" }
    Write-Host "Release: https://github.com/Lisood/PoEMercPricer/releases/tag/v$Version"
}
catch {
    Write-Host ''
    if (-not $pushed) {
        Write-Host 'Failed after the commit. To undo:'
        Write-Host "  git tag -d v$Version"
        Write-Host '  git reset --hard HEAD~1'
    }
    else {
        Write-Host "Pushed already. If the workflow failed: gh run view <id> --log-failed, then delete the draft release if one exists (gh release delete v$Version --yes) and the tag (git push origin :refs/tags/v$Version), fix, and rerun. A published release is immutable and cannot be deleted; fix forward with a new patch version."
    }
    throw
}
