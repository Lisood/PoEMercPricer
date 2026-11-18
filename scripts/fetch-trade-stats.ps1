#Requires -Version 7
<#
.SYNOPSIS
    Writes the official trade site's mercenary stat ids (skills, supports, tiers).
.EXAMPLE
    pwsh scripts/fetch-trade-stats.ps1 -League Covens -Patch 3.30 -Out assets/trade-stats-3.30.json
#>
[CmdletBinding()]
param(
    [string]$League = 'Covens',
    [string]$Patch = '3.30',
    [string]$Out = 'assets/trade-stats-3.30.json'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'trade-http.ps1')
$repo = Split-Path -Parent $PSScriptRoot

$resp = Invoke-TradeRequest GET "$TradeApi/data/stats" $null 'trade-data-request-limit'
if ($resp.Status -ne 200) { throw "stats endpoint returned HTTP $($resp.Status)" }
$entries = @(($resp.Content | ConvertFrom-Json).result | ForEach-Object { $_.entries } |
    Where-Object { $_.id -like 'mercenary.*' } | ForEach-Object { [ordered]@{ id = $_.id; text = $_.text } })
if ($entries.Count -eq 0) { throw 'no mercenary stat entries in the response' }

$doc = [ordered]@{
    patch   = $Patch
    league  = $League
    source  = 'https://www.pathofexile.com/api/trade/data/stats'
    entries = $entries
}
$path = if ([IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $repo $Out }
# Same bytes as the 3.29 file: PowerShell's own formatting, BOM-less, no trailing newline.
$json = $doc | ConvertTo-Json -Depth 6
$null = $json | ConvertFrom-Json
[IO.File]::WriteAllText($path, $json, [Text.UTF8Encoding]::new($false))
Write-Output "$($entries.Count) mercenary stat entries -> $path"
