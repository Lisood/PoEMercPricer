param(
  [string]$OutputPath,
  [switch]$SummaryOnly
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$catalog = Get-Content -Raw (Join-Path $root 'assets\catalog-3.29.json') | ConvertFrom-Json
$headers = @{
  Referer = 'https://poedb.tw/'
  'User-Agent' = 'PoEMercPricer/0.3 (+https://github.com/Lisood/PoEMercPricer)'
}

$supportPattern = '<img\s+loading="lazy"\s+src="(?<url>https://cdn\.poedb\.tw/image/Art/2DItems/Gems/Support/[^"]+)"[^>]*/></div><div class="flex-grow-1 ms-2">(?<name>[^<]+)\s*<span>(?<tier>I{1,3})</span>'
$sectionPattern = '<div class="border mb-2">(?<section>.*?)(?=<div class="border mb-2">|\z)'
$rows = [Collections.Generic.List[object]]::new()
$auditedBuilds = [Collections.Generic.List[string]]::new()
$failedBuilds = [Collections.Generic.List[string]]::new()

foreach ($build in $catalog.builds) {
  $pageName = $build.name -replace ' ', '_'
  try {
    $html = (Invoke-WebRequest -UseBasicParsing -Headers $headers "https://poedb.tw/us/$pageName").Content
  } catch {
    if ($build.name -eq 'Warpriest of the Ruckus') {
      $pageName = 'Infamous_Warpriest_of_the_Ruckus'
      $html = (Invoke-WebRequest -UseBasicParsing -Headers $headers "https://poedb.tw/us/$pageName").Content
    } else {
      $failedBuilds.Add($build.name)
      Write-Warning "No PoEDB page for $($build.name): $($_.Exception.Message)"
      continue
    }
  }
  $auditedBuilds.Add($build.name)
  foreach ($sectionMatch in [regex]::Matches($html, $sectionPattern, [Text.RegularExpressions.RegexOptions]::Singleline)) {
    $section = $sectionMatch.Groups['section'].Value
    $skillMatch = [regex]::Match($section, '<span class="lc">(?<name>[^<]+)</span>')
    if (-not $skillMatch.Success) { continue }
    $skill = [Net.WebUtility]::HtmlDecode($skillMatch.Groups['name'].Value.Trim())
    foreach ($supportMatch in [regex]::Matches($section, $supportPattern, [Text.RegularExpressions.RegexOptions]::Singleline)) {
      $displayName = [Net.WebUtility]::HtmlDecode($supportMatch.Groups['name'].Value.Trim())
      $baseName = $displayName -replace '^(Lesser|Greater)\s+', ''
      $art = [IO.Path]::GetFileNameWithoutExtension(([uri]$supportMatch.Groups['url'].Value).AbsolutePath)
      $rows.Add([pscustomobject]@{
        build = $build.name
        skill = $skill
        tier = $supportMatch.Groups['tier'].Value.Length
        support = $baseName
        art = $art.ToLowerInvariant()
      })
    }
  }
}

function Get-ContextSummary([object[]]$Rows, [string[]]$Fields) {
  $deduped = @($Rows | Sort-Object ($Fields + 'support') -Unique)
  $groups = @($deduped | Group-Object {
      $row = $_
      ($Fields | ForEach-Object { $_ + '=' + $row.$_ }) -join '|'
    })
  $ambiguous = @($groups | Where-Object { @($_.Group.support | Sort-Object -Unique).Count -gt 1 })
  [ordered]@{
    candidate_rows = $deduped.Count
    contexts = $groups.Count
    exact_contexts = $groups.Count - $ambiguous.Count
    ambiguous_contexts = $ambiguous.Count
    exact_candidate_rows = $deduped.Count - @($ambiguous.Group).Count
    ambiguous_candidate_rows = @($ambiguous.Group).Count
  }
}

function Get-BuildContextGain([object[]]$Rows) {
  $deduped = @($Rows | Sort-Object art, build, skill, tier, support -Unique)
  $skillOnly = @{}
  foreach ($group in @($deduped | Group-Object { "art=$($_.art)|skill=$($_.skill)" })) {
    $skillOnly[$group.Name] = @($group.Group.support | Sort-Object -Unique)
  }
  $global = @{}
  foreach ($group in @($deduped | Group-Object { "art=$($_.art)|skill=$($_.skill)|tier=$($_.tier)" })) {
    $global[$group.Name] = @($group.Group.support | Sort-Object -Unique)
  }
  $localGroups = @($deduped | Group-Object { "art=$($_.art)|build=$($_.build)|skill=$($_.skill)|tier=$($_.tier)" })
  $skillOnlyExact = 0
  $globalExact = 0
  $localExact = 0
  $resolvedByBuild = 0
  foreach ($group in $localGroups) {
    $sample = $group.Group[0]
    $globalKey = "art=$($sample.art)|skill=$($sample.skill)|tier=$($sample.tier)"
    $skillOnlyKey = "art=$($sample.art)|skill=$($sample.skill)"
    $skillOnlyCount = @($skillOnly[$skillOnlyKey]).Count
    $globalCount = @($global[$globalKey]).Count
    $localCount = @($group.Group.support | Sort-Object -Unique).Count
    if ($skillOnlyCount -eq 1) { $skillOnlyExact++ }
    if ($globalCount -eq 1) { $globalExact++ }
    if ($localCount -eq 1) {
      $localExact++
      if ($globalCount -gt 1) { $resolvedByBuild++ }
    }
  }
  [ordered]@{
    evaluated_build_skill_art_tier_contexts = $localGroups.Count
    exact_with_skill_only = $skillOnlyExact
    exact_with_skill_and_tier = $globalExact
    additional_contexts_resolved_by_tier = $globalExact - $skillOnlyExact
    exact_with_build_skill_and_tier = $localExact
    additional_contexts_resolved_by_build = $resolvedByBuild
    unresolved_with_build_skill_and_tier = $localGroups.Count - $localExact
  }
}

$remainingAmbiguities = @(
  $rows |
    Sort-Object art, build, skill, tier, support -Unique |
    Group-Object { "art=$($_.art)|build=$($_.build)|skill=$($_.skill)|tier=$($_.tier)" } |
    Where-Object { @($_.Group.support | Sort-Object -Unique).Count -gt 1 } |
    ForEach-Object {
      [pscustomobject][ordered]@{
        build = $_.Group[0].build
        skill = $_.Group[0].skill
        tier = $_.Group[0].tier
        art = $_.Group[0].art
        supports = @($_.Group.support | Sort-Object -Unique)
      }
    }
)

$report = [ordered]@{
  source = 'https://poedb.tw/us/Mercenaries and its 36 build pages'
  audited_at = [DateTime]::UtcNow.ToString('o')
  requested_build_pages = $catalog.builds.Count
  audited_build_pages = $auditedBuilds.Count
  failed_build_pages = @($failedBuilds)
  rows = $rows.Count
  art_and_skill = Get-ContextSummary $rows @('art', 'skill')
  art_skill_and_tier = Get-ContextSummary $rows @('art', 'skill', 'tier')
  art_build_and_skill = Get-ContextSummary $rows @('art', 'build', 'skill')
  art_build_skill_and_tier = Get-ContextSummary $rows @('art', 'build', 'skill', 'tier')
  build_context_gain = Get-BuildContextGain $rows
  holy_flame_totem_silver = @($rows | Where-Object {
      $_.build -eq 'Flaming Charlatan' -and
      $_.skill -eq 'Holy Flame Totem' -and
      $_.art -eq 'mercsilverstrintsupportgem'
    } | Sort-Object tier, support -Unique)
  ambiguity_sets = @(
    $remainingAmbiguities |
      Group-Object { ($_.supports | Sort-Object) -join ' / ' } |
      Sort-Object Count -Descending |
      ForEach-Object { [ordered]@{ supports = $_.Name; contexts = $_.Count } }
  )
  remaining_ambiguities = $remainingAmbiguities
}

if ($SummaryOnly) {
  $report.Remove('remaining_ambiguities')
}

$json = $report | ConvertTo-Json -Depth 8
if ($OutputPath) {
  $json | Set-Content -LiteralPath $OutputPath -Encoding utf8
} else {
  $json
}
