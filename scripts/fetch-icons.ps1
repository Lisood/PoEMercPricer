#Requires -Version 7
param(
  [switch]$RefreshCatalog,
  [string]$Patch = '3.30.0',
  [string]$League = 'Covens',
  [string]$Out = 'assets/catalog-3.30.json',
  # Catalog of the previous patch: entries it has that PoEDB dropped are kept
  # when the trade site still knows the skill (stat) or the build (warrant type).
  # Empty (the default) keeps nothing over.
  [string]$Previous,
  [int]$MinLevel = 83,
  # Reuse `listings` by build name from an earlier catalog instead of searching again.
  [string]$ListingsFrom
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'trade-http.ps1')
$root = Split-Path -Parent $PSScriptRoot
$skillDir = Join-Path $root 'assets\icons\skills'
$supportDir = Join-Path $root 'assets\icons\supports'
$catalogPath = if ([IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $root $Out }
$patchShort = ($Patch -split '\.')[0..1] -join '.'
$tradeStatsPath = Join-Path $root "assets\trade-stats-$patchShort.json"
$headers = @{
  Referer = 'https://poedb.tw/'
  'User-Agent' = 'PoEMercPricer/0.1.0 (+https://github.com/Lisood/PoEMercPricer)'
}

New-Item -ItemType Directory -Force -Path $skillDir, $supportDir | Out-Null

function ConvertTo-Key([string]$Name) {
  $key = $Name.ToLowerInvariant() -replace "['’]", ''
  $key = $key -replace '[^a-z0-9]+', '_'
  $key.Trim('_')
}

function ConvertTo-SupportFamily([string]$Name) {
  $base = $Name -replace '^(Lesser|Greater)\s+', ''
  switch -Regex ($base) {
    '^Return$' { 'return'; break }
    '^Multiple Projectiles$' { 'gmp'; break }
    '^Elemental Damage with Attacks$' { 'edwa'; break }
    '^Faster Attacks$' { 'faster_attacks'; break }
    '^Critical Damage$' { 'crit_damage'; break }
    '^Critical Chance$' { 'crit_chance'; break }
    '^Cooldown Recovery$' { 'cdr'; break }
    '^Area of Effect$|^Increased Area of Effect$' { 'aoe'; break }
    '^More Duration$' { 'more_duration'; break }
    '^Sacred Wisps$' { 'sacred_wisps'; break }
    default { ConvertTo-Key $base }
  }
}

function Get-Page([string]$Url) {
  (Invoke-WebRequest -UseBasicParsing -Headers $headers -TimeoutSec 60 $Url).Content
}

# PoEDB still serves the pre-3.26 swirl art for Return; the live client draws
# the green boomerang gem, so that template is hand-pinned and never refetched.
$pinnedIcons = @('return__returnprojectiles.webp')
function Save-Icon([string]$Url, [string]$Path) {
  if ((Test-Path -LiteralPath $Path) -and -not $RefreshCatalog) { return }
  if ($pinnedIcons -contains (Split-Path -Leaf $Path)) { return }
  # Download beside the target, then swap: a failed download must not leave a truncated icon that later runs skip.
  try { Invoke-WebRequest -UseBasicParsing -Headers $headers -TimeoutSec 60 $Url -OutFile "$Path.part" }
  catch { Remove-Item -LiteralPath "$Path.part" -Force -ErrorAction SilentlyContinue; throw }
  Move-Item -LiteralPath "$Path.part" -Destination $Path -Force
}

$mercHtml = Get-Page 'https://poedb.tw/us/Mercenaries'
$skillPattern = '<img\s+loading="lazy"\s+(?:decoding="async"\s+)?src="(?<url>https://cdn\.poedb\.tw/image/Art/2DArt/(?:Skill|Buff)Icons/[^"]+)"\s+alt="[^"]*"\s+class="size32 MercenarySkill(?:Primary|Secondary|Utility)"\s*/></span>\s*<span[^>]*>(?<name>.*?)</span>'
$skillsByName = [ordered]@{}
foreach ($match in [regex]::Matches($mercHtml, $skillPattern, [Text.RegularExpressions.RegexOptions]::Singleline)) {
  $rawName = $match.Groups['name'].Value -replace '<[^>]+>', ''
  $rawName = $rawName -replace '\s+', ' '
  $name = [Net.WebUtility]::HtmlDecode($rawName.Trim())
  if (-not $name -or $name.StartsWith('[DNT]')) { continue }
  $url = $match.Groups['url'].Value
  $key = ConvertTo-Key $name
  $file = "$key.webp"
  $skillsByName[$name] = [ordered]@{ name = $name; icon = $file; source = $url }
}

# Builds come from the PoEDB Mercenaries index (an Infamous-only build such as
# Warpriest of the Ruckus is listed under its "Infamous_" page). `listings` is
# the official trade search total (securable, ilvl >= $MinLevel) summed over the
# build's ordinary and Infamous warrant types; the Infamous-only build counts
# just its own type.
$buildLinks = [ordered]@{}
foreach ($m in [regex]::Matches($mercHtml, '<a href="(?<page>[A-Za-z_0-9]+)"><img loading="lazy" decoding="async" src="[^"]*InGame/MercenariesofTrarthus/[^"]*"[^>]*/></a></div><div class="flex-grow-1[^"]*">.{0,200}?<a [^>]*>(?<name>[^<]+)</a>', [Text.RegularExpressions.RegexOptions]::Singleline)) {
  $name = [Net.WebUtility]::HtmlDecode($m.Groups['name'].Value.Trim()) -replace '^Infamous ', ''
  $page = $m.Groups['page'].Value
  # The ordinary page wins over an Infamous_ one.
  if (-not $buildLinks.Contains($name) -or -not $page.StartsWith('Infamous_')) { $buildLinks[$name] = $page }
}
if ($buildLinks.Count -eq 0) { throw 'no builds found on the PoEDB Mercenaries page' }

$itemsResp = Invoke-TradeRequest GET "$TradeApi/data/items" $null 'trade-data-request-limit'
$warrantTypes = @{}   # "Mercenary Warrant (Name)" -> type id
foreach ($group in ($itemsResp.Content | ConvertFrom-Json).result) {
  foreach ($e in $group.entries) { if ($e.PSObject.Properties['disc'] -and $e.disc -eq 'mercenary_warrant' -and $e.text) { $warrantTypes[$e.text] = $e.type } }
}
function Get-TypeTotal([string]$typeId) {
  $q = [ordered]@{ query = [ordered]@{ status = @{ option = 'securable' }; type = [ordered]@{ option = $typeId; discriminator = 'mercenary_warrant' }
        stats = @(); filters = @{ misc_filters = @{ filters = @{ ilvl = @{ min = $MinLevel } } } } }; sort = @{ price = 'desc' } }
  $r = Search-Trade $League $q
  if ($r.Status -ne 200) { throw "trade search for $typeId returned HTTP $($r.Status)" }
  [int]($r.Content | ConvertFrom-Json).total
}
function Get-BuildListings([string]$name) {
  $total = 0; $found = $false
  foreach ($text in @("Mercenary Warrant ($name)", "Mercenary Warrant (Infamous $name)")) {
    if ($warrantTypes.ContainsKey($text)) { $found = $true; $total += Get-TypeTotal $warrantTypes[$text] }
  }
  if (-not $found) { Write-Warning "no trade warrant type for build '$name'"; return $null }
  $total
}

$supportPattern = '<img\s+loading="lazy"\s+(?:decoding="async"\s+)?src="(?<url>https://cdn\.poedb\.tw/image/Art/2DItems/Gems/Support/[^"]+)"[^>]*/></div><div class="flex-grow-1 ms-2">(?<name>[^<]+)\s*<span>(?<tier>I{1,3})</span>'
$supportsByIdentity = [ordered]@{}
$builds = [Collections.Generic.List[object]]::new()
foreach ($buildName in $buildLinks.Keys) {
  Start-Sleep -Milliseconds 250
  try {
    $html = Get-Page "https://poedb.tw/us/$($buildLinks[$buildName])"
  } catch {
    # A partial catalog must never replace a full one.
    throw "PoEDB page for build '$buildName' failed to download ($($_.Exception.Message)); the catalog was NOT written"
  }
  # Placeholder pages (Bladereach: "This really shouldn't be here...") have no
  # support table and no trade warrant type; they are not builds.
  $hasType = $warrantTypes.ContainsKey("Mercenary Warrant ($buildName)") -or $warrantTypes.ContainsKey("Mercenary Warrant (Infamous $buildName)")
  if (-not [regex]::IsMatch($html, $supportPattern)) {
    if ($hasType) { throw "PoEDB page for build '$buildName' has a trade warrant type but no parsable support table (markup change?); the catalog was NOT written" }
    Write-Warning "skipping placeholder build $buildName"; continue
  }
  $builds.Add([ordered]@{ name = $buildName; family = ConvertTo-Key $buildName; listings = 0 })
  foreach ($match in [regex]::Matches($html, $supportPattern, [Text.RegularExpressions.RegexOptions]::Singleline)) {
    $name = [Net.WebUtility]::HtmlDecode($match.Groups['name'].Value.Trim())
    $url = $match.Groups['url'].Value
    $family = ConvertTo-SupportFamily $name
    $art = [IO.Path]::GetFileNameWithoutExtension(([uri]$url).AbsolutePath)
    $identity = "$family|$art"
    if ($supportsByIdentity.Contains($identity)) { continue }
    $file = "${family}__$((ConvertTo-Key $art)).webp"
    $supportsByIdentity[$identity] = [ordered]@{
      name = ($name -replace '^(Lesser|Greater)\s+', '')
      canonical = $family
      icon = $file
      source = $url
      skill_tiers_by_skill = [ordered]@{}
    }
  }

  # Record the active skills each support can actually roll on. This lets the
  # scanner disambiguate shared MercSilver/MercGold artwork without guessing.
  $sectionPattern = '<div class="border mb-2">(?<section>.*?)(?=<div class="border mb-2">|\z)'
  foreach ($sectionMatch in [regex]::Matches($html, $sectionPattern, [Text.RegularExpressions.RegexOptions]::Singleline)) {
    $section = $sectionMatch.Groups['section'].Value
    $skillMatch = [regex]::Match($section, '<span class="lc">(?<name>[^<]+)</span>')
    if (-not $skillMatch.Success) { continue }
    $skillName = [Net.WebUtility]::HtmlDecode($skillMatch.Groups['name'].Value.Trim())
    if ($skillName.StartsWith('[DNT]')) { continue }
    foreach ($match in [regex]::Matches($section, $supportPattern, [Text.RegularExpressions.RegexOptions]::Singleline)) {
      $supportName = [Net.WebUtility]::HtmlDecode($match.Groups['name'].Value.Trim())
      $family = ConvertTo-SupportFamily $supportName
      $url = $match.Groups['url'].Value
      $art = [IO.Path]::GetFileNameWithoutExtension(([uri]$url).AbsolutePath)
      $identity = "$family|$art"
      if (-not $supportsByIdentity.Contains($identity)) { continue }
      $entry = $supportsByIdentity[$identity]
      # Some supports using the same generic Mercenary artwork exist only at
      # particular tiers for a skill. Preserve that exact pair; build identity
      # was audited too, but did not eliminate any additional candidates.
      $tier = $match.Groups['tier'].Value.Length
      $tiers = [string]$entry.skill_tiers_by_skill[$skillName]
      if (-not $tiers.Contains([string]$tier)) {
        $entry.skill_tiers_by_skill[$skillName] = -join (@($tiers.ToCharArray()) + [char](48 + $tier) | Sort-Object -Unique)
      }
    }
  }
}

foreach ($skill in $skillsByName.Values) {
  Save-Icon $skill.source (Join-Path $skillDir $skill.icon)
}
foreach ($support in $supportsByIdentity.Values) {
  Save-Icon $support.source (Join-Path $supportDir $support.icon)
}

if ($skillsByName.Count -eq 0 -or $supportsByIdentity.Count -eq 0) { throw 'PoEDB scrape found no skills or no supports; its markup probably changed' }
# Previous-patch entries PoEDB dropped stay while the trade site still has them.
$kept = [Collections.Generic.List[string]]::new()
$previousPath = if (-not $Previous) { $null } elseif ([IO.Path]::IsPathRooted($Previous)) { $Previous } else { Join-Path $root $Previous }
if ($previousPath -and (Test-Path -LiteralPath $previousPath)) {
  $prev = Get-Content -Raw -LiteralPath $previousPath | ConvertFrom-Json
  $tradeSkills = @{}
  foreach ($e in (Get-Content -Raw -LiteralPath $tradeStatsPath | ConvertFrom-Json).entries) { if ($e.id -like 'mercenary.skill_*') { $tradeSkills[$e.text] = $true } }
  foreach ($s in $prev.skills) {
    if (-not $skillsByName.Contains($s.name) -and $tradeSkills.ContainsKey($s.name)) { $skillsByName[$s.name] = [ordered]@{ name = $s.name; icon = $s.icon }; $kept.Add("skill $($s.name)") }
  }
  foreach ($b in $prev.builds) {
    if (-not @($builds | Where-Object { $_.name -eq $b.name }) -and ($warrantTypes.ContainsKey("Mercenary Warrant ($($b.name))") -or $warrantTypes.ContainsKey("Mercenary Warrant (Infamous $($b.name))"))) {
      $builds.Add([ordered]@{ name = $b.name; family = $b.family; listings = 0 }); $kept.Add("build $($b.name)")
    }
  }
}
$reuse = @{}
if ($ListingsFrom) { foreach ($b in (Get-Content -Raw -LiteralPath $(if ([IO.Path]::IsPathRooted($ListingsFrom)) { $ListingsFrom } else { Join-Path $root $ListingsFrom }) | ConvertFrom-Json).builds) { $reuse[$b.name] = [int]$b.listings } }
foreach ($b in $builds) { $b.listings = if ($reuse.ContainsKey($b.name)) { $reuse[$b.name] } else { [int](Get-BuildListings $b.name) } }
$builds = @($builds | Sort-Object { -$_.listings }, { $_.name })

$catalog = [ordered]@{
  patch = $Patch
  league = $League
  generated_at = [datetime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'")
  market_source = "https://www.pathofexile.com/api/trade/search (status securable, ilvl >= $MinLevel, per warrant type total)"
  game_data_source = 'https://poedb.tw/us/Mercenaries'
  builds = $builds
  skills = @($skillsByName.Values | ForEach-Object {
    [ordered]@{ name = $_.name; icon = $_.icon }
  })
  supports = @($supportsByIdentity.Values | ForEach-Object {
    [ordered]@{
      name = $_.name
      canonical = $_.canonical
      icon = $_.icon
      # One compact record per skill; e.g. `Holy Flame Totem|123` means all
      # three visible tiers are possible. One index rather than a separate
      # skills list and tier map to keep in step.
      skill_tiers = @($_.skill_tiers_by_skill.GetEnumerator() | Sort-Object Key | ForEach-Object {
        "$($_.Key)|$($_.Value)"
      })
    }
  })
}
$catalogJson = $catalog | ConvertTo-Json -Depth 8
# serde_json rejects a BOM at byte zero when Rust embeds this file, so write
# BOM-less UTF-8 explicitly.
[IO.File]::WriteAllText($catalogPath, $catalogJson, [Text.UTF8Encoding]::new($false))

Write-Output "$Patch catalog: $($builds.Count) builds, $($skillsByName.Count) skills, $($supportsByIdentity.Count) support arts"
Write-Output "Catalog: $catalogPath"
if ($kept.Count) { Write-Output "Kept from $Previous (gone from PoEDB, still on trade): $($kept -join '; ')" }
Write-Output "Trade calls: $($script:Counts | ConvertTo-Json -Compress)"
