[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Image,
    [string]$League = 'Allflame',
    [switch]$Live,
    # One group per skill; anonymous POSTs reject this as too complex, so use it offline or with a session.
    [switch]$EverySkill
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repo 'target\release\poemercpricer.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw 'Build target\release\poemercpricer.exe before verifying a trade query.'
}

$resolvedImage = (Resolve-Path -LiteralPath $Image).Path
$extra = if ($EverySkill) { '--every-skill' } else { $null }
$output = & $exe dump-trade-query $resolvedImage $League $extra
if ($LASTEXITCODE -ne 0 -or $output.Count -lt 2) {
    throw 'PoEMercPricer did not produce a trade query.'
}

$url = $output[-1]
$encodedQuery = ($url -split '\?q=', 2)[1]
if ([string]::IsNullOrWhiteSpace($encodedQuery)) {
    throw 'The generated official trade URL contains no query.'
}
$query = [System.Uri]::UnescapeDataString($encodedQuery)
$parsed = $query | ConvertFrom-Json
if (-not $parsed.query.stats[0].filters[0].id) {
    throw 'The generated trade query has no skill filter.'
}

if (-not $Live) {
    $query
    Write-Host "Offline validation only: skill filter $($parsed.query.stats[0].filters[0].id). Add -Live for one controlled search POST."
    exit 0
}

# Deliberately one request: no polling, retry, result lookup, or hidden fallback.
$headers = @{
    'User-Agent' = 'PoEMercPricer/0.1.0 (github.com/Lisood/PoEMercPricer)'
}
$endpoint = "https://www.pathofexile.com/api/trade/search/$([Uri]::EscapeDataString($League))"
$response = Invoke-WebRequest -UseBasicParsing -Method Post -Uri $endpoint `
    -Headers $headers -ContentType 'application/json' -Body $query
$payload = $response.Content | ConvertFrom-Json
Write-Host "Total listings: $($payload.total)"
if (-not $payload.total) {
    Write-Warning 'The search returned 0 listings; check the selected skill, support tiers and ilvl.'
}

[pscustomobject]@{
    HttpStatus = [int]$response.StatusCode
    SearchId = $payload.id
    Total = $payload.total
    ResultIdsReturned = @($payload.result).Count
    RateLimitPolicy = $response.Headers['X-Rate-Limit-Policy']
    RateLimitState = $response.Headers['X-Rate-Limit-Ip-State']
}
