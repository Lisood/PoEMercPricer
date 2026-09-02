param(
  [switch]$RefreshCatalog
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$skillDir = Join-Path $root 'assets\icons\skills'
$supportDir = Join-Path $root 'assets\icons\supports'
$catalogPath = Join-Path $root 'assets\catalog-3.29.json'
$headers = @{
  Referer = 'https://poedb.tw/'
  'User-Agent' = 'PoEMercPricer/0.3 (+https://github.com/Lisood/PoEMercPricer)'
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
  (Invoke-WebRequest -UseBasicParsing -Headers $headers $Url).Content
}

function Save-Icon([string]$Url, [string]$Path) {
  if ((Test-Path -LiteralPath $Path) -and -not $RefreshCatalog) { return }
  Invoke-WebRequest -UseBasicParsing -Headers $headers $Url -OutFile $Path
}

$mercHtml = Get-Page 'https://poedb.tw/us/Mercenaries'
$skillPattern = '<img\s+loading="lazy"\s+src="(?<url>https://cdn\.poedb\.tw/image/Art/2DArt/SkillIcons/[^"]+)"\s+alt="[^"]*"\s+class="size32 MercenarySkill(?:Primary|Secondary|Utility)"\s*/></span>\s*<span[^>]*>(?<name>.*?)</span>'
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

$index = Get-Page 'https://xddbsns.com/data/allflame/mercenary-builder.json' | ConvertFrom-Json
$builds = @($index.builds | ForEach-Object {
  [ordered]@{ name = $_.build; family = ConvertTo-Key $_.build; listings = $_.listings }
})

$supportPattern = '<img\s+loading="lazy"\s+src="(?<url>https://cdn\.poedb\.tw/image/Art/2DItems/Gems/Support/[^"]+)"[^>]*/></div><div class="flex-grow-1 ms-2">(?<name>[^<]+)\s*<span>(?<tier>I{1,3})</span>'
$supportsByIdentity = [ordered]@{}
foreach ($build in $builds) {
  $pageName = $build.name -replace ' ', '_'
  try {
    $html = Get-Page "https://poedb.tw/us/$pageName"
  } catch {
    # PoEDB exposes this one current build under its infamous-prefixed slug.
    # Keep the public build name/family, but fetch the real support table so
    # regeneration does not silently omit one of the 36 pools.
    if ($build.name -eq 'Warpriest of the Ruckus') {
      $html = Get-Page 'https://poedb.tw/us/Infamous_Warpriest_of_the_Ruckus'
    } else {
      Write-Warning "No PoEDB support page for $($build.name); continuing"
      continue
    }
  }
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

$catalog = [ordered]@{
  patch = '3.29.3'
  league = 'Allflame'
  generated_at = $index.generated_at
  market_source = 'https://xddbsns.com/data/allflame/mercenary-builder.json'
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
      # three visible tiers are possible. This replaces the former skills list
      # rather than maintaining two redundant runtime indexes.
      skill_tiers = @($_.skill_tiers_by_skill.GetEnumerator() | Sort-Object Key | ForEach-Object {
        "$($_.Key)|$($_.Value)"
      })
    }
  })
}
$catalogJson = $catalog | ConvertTo-Json -Depth 8
# Windows PowerShell's `-Encoding utf8` writes a BOM, which serde_json rejects
# at byte zero when this file is embedded by Rust. Emit portable BOM-less UTF-8
# under both Windows PowerShell 5.1 and PowerShell 7.
[IO.File]::WriteAllText($catalogPath, $catalogJson, [Text.UTF8Encoding]::new($false))

Write-Output "3.29 catalog: $($builds.Count) builds, $($skillsByName.Count) skills, $($supportsByIdentity.Count) support arts"
Write-Output "Catalog: $catalogPath"
