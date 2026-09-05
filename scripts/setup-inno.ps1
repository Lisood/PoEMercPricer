#Requires -Version 7
# Installs a pinned, verified compiler into the ignored build tools directory.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$toolRoot = Join-Path (Split-Path $PSScriptRoot -Parent) 'target/installer-tools'
$compiler = Join-Path $toolRoot 'inno/ISCC.exe'
if (Test-Path -LiteralPath $compiler) {
    # ISCC's PE resource is 0.0.0.0; the pinned installer, rather than that
    # resource, establishes the compiler version. The script checks Ver too.
    return $compiler
}
New-Item -ItemType Directory -Path $toolRoot -Force | Out-Null
$download = Join-Path $toolRoot 'innosetup-6.7.3.exe'
$expected = '9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732'
if (-not (Test-Path -LiteralPath $download)) {
    Invoke-WebRequest 'https://github.com/jrsoftware/issrc/releases/download/is-6_7_3/innosetup-6.7.3.exe' -OutFile $download
}
if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ne $expected) { throw 'Inno Setup download checksum mismatch.' }
$signature = Get-AuthenticodeSignature -LiteralPath $download
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=Pyrsys B\.V\.') { throw 'Inno Setup download signature is not valid for Pyrsys B.V.' }
$destination = Join-Path $toolRoot 'inno'
$process = Start-Process -FilePath $download -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', '/NOICONS', '/TASKS=', ('/DIR="{0}"' -f $destination)) -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $compiler)) { throw "Inno Setup compiler installation failed: $($process.ExitCode)." }
return $compiler
