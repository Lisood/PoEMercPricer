[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Image,
    [string]$League = 'Allflame',
    [switch]$Live
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repo 'target\release\poemercpricer.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw 'Build target\release\poemercpricer.exe before verifying a trade query.'
}

$resolvedImage = (Resolve-Path -LiteralPath $Image).Path
$output = & $exe dump-trade-query $resolvedImage $League
if ($LASTEXITCODE -ne 0 -or $output.Count -lt 2) {
    throw 'PoEMercPricer did not produce a trade query.'
}

$url = $output[-1]
$encodedQuery = ($url -split '\?q=', 2)[1]
if ([string]::IsNullOrWhiteSpace($encodedQuery)) {
    throw 'The generated official trade URL contains no query.'
}
$query = [System.Uri]::UnescapeDataString($encodedQuery)

if (-not $Live) {
    $query
    Write-Host 'Offline validation only. Add -Live for one controlled search POST.'
    exit 0
}

# Deliberately one request: no polling, retry, result lookup, or hidden fallback.
$headers = @{
    'User-Agent' = 'PoEMercPricer/0.2.0 (github.com/Lisood/PoEMercPricer)'
}
$endpoint = "https://www.pathofexile.com/api/trade/search/$([Uri]::EscapeDataString($League))"
$response = Invoke-WebRequest -UseBasicParsing -Method Post -Uri $endpoint `
    -Headers $headers -ContentType 'application/json' -Body $query
$payload = $response.Content | ConvertFrom-Json

[pscustomobject]@{
    HttpStatus = [int]$response.StatusCode
    SearchId = $payload.id
    Total = $payload.total
    ResultIdsReturned = @($payload.result).Count
    RateLimitPolicy = $response.Headers['X-Rate-Limit-Policy']
    RateLimitState = $response.Headers['X-Rate-Limit-Ip-State']
}
