#Requires -Version 7
param(
    [string]$Payload = 'target/release/poemercpricer.exe',
    [string]$Notices = 'THIRD_PARTY_NOTICES.html',
    [string]$Setup
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = Split-Path $PSScriptRoot -Parent
$identity = 'PoEMercPricer.Test.' + [guid]::NewGuid().ToString()
$appIdentity = if ($Setup) { 'PoEMercPricer' } else { $identity }
$testRoot = Join-Path ([IO.Path]::GetTempPath()) $identity
$installDir = Join-Path $testRoot 'permanent home 文'
$key = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\${appIdentity}_is1"
$startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) $appIdentity
$desktop = Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) "$appIdentity.lnk"
$output = Join-Path $repo "target/installer-tests/$identity"
$installer = Join-Path $output 'poemercpricer-setup-windows-x64.exe'
if ($Setup) { $installer = (Resolve-Path -LiteralPath $Setup).Path }
$exe = Join-Path $installDir 'poemercpricer.exe'
$uninstaller = Join-Path $installDir 'unins000.exe'
$metrics = [Collections.Generic.List[object]]::new()
$ownsInstallation = $false

function Assert([bool]$condition, [string]$message) { if (-not $condition) { throw $message } }
function Read-Version([string]$file) {
    # PowerShell does not wait for GUI-subsystem binaries invoked with &.
    $info = [Diagnostics.ProcessStartInfo]::new($file)
    $info.ArgumentList.Add('--version')
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $child = [Diagnostics.Process]::Start($info)
    $stdoutTask = $child.StandardOutput.ReadToEndAsync()
    $stderrTask = $child.StandardError.ReadToEndAsync()
    if (-not $child.WaitForExit(10000)) { $child.Kill($true); throw 'Version command timed out.' }
    $text = $stdoutTask.GetAwaiter().GetResult()
    $errorText = $stderrTask.GetAwaiter().GetResult()
    Assert ($child.ExitCode -eq 0) "Version command failed: $errorText"
    $child.Dispose()
    return $text
}
function Write-VersionFixture([string]$version) {
    $fixtureSource = Join-Path $testRoot 'version-fixture.cs'
    Set-Content -LiteralPath $fixtureSource -Value ('[assembly:System.Reflection.AssemblyFileVersion("' + $version + '")] class Fixture { static void Main() {} }')
    $csc = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
    & $csc /nologo /target:winexe "/out:$exe" $fixtureSource
    Assert ($LASTEXITCODE -eq 0) 'Cannot build version fixture.'
}
function Run-Setup([string]$name, [string]$file, [string[]]$extra = @(), [bool]$success = $true) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$testRoot/$name.log`"") + $extra
    $process = Start-Process -FilePath $file -ArgumentList $arguments -WindowStyle Hidden -PassThru
    $processHandle = $process.Handle
    $peak = 0L
    while (-not $process.WaitForExit(100)) {
        $process.Refresh()
        if (-not $process.HasExited) { $peak = [Math]::Max($peak, $process.PeakWorkingSet64) }
        if ($watch.Elapsed.TotalSeconds -gt 60) { $process.Kill($true); throw "$name timed out." }
    }
    $process.WaitForExit()
    $watch.Stop()
    $metrics.Add([pscustomobject]@{ Case = $name; Seconds = [Math]::Round($watch.Elapsed.TotalSeconds, 2); LauncherPeakBytes = $peak; ExitCode = $process.ExitCode })
    if ($success) { Assert ($process.ExitCode -eq 0) "$name failed ($($process.ExitCode)); see $testRoot/$name.log" }
    else { Assert ($process.ExitCode -ne 0) "$name unexpectedly succeeded." }
}

Push-Location -LiteralPath $repo
try {
    Assert (-not (Test-Path -LiteralPath $key)) 'Test identity already registered.'
    Assert (-not (Test-Path -LiteralPath $testRoot)) 'Test directory already exists.'
    Assert (-not (Test-Path -LiteralPath $startMenu)) 'Test shortcut directory already exists.'
    Assert (-not (Test-Path -LiteralPath $desktop)) 'Test desktop shortcut already exists.'
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    if (-not $Setup) { & "$PSScriptRoot/build-installer.ps1" -Payload $Payload -Notices $Notices -OutputDirectory $output -TestIdentity $identity }
    $ownsInstallation = $true
    $options = @("/DIR=`"$installDir`"", '/TASKS=desktopicon')
    Run-Setup 'install' $installer $options
    $registered = Get-ItemProperty -LiteralPath $key
    Assert ($registered.InstallLocation.TrimEnd('\') -eq $installDir) 'Incorrect registered install location.'
    Assert ($registered.DisplayVersion -eq (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion) 'Incorrect registered version.'
    Assert (Test-Path -LiteralPath $uninstaller) 'Uninstaller missing.'
    Assert ((Get-FileHash -LiteralPath $exe).Hash -eq (Get-FileHash -LiteralPath $Payload).Hash) 'Installed payload differs.'
    $installedNotices = Join-Path $installDir "THIRD_PARTY_NOTICES-$($registered.DisplayVersion).html"
    Assert ((Get-FileHash -LiteralPath $installedNotices).Hash -eq (Get-FileHash -LiteralPath $Notices).Hash) 'Notices differ.'
    foreach ($link in @((Join-Path $startMenu "$appIdentity.lnk"), $desktop)) {
        Assert (Test-Path -LiteralPath $link) "Missing shortcut: $link"
        # WScript.Shell's legacy TargetPath getter loses characters outside the
        # system codepage. Check the Unicode strings stored in the shell link.
        $linkText = [Text.Encoding]::Unicode.GetString([IO.File]::ReadAllBytes($link))
        Assert ($linkText.Contains($exe)) "Shortcut does not contain the Unicode executable path: $link"
        Assert ($linkText.Contains($installDir)) 'Shortcut working directory is missing.'
    }
    $versionOutput = Read-Version $exe
    Assert ($versionOutput -match 'poemercpricer \d+\.\d+\.\d+') 'Installed --version failed.'
    $sentinel = Join-Path $installDir 'user-file.txt'
    Set-Content -LiteralPath $sentinel -Value 'keep this file'
    Set-Content -LiteralPath (Join-Path $installDir 'installer-test-marker') -Value $identity

    # The app keeps this object alive even after its image is renamed by updating.
    $running = [Threading.Mutex]::new($false, 'Local\PoEMercPricer.Running')
    try {
        Run-Setup 'running-install-blocked' $installer $options $false
        Run-Setup 'running-uninstall-blocked' $uninstaller @() $false
        Assert (Test-Path -LiteralPath $exe) 'Running-app guard lost the executable.'
    } finally { $running.Dispose() }

    Run-Setup 'relocation-blocked' $installer @("/DIR=`"$testRoot/other`"") $false
    Run-Setup 'repair' $installer $options
    Assert ((Get-FileHash -LiteralPath $exe).Hash -eq (Get-FileHash -LiteralPath $Payload).Hash) 'Repair changed the payload.'
    Write-VersionFixture '0.0.0.0'
    Run-Setup 'upgrade' $installer $options
    Assert ((Get-FileHash -LiteralPath $exe).Hash -eq (Get-FileHash -LiteralPath $Payload).Hash) 'Upgrade did not replace the older executable.'

    # Real updater replaces a child harness in the installed folder with app
    # bytes served over loopback. The restored app is then run only with --version.
    $env:PMP_INSTALLER_TEST_DIR = $installDir
    try {
        cargo test --locked --test update_install installed_app_survives_update -- --ignored --exact
        Assert ($LASTEXITCODE -eq 0) 'Installed updater roundtrip failed.'
    } finally { Remove-Item Env:PMP_INSTALLER_TEST_DIR }
    Assert ((Read-Version $exe) -eq $versionOutput) 'Updated app version differs.'
    if ($Setup) { Assert ((Get-ItemProperty -LiteralPath $key).DisplayVersion -eq '9.9.9') 'Updater did not refresh Windows Installed apps.' }
    Assert (Test-Path -LiteralPath (Join-Path $installDir 'poemercpricer-previous.exe')) 'Update rollback missing.'
    Set-Content -LiteralPath (Join-Path $installDir 'poemercpricer.exe.123.update') -Value 'stale update'

    # A versioned PE fixture reproduces an app updated beyond this setup version.
    Write-VersionFixture '99.0.0.0'
    $newerHash = (Get-FileHash -LiteralPath $exe).Hash
    Run-Setup 'downgrade-blocked' $installer $options $false
    Assert ((Get-FileHash -LiteralPath $exe).Hash -eq $newerHash) 'Older setup overwrote the newer app.'

    Run-Setup 'uninstall' $uninstaller
    Assert (-not (Test-Path -LiteralPath $key)) 'Uninstall registration left behind.'
    Assert (-not (Test-Path -LiteralPath $exe)) 'Uninstall left the executable.'
    Assert (-not (Test-Path -LiteralPath $startMenu)) 'Uninstall left Start menu shortcuts.'
    Assert (-not (Test-Path -LiteralPath $desktop)) 'Uninstall left desktop shortcut.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $installDir 'poemercpricer-previous.exe'))) 'Uninstall left rollback executable.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $installDir 'poemercpricer.exe.123.update'))) 'Uninstall left stale download.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $installDir 'THIRD_PARTY_NOTICES-9.9.9.html'))) 'Uninstall left update notices.'
    Assert ((Get-Content -LiteralPath $sentinel -Raw).Trim() -eq 'keep this file') 'Uninstall removed user data.'
    if (-not $Setup) {
        # Verify the default permanent folder too, with the GUID test identity.
        $installDir = Join-Path $env:LOCALAPPDATA "Programs/$appIdentity"
        Assert (-not (Test-Path -LiteralPath $installDir)) 'Default test folder already exists.'
        $uninstaller = Join-Path $installDir 'unins000.exe'
        Run-Setup 'default-install' $installer @('/TASKS=')
        Assert ((Get-ItemProperty -LiteralPath $key).InstallLocation.TrimEnd('\') -eq $installDir) 'Default location is not per-user Programs.'
        Assert (-not (Test-Path -LiteralPath $desktop)) 'Desktop shortcut should be opt-in.'
        Run-Setup 'default-uninstall' $uninstaller
        Assert (-not (Test-Path -LiteralPath $key)) 'Default install registration remained.'
        # Inno's second-phase helper removes its image after the first exits.
        for ($attempt = 0; $attempt -lt 50 -and (Test-Path -LiteralPath $installDir); $attempt++) { Start-Sleep -Milliseconds 100 }
        Assert (-not (Test-Path -LiteralPath $installDir)) 'Default installation left files behind.'
    }
    $metrics | Format-Table | Out-Host
    $metrics | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'metrics.json')
    Write-Host 'Installer lifecycle passed; no game or overlay window was launched.'
} finally {
    # Cleanup is restricted to this invocation's GUID identity and temp root.
    if ($ownsInstallation -and (Test-Path -LiteralPath $key) -and (Test-Path -LiteralPath $uninstaller)) {
        try { Run-Setup 'cleanup-uninstall' $uninstaller } catch { Write-Warning $_ }
    }
    $tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    $resolvedRoot = [IO.Path]::GetFullPath($testRoot)
    if (-not $resolvedRoot.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase) -or (Split-Path $resolvedRoot -Leaf) -ne $identity) { throw 'Unsafe test cleanup path.' }
    if (Test-Path -LiteralPath $testRoot) {
        New-Item -ItemType Directory -Path $output -Force | Out-Null
        Get-ChildItem -LiteralPath $testRoot -Filter '*.log' | Copy-Item -Destination $output
        if (-not (Test-Path -LiteralPath $key)) {
            for ($retry = 0; $retry -lt 20; $retry++) {
                try { Remove-Item -LiteralPath $testRoot -Recurse -Force; break }
                catch { if ($retry -eq 19) { Write-Warning "Test files retained at $testRoot : $_" }; Start-Sleep -Milliseconds 200 }
            }
        }
    }
    Pop-Location
}
