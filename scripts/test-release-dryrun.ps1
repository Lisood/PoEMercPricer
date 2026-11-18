#Requires -Version 7
# Exercise the real release script in a disposable checkout with command mocks.
# Any attempted commit, tag creation, push or publication fails the test.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) ('poemercpricer-dryrun-' + [guid]::NewGuid())
$originalLocation = Get-Location
try {
    New-Item -ItemType Directory -Path "$fixtureRoot/scripts", "$fixtureRoot/target/release" -Force | Out-Null
    Copy-Item -LiteralPath "$PSScriptRoot/release.ps1" -Destination "$fixtureRoot/scripts/release.ps1"
    [IO.File]::WriteAllText("$fixtureRoot/Cargo.toml", "[package]`r`nname = `"poemercpricer`"`r`nversion = `"0.1.0`"`r`n# existing local edits`r`n")
    [IO.File]::WriteAllText("$fixtureRoot/Cargo.lock", "# original lock with local edits`r`n")
    [IO.File]::WriteAllBytes("$fixtureRoot/target/release/poemercpricer.exe", [byte[]](1, 2, 3))
    $cargoBefore = [Convert]::ToBase64String([IO.File]::ReadAllBytes("$fixtureRoot/Cargo.toml"))
    $lockBefore = [Convert]::ToBase64String([IO.File]::ReadAllBytes("$fixtureRoot/Cargo.lock"))
    Set-Content -LiteralPath "$fixtureRoot/scripts/test-installer.ps1" -Value '# mocked lifecycle'

    function git {
        $global:LASTEXITCODE = 0
        switch ($args[0]) {
            'branch' { return 'main' }
            'status' { return ' M existing-local-file' }
            'tag' { if ($args[1] -eq '--list') { return }; throw 'Dry run attempted to create a tag.' }
            'ls-remote' { return }
            'grep' { return }
            'ls-files' { return }
            default { throw "Dry run attempted an unexpected git operation: $args" }
        }
    }
    function gh {
        $global:LASTEXITCODE = 0
        if ($args[0] -eq 'auth') { return }
        if ($args[0] -eq 'repo' -and $args[1] -eq 'view') { return 'false' }
        throw "Dry run attempted an unexpected GitHub operation: $args"
    }
    function cargo {
        $global:LASTEXITCODE = 0
        if ($args[0] -notin @('update', 'build', 'fmt', 'clippy', 'test', 'about')) { throw "Unexpected cargo operation: $args" }
        $global:cargoCalls++
        if ($args[0] -eq 'update') { Add-Content -LiteralPath "$fixtureRoot/Cargo.lock" -Value '# changed by version bump' }
    }
    # release.ps1 refuses a version without a clean CHANGELOG.md section before it touches anything.
    $changelogCases = @(
        @{ Name = 'missing section'; Text = "# Changelog`n`n## [0.1.0] - 2026-09-04`n`n- First release.`n" },
        @{ Name = 'empty section'; Text = "# Changelog`n`n## [0.2.0] - 2026-09-05`n`n## [0.1.0] - 2026-09-04`n`n- First release.`n" },
        @{ Name = 'TODO in heading'; Text = "# Changelog`n`n## [0.2.0] - TODO(date)`n`n- Something changed.`n" }
    )
    foreach ($case in $changelogCases) {
        [IO.File]::WriteAllText("$fixtureRoot/CHANGELOG.md", $case.Text)
        $global:cargoCalls = 0
        $message = $null
        try { & "$fixtureRoot/scripts/release.ps1" -Version 0.2.0 -DryRun } catch { $message = $_.Exception.Message }
        if ($message -notmatch 'CHANGELOG\.md') { throw "Changelog case '$($case.Name)' was not rejected with the changelog message (got: $message)." }
        if ($global:cargoCalls -ne 0) { throw "Changelog case '$($case.Name)' ran cargo before the changelog check." }
        if ([Convert]::ToBase64String([IO.File]::ReadAllBytes("$fixtureRoot/Cargo.toml")) -ne $cargoBefore) { throw "Changelog case '$($case.Name)' changed Cargo.toml." }
    }
    [IO.File]::WriteAllText("$fixtureRoot/CHANGELOG.md", "# Changelog`n`n## [0.2.0] - 2026-09-05`n`n- Something changed.`n")
    foreach ($fail in @($true, $false)) {
        Set-Content -LiteralPath "$fixtureRoot/scripts/build-installer.ps1" -Value $(if ($fail) { "throw 'simulated installer failure'" } else { '# mocked successful build' })
        $failed = $false
        try { & "$fixtureRoot/scripts/release.ps1" -Version 0.2.0 -DryRun }
        catch {
            if (-not $fail -or $_.Exception.Message -notmatch 'simulated installer failure') { throw }
            $failed = $true
        }
        if ($failed -ne $fail) { throw 'Did not exercise the expected dry-run outcome.' }
        if ([Convert]::ToBase64String([IO.File]::ReadAllBytes("$fixtureRoot/Cargo.toml")) -ne $cargoBefore) { throw 'Dry run lost Cargo.toml bytes.' }
        if ([Convert]::ToBase64String([IO.File]::ReadAllBytes("$fixtureRoot/Cargo.lock")) -ne $lockBefore) { throw 'Dry run lost Cargo.lock bytes.' }
    }
    Write-Host 'Release dry-run success and failure preserve local Cargo edits, bad changelog sections are refused first; no write to GitHub or git history occurred.'
} finally {
    Set-Location -LiteralPath $originalLocation.Path
    $tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    $resolved = [IO.Path]::GetFullPath($fixtureRoot)
    if (-not $resolved.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase) -or (Split-Path $resolved -Leaf) -notmatch '^poemercpricer-dryrun-[a-f0-9-]+$') { throw 'Unsafe fixture cleanup path.' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
