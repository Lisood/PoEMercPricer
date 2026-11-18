#Requires -Version 7
# Dot-sourced by the fetch-* scripts: Invoke-TradeRequest with the trade API's
# rate-limit accounting (copied, reduced, from fetch-warrant-prices.ps1).
# Honours X-Rate-Limit-Ip(-State) at $RateShare of every window, Retry-After on
# 429, and aborts after 3 consecutive 429s. Callers may set $MinIntervalSeconds
# / $RateShare / $LongWindowShare before dot-sourcing.
$script:UserAgent = 'PoEMercPricer/0.1.0 (github.com/Lisood/PoEMercPricer)'
$script:TradeApi = 'https://www.pathofexile.com/api/trade'
if (-not (Test-Path variable:MinIntervalSeconds)) { $MinIntervalSeconds = 2.5 }
if (-not (Test-Path variable:RateShare)) { $RateShare = 0.7 }
if (-not (Test-Path variable:LongWindowShare)) { $LongWindowShare = 0.5 }
$script:Counts = @{ search = 0; fetch = 0; other = 0; r429 = 0 }
$script:LastAt = [datetime]::MinValue
$script:Policies = @{}
$script:Consec429 = 0

function Get-Share([int]$window) { if ($window -ge 3600) { $LongWindowShare } else { $RateShare } }
function Read-Header($r, [string]$name) {
    if ($r.Headers.ContainsKey($name)) { (@($r.Headers[$name]) -join ',') } else { $null }
}

function Invoke-TradeRequest([string]$Method, [string]$Uri, [string]$Body, [string]$Policy, [string]$Kind = 'other') {
    $netRetries = 0
    while ($true) {
        # Self-throttle: min interval, then our own timestamps per known window.
        $gap = ([datetime]::UtcNow - $script:LastAt).TotalSeconds
        if ($gap -lt $MinIntervalSeconds) { Start-Sleep -Milliseconds ([int](($MinIntervalSeconds - $gap) * 1000) + 1) }
        $entry = $script:Policies[$Policy]
        while ($entry) {
            $now = [datetime]::UtcNow; $until = $null
            foreach ($rule in $entry.Rules) {
                $allowed = [math]::Max(1, [math]::Floor($rule.Max * (Get-Share $rule.Window)))
                $in = @($entry.Times | Where-Object { ($now - $_).TotalSeconds -lt $rule.Window } | Sort-Object)
                if ($in.Count -ge $allowed) {
                    $c = $in[$in.Count - $allowed].AddSeconds($rule.Window + 0.5)
                    if ($null -eq $until -or $c -gt $until) { $until = $c }
                }
            }
            if ($null -eq $until -or ($until - $now).TotalSeconds -le 0) { break }
            Write-Host ("    rate budget ({0}): sleeping {1:n1}s" -f $Policy, ($until - $now).TotalSeconds)
            Start-Sleep -Milliseconds ([int](($until - $now).TotalSeconds * 1000) + 1)
        }
        $script:LastAt = [datetime]::UtcNow
        $script:Counts[$Kind]++
        try {
            $p = @{ Method = $Method; Uri = $Uri; Headers = @{ 'User-Agent' = $script:UserAgent; Accept = 'application/json' }
                UseBasicParsing = $true; SkipHttpErrorCheck = $true; TimeoutSec = 60; MaximumRetryCount = 0 }
            if ($Method -eq 'POST') { $p.ContentType = 'application/json'; $p.Body = $Body }
            $r = Invoke-WebRequest @p
        }
        catch {
            if (++$netRetries -ge 3) { throw }
            Write-Host "    network error: $($_.Exception.Message); retrying in 30s"; Start-Sleep -Seconds 30; continue
        }
        $status = [int]$r.StatusCode
        $rules = Read-Header $r 'X-Rate-Limit-Ip'; $state = Read-Header $r 'X-Rate-Limit-Ip-State'
        $pol = $Policy
        if (-not $script:Policies.ContainsKey($pol)) { $script:Policies[$pol] = @{ Rules = @(); Times = [System.Collections.Generic.List[datetime]]::new() } }
        $entry = $script:Policies[$pol]; $entry.Times.Add([datetime]::UtcNow)
        if ($rules) {
            $entry.Rules = @($rules -split ',' | ForEach-Object { $q = $_.Trim() -split ':'; @{ Max = [int]$q[0]; Window = [int]$q[1] } })
        }
        $forced = 0.0
        if ($state) {
            $st = @($state -split ',')
            for ($i = 0; $i -lt $st.Count -and $i -lt $entry.Rules.Count; $i++) {
                $hits = [int](($st[$i].Trim() -split ':')[0]); $rule = $entry.Rules[$i]
                $allowed = [math]::Max(1, [math]::Floor($rule.Max * (Get-Share $rule.Window)))
                if ($hits -ge $allowed) { $forced = [math]::Max($forced, [math]::Min($rule.Window, ($rule.Window / [double]$allowed) * ($hits - $allowed + 1))) }
            }
        }
        Write-Host ("{0} #{1}/{2} {3} {4} ip=[{5}] state=[{6}]" -f [datetime]::UtcNow.ToString('HH:mm:ss'), $Kind, $script:Counts[$Kind], $Method, $status, $rules, $state)
        if ($status -eq 429) {
            $script:Counts.r429++
            if (++$script:Consec429 -ge 3) { throw 'aborting: 3 consecutive HTTP 429 responses' }
            $ra = Read-Header $r 'Retry-After'; $sleep = 65
            if ($ra -and [int]::TryParse($ra, [ref]$null) -and [int]$ra -ge 1) { $sleep = [int]$ra }
            Write-Host "    429: sleeping $($sleep + 1)s"; Start-Sleep -Seconds ($sleep + 1); continue
        }
        $script:Consec429 = 0
        if ($forced -gt 0) { Write-Host ("    server state near limit: sleeping {0:n1}s" -f $forced); Start-Sleep -Milliseconds ([int]($forced * 1000)) }
        return @{ Status = $status; Content = $r.Content }
    }
}

# Policy names: search -> 'trade-search-request-limit', fetch -> 'trade-fetch-request-limit',
# data endpoints -> 'trade-data-request-limit'.
function Search-Trade([string]$League, $Query) {
    Invoke-TradeRequest POST "$script:TradeApi/search/$([uri]::EscapeDataString($League))" ($Query | ConvertTo-Json -Depth 10 -Compress) 'trade-search-request-limit' 'search'
}
function Get-TradeListings([string[]]$Ids, [string]$QueryId) {
    Invoke-TradeRequest GET "$script:TradeApi/fetch/$($Ids -join ',')?query=$QueryId" $null 'trade-fetch-request-limit' 'fetch'
}
function Write-JsonFile([string]$Path, $Object, [int]$Depth = 12) {
    $json = $Object | ConvertTo-Json -Depth $Depth
    $null = $json | ConvertFrom-Json
    [System.IO.File]::WriteAllText($Path, $json + "`n", [System.Text.UTF8Encoding]::new($false))
}
