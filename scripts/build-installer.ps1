#Requires -Version 7
param(
    [string]$Payload = 'target/release/poemercpricer.exe',
    [string]$Notices = 'THIRD_PARTY_NOTICES.html',
    [string]$OutputDirectory = 'target/installer',
    [string]$TestIdentity
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = Split-Path $PSScriptRoot -Parent
Push-Location -LiteralPath $repo
try {
    $compiler = & "$PSScriptRoot/setup-inno.ps1"
    $metadata = cargo metadata --format-version 1 --no-deps --locked | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }
    $version = ($metadata.packages | Where-Object name -eq 'poemercpricer').version
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'The installer needs a stable X.Y.Z version.' }
    $exe = (Resolve-Path -LiteralPath $Payload).Path
    $noticesFile = (Resolve-Path -LiteralPath $Notices).Path
    if ((Get-Item -LiteralPath $noticesFile).Length -lt 1000) { throw 'Third-party notices are missing or incomplete.' }
    $file = Get-Item -LiteralPath $exe
    if ($file.VersionInfo.ProductVersion -ne $version) { throw "Payload version differs from Cargo.toml ($version). Rebuild first." }
    if ($file.Length -gt 11000000) { throw 'App exceeds the 11,000,000 byte size budget.' }
    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    $outputPath = (Resolve-Path -LiteralPath $OutputDirectory).Path
    $arguments = @('/Qp', "/DAppVersion=$version", "/DPayloadPath=$exe", "/DNoticesPath=$noticesFile", "/DOutputPath=$outputPath")
    if ($TestIdentity) {
        if ($TestIdentity -notmatch '^PoEMercPricer\.Test\.[a-f0-9-]+$') { throw 'Invalid isolated test identity.' }
        $arguments += @("/DAppIdentity=$TestIdentity", "/DAppTitle=$TestIdentity")
    }
    $arguments += (Join-Path $repo 'installer/PoEMercPricer.iss')
    & $compiler @arguments
    $compileExit = $LASTEXITCODE
    if ($compileExit -ne 0) { throw "Inno Setup compilation failed ($compileExit)." }
    $setup = Join-Path $outputPath 'poemercpricer-setup-windows-x64.exe'
    $size = (Get-Item -LiteralPath $setup).Length
    if ($size -gt 8000000) { throw "Installer exceeds the 8,000,000 byte budget ($size)." }
    Write-Host "Installer: $setup ($size bytes; unsigned)"
} finally { Pop-Location }
