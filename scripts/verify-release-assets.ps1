#Requires -Version 7
param([string]$Directory = '.', [string]$InstallerDirectory = 'target/installer')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$exe = Join-Path $Directory 'poemercpricer-windows-x64.exe'
$gzip = Join-Path $Directory 'poemercpricer-windows-x64.exe.gz'
$notices = Join-Path $Directory 'THIRD_PARTY_NOTICES.html'
$setup = Join-Path $InstallerDirectory 'poemercpricer-setup-windows-x64.exe'
foreach ($file in @($exe, $gzip, $notices, $setup)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf) -or (Get-Item -LiteralPath $file).Length -eq 0) { throw "Release asset missing or empty: $file" }
}
if ((Get-Item -LiteralPath $exe).Length -gt 11000000) { throw 'App size budget exceeded.' }
if ((Get-Item -LiteralPath $setup).Length -gt 8000000) { throw 'Installer size budget exceeded.' }
if ((Get-Item -LiteralPath $gzip).Length -ge (Get-Item -LiteralPath $exe).Length) { throw 'Gzip must be smaller than the exe.' }
$inputStream = [IO.File]::OpenRead($gzip)
$gzipStream = [IO.Compression.GZipStream]::new($inputStream, [IO.Compression.CompressionMode]::Decompress)
$hash = [Security.Cryptography.SHA256]::Create()
try {
    $actual = [Convert]::ToHexString($hash.ComputeHash($gzipStream))
    if ($actual -ne (Get-FileHash -LiteralPath $exe).Hash) { throw 'Gzip does not contain the exact release exe.' }
} finally { $hash.Dispose(); $gzipStream.Dispose(); $inputStream.Dispose() }
$setupVersion = [version](Get-Item -LiteralPath $setup).VersionInfo.ProductVersion
$appVersion = [version](Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
if ($setupVersion.ToString(3) -ne $appVersion.ToString(3)) { throw 'Installer and app versions differ.' }
foreach ($file in @($exe, $gzip, $notices, $setup)) {
    Write-Host "$(Split-Path $file -Leaf) $((Get-Item -LiteralPath $file).Length) sha256:$((Get-FileHash -LiteralPath $file).Hash.ToLowerInvariant())"
}
