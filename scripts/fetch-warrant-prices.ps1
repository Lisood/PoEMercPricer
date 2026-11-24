#Requires -Version 7
<#
.SYNOPSIS
    Rate-limited market-data collector for Mercenary Warrants (default patch 3.30).

.DESCRIPTION
    Queries the official trade API once per (family, warrant type, package) and
    samples the ten cheapest instant-buyout ("securable") listings. Packages:
      base        warrant type only (stock floor of the family)
      money       type + the family's money skill
      money_gates type + money skill + the positive gate supports (T3 wanted,
                  highest existing tier otherwise; the tier used is recorded)
    Prices are converted to chaos with poe.ninja's currency overview.

    Rate limits are a hard boundary: every response's X-Rate-Limit-Ip /
    X-Rate-Limit-Ip-State headers are honoured at half of every window, requests
    are at least 2.5 s apart, a 429 sleeps Retry-After (or 65 s), and three
    consecutive 429s abort the run.

.EXAMPLE
    pwsh scripts/fetch-warrant-prices.ps1 -League Covens -Out assets/warrant-prices-3.30.json
    pwsh scripts/fetch-warrant-prices.ps1 -Families kineticist,manyshot -Out $env:TEMP\test.json
#>
[CmdletBinding()]
param(
    [string]$League = 'Covens',
    [string]$Patch = '3.30',
    [string]$Out = 'assets/warrant-prices-3.30.json',
    [string[]]$Families,
    [string[]]$Packages = @('base', 'money', 'money_gates'),
    [double]$MinIntervalSeconds = 2.5,
    [double]$RateShare = 0.5,
    # Share applied to windows of an hour or longer (the 6-hour policy windows);
    # defaults to RateShare. Only raise it deliberately for a resumed sweep.
    [double]$LongWindowShare = -1,
    [int]$SampleSize = 10,
    # Item level floor on every search: sub-83 warrants are a different, much
    # cheaper market and would drag the medians down. 0 disables the filter.
    [int]$MinLevel = 83,
    # Rows are appended here after every search so an aborted run can resume
    # (rows already present are skipped). Defaults next to -Out.
    [string]$Checkpoint,
    # Partner sweep: re-run only rows that have a partner rule (see
    # $partnerRules), scan up to -ScanSize listings per row, keep the ones whose
    # mercenarySkills satisfy the rule, and merge the rows into the existing
    # -Out file instead of rebuilding it.
    [switch]$PartnerSweep,
    # Re-run only the rows selected by -Families/-Packages and merge them into the existing -Out
    # file by row key (a plain run would replace the whole file). -PartnerSweep implies this.
    [switch]$Merge,
    [int]$ScanSize = 30,
    # Rows whose cheapest listings are all bricks scan every id the search
    # returns (the API hands back at most 100) instead of -ScanSize.
    [int]$DeepScanSize = 100,
    # Print the planned searches (rows plus price-floor retries) and exit; no trade searches.
    [switch]$PlanOnly
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($LongWindowShare -lt 0) { $LongWindowShare = $RateShare }

$repo = Split-Path -Parent $PSScriptRoot
$patchShort = ($Patch -split '\.')[0..1] -join '.'
. (Join-Path $PSScriptRoot 'trade-http.ps1')   # Invoke-TradeRequest, rate limiting, $TradeApi, User-Agent
$listingStatus = 'securable'
# Repeated 403s mean we are blocked, not that one row is bad: stop, keep the checkpoint, write no asset.
$script:consecutive403 = 0
function Test-Forbidden($response, [string]$what) {
    # Counted across search AND fetch calls; reset only when a whole row completes (see $rows.Add below).
    if ($response.Status -eq 403 -and ++$script:consecutive403 -ge 3) {
        throw "aborting: 3 consecutive HTTP 403 responses (last: $what); the asset was NOT written, the checkpoint is kept"
    }
}
$groupFilterLimit = 6

# Family table: warrant type ids (src/trade.rs warrant_trade_type) and the
# money packages (src/scoring/{kineticist,manyshot,combatant,extra}.rs, docs/market-3.30.md and
# src/scoring/market.rs SCREENS, positive gates ordered by screen points).
# Gates are canonical support ids from assets/catalog-<patch>.json.
function Pkg([string]$skill, [string[]]$gates) {
    [pscustomobject]@{ Skill = $skill; Gates = @($gates) }
}
$familyTable = [ordered]@{
    manyshot          = @{ Types = @('EleBowRangerClones', 'EleBowRangerClonesNoble'); Money = @((Pkg 'Vaal Ice Shot' @('return', 'edwa', 'hypothermia')), (Pkg 'Ice Shot' @('return', 'edwa', 'hypothermia'))) }
    kineticist        = @{ Types = @('MiscScionWandAttacks', 'MiscScionWandAttacksNoble'); Money = @((Pkg 'Kinetic Blast of Clustering' @('return', 'gmp'))) }
    combatant         = @{ Types = @('MeleeAOEStrikeDuelistRangeStrikes', 'MeleeAOEStrikeDuelistRangeStrikesNoble'); Money = @((Pkg 'Frost Blades' @('return', 'chain', 'edwa')), (Pkg 'Wild Strike' @('edwa', 'faster_attacks', 'hypothermia'))) }
    chaplain_of_the_well = @{ Types = @('AbyssalTemplar', $null); Money = @((Pkg 'Abyssal Spray' @('return', 'faster_casting'))) }
    sniper            = @{ Types = @('NonEleBowRangerPhys', 'NonEleBowRangerPhysNoble'); Money = @((Pkg 'Tornado Shot' @('gilded_secondary_shots', 'arrow_nova', 'gmp'))) }
    cruel_mistress    = @{ Types = @('ChaosMinionWitchChaosHit', 'ChaosMinionWitchChaosHitNoble'); Money = @((Pkg 'Soulrend of Reaping' @('gmp', 'return'))) }
    thunderquiver     = @{ Types = @('EleBowRangerLightning', $null); Money = @((Pkg 'Lightning Arrow' @('return', 'gmp'))) }
    flamequiver       = @{ Types = @('EleBowRangerFire', 'EleBowRangerFireNoble'); Money = @((Pkg 'Artillery Ballista' @('gilded_totemic_onslaught', 'multiple_totems', 'aoe', 'fire_penetration'))) }
    toxicologist      = @{ Types = @('NonEleBowRangerChaos', 'NonEleBowRangerChaosNoble'); Money = @((Pkg 'Scourge Arrow of Menace' @('gilded_additional_pods', 'gmp', 'mirage_archer', 'physical_as_extra_chaos'))) }
    blade_ambusher    = @{ Types = @('TrapsMinesShadowAttack', 'TrapsMinesShadowAttackNoble'); Money = @((Pkg 'Bear Trap' @('trigger_radius'))) }
    striker           = @{ Types = @('MeleeStrikesMaraduerPhys', 'MeleeStrikesMaraduerPhysNoble'); Money = @((Pkg 'Leap Slam' @('gilded_frenzy', 'brutality'))) }
    bladebitter       = @{ Types = @('Crit1HShadowPoison', 'Crit1HShadowPoisonNoble'); Money = @((Pkg 'Pestilent Strike' @('faster_attacks', 'chance_to_poison'))) }
    stormhand         = @{ Types = @('ElementalWitchLightning', 'ElementalWitchLightningNoble'); Money = @((Pkg 'Arc' @('chain', 'gilded_chain_distance', 'lucky_lightning_damage', 'faster_casting'))) }
    withertouch       = @{ Types = @('ChaosMinionWitchDot', 'ChaosMinionWitchDotNoble'); Money = @((Pkg 'Scourstorm' @('dot_multiplier', 'cdr', 'swift_affliction'))) }
    mysterious_diver  = @{ Types = @('DivingDuelist', 'DivingDuelistNoble'); Money = @((Pkg 'Frost Blades' @('edwa', 'return', 'hypothermia'))) }
    frosthand         = @{ Types = @('ElementalWitchCold', 'ElementalWitchColdNoble'); Money = @((Pkg 'Ice Nova' @('gilded_freezer_burn', 'cold_penetration', 'ailment_effect'))) }
    storming_zealot   = @{ Types = @('PhysConvertTemplarLightning', 'PhysConvertTemplarLightningNoble'); Money = @((Pkg 'Shockwave Totem of Shocking' @('gilded_astral_totem', 'multiple_totems', 'more_duration', 'aoe'))) }
    bladecaster       = @{ Types = @('Crit1HShadowPhysSpell', 'Crit1HShadowPhysSpellNoble'); Money = @((Pkg 'Seismic Crush' @('crit_damage', 'crit_chance', 'brutality', 'aoe'))) }
    shock_ambusher    = @{ Types = @('TrapsMinesShadowLightning', 'TrapsMinesShadowLightningNoble'); Money = @((Pkg 'Vaal Lightning Trap' @('added_lightning'))) }
    cardinal          = @{ Types = @('AurasMinionsTemplarStaff', 'AurasMinionsTemplarStaffNoble'); Money = @((Pkg 'Consecrated Path' @('gilded_consecration', 'faster_attacks', 'aoe'))) }
    warpriest         = @{ Types = @('AurasMinionsTemplarSmite', 'AurasMinionsTemplarSmiteNoble'); Money = @((Pkg 'Herald of Purity' @('minion_damage', 'pulverise', 'aoe', 'brutality'))) }
    swiftblade        = @{ Types = @('MeleeAOEStrikeDuelistCyclone', 'MeleeAOEStrikeDuelistCycloneNoble'); Money = @((Pkg 'Rallying Cry' @('cdr', 'more_duration'))) }
    smoulderstrike    = @{ Types = @('MeleeStrikesMarauderFire', $null); Money = @((Pkg 'Infernal Cry' @('aoe', 'more_duration'))) }
    sanguimancer      = @{ Types = @('MiscScionPhysDot', 'MiscScionPhysDotNoble'); Money = @((Pkg 'Vaal Reap' @('gilded_searing_agony', 'dot_multiplier', 'cdr'))) }
    earthshaker       = @{ Types = @('MeleeAOEMarauderPhysSlam', 'MeleeAOEMarauderPhysSlamNoble'); Money = @((Pkg 'Ground Slam' @('fortify'))) }
    reanimator        = @{ Types = @('ChaosMinionWitchInstability', 'ChaosMinionWitchInstabilityNoble'); Money = @((Pkg 'Summon Raging Spirit' @('minion_damage'))) }
    bloodletter       = @{ Types = @('PhysicalDuelistBleed', 'PhysicalDuelistBleedNoble'); Money = @((Pkg 'Leap Slam' @('gilded_frenzy', 'faster_attacks'))) }
    fallen_reverend   = @{ Types = @('AurasMinionsTemplarSpectres', 'AurasMinionsTemplarSpectresNoble'); Money = @((Pkg 'Reinforce: Fallen Bishop' @())) }
    bastion           = @{ Types = @('PhysicalDuelistShields', 'PhysicalDuelistShieldsNoble'); Money = @((Pkg 'Prismplate' @())) }
    ripper            = @{ Types = @('MeleeAOEMarauderNonSlam', 'MeleeAOEMarauderNonSlamNoble'); Money = @((Pkg 'Leap Slam' @('gilded_frenzy', 'pulverise', 'brutality'))) }
    eruptor           = @{ Types = @('MeleeAOEMarauderFireSlam', 'MeleeAOEMarauderFireSlamNoble'); Money = @((Pkg 'Flame Link' @('gilded_empowered_link', 'more_duration', 'cdr'))) }
    flamehand         = @{ Types = @('ElementalWitchFire', $null); Money = @((Pkg 'Rolling Magma' @('gilded_area_per_projectile', 'gmp'))) }
    winter_deacon     = @{ Types = @('PhysConvertTemplarCold', $null); Money = @((Pkg 'Earthquake of Winter' @('concentrated_effect', 'freeze_chance'))) }
    frost_ambusher    = @{ Types = @('TrapsMinesShadowCold', $null); Money = @((Pkg 'Ice Trap' @('cold_penetration'))) }
    flaming_charlatan = @{ Types = @('PhysConvertTemplarFire', $null); Money = @((Pkg 'Wave of Conviction of Trarthus' @('added_fire', 'cdr'))) }
    shattersword      = @{ Types = @('PhysicalDuelistSteel', $null); Money = @((Pkg 'Rallying Cry' @('more_duration'))) }
    warpriest_of_the_ruckus = @{ Types = @($null, 'AurasMinionsTemplarSmiteRuckusNoble'); Money = @() }
}

# Partner rules (src/scoring/{kineticist,manyshot,combatant,extra}.rs jackpot
# and brick logic, src/scoring/market.rs `bricks` and support bricks). The
# anonymous query cannot express a second skill, so these are applied to the
# fetched listings' item.mercenarySkills instead. Key: "family|money skill" or
# "family|*". SupportExclude names catalog canonicals that brick the money skill.
$partnerRules = @{
    'kineticist|*'             = @{ Require = @('Greater Kinetic Blast'); Exclude = @('Kinetic Bolt', 'Kinetic Rain of Impact', 'Power Siphon'); SupportExclude = @() }
    'manyshot|*'               = @{ Require = @('Mirror Arrow'); Exclude = @('Icicle Rain'); SupportExclude = @() }
    'combatant|Frost Blades'   = @{ Require = @('Static Strike'); Exclude = @('Wild Strike', 'Spectral Helix of Trarthus', 'Spectral Helix'); SupportExclude = @() }
    'combatant|Wild Strike'    = @{ Require = @('Static Strike'); Exclude = @('Frost Blades', 'Spectral Helix of Trarthus', 'Spectral Helix'); SupportExclude = @() }
    'fallen_reverend|*'        = @{ Require = @('Wrath', 'Zealotry'); Exclude = @('Battlemage''s Cry', 'Frostblink'); SupportExclude = @() }
    'thunderquiver|*'          = @{ Require = @(); Exclude = @('Galvanic Arrow'); SupportExclude = @() }
    'sniper|*'                 = @{ Require = @(); Exclude = @(); SupportExclude = @('brutality') }
}
# "family|package|money skill" rows scanned to -DeepScanSize: their cheapest 30
# are brick variants and the real package starts above them.
$deepScanRows = @('kineticist|money_gates|Kinetic Blast of Clustering', 'manyshot|money_gates|Vaal Ice Shot', 'combatant|money_gates|Frost Blades')
# "family|package|money skill" rows whose cheapest 100 are entirely bricks:
# the search is re-run with a trade price floor (divines, tried in order until a
# listing passes the rule) so the 100-id window starts above the brick wall.
$priceFloorRows = @{
    'kineticist|money_gates|Kinetic Blast of Clustering'    = @(1, 10, 50)
    'manyshot|money_gates|Vaal Ice Shot'                    = @(1, 10, 50)
    'combatant|money_gates|Frost Blades'                    = @(1, 10, 25)
}
# Families whose plain "money" row is filtered by the rule as well.
$moneyRuleFamilies = @('kineticist', 'manyshot', 'combatant', 'fallen_reverend')
function Get-PartnerRule([string]$family, [string]$skill) {
    foreach ($k in @("$family|$skill", "$family|*")) { if ($partnerRules.ContainsKey($k)) { return $partnerRules[$k] } }
    return $null
}
function Format-PartnerRule($rule) {
    $text = "Require $($rule.Require -join ',')/Exclude $($rule.Exclude -join ',')"
    if ($rule.SupportExclude.Count -gt 0) { $text += "/ExcludeSupport $($rule.SupportExclude -join ',')" }
    return $text
}

# Catalog + official stat resolution (mirrors src/trade.rs key/support_key).
$catalog = Get-Content -LiteralPath (Join-Path $repo "assets/catalog-$patchShort.json") -Raw | ConvertFrom-Json
$stats = Get-Content -LiteralPath (Join-Path $repo "assets/trade-stats-$patchShort.json") -Raw | ConvertFrom-Json
$catalogFamilies = @($catalog.builds | ForEach-Object { $_.family })
foreach ($f in $catalogFamilies) {
    if (-not $familyTable.Contains($f)) { throw "catalog family '$f' has no entry in the script's family table" }
}
foreach ($f in $familyTable.Keys) {
    if ($catalogFamilies -notcontains $f) { throw "script family '$f' is not in the catalog" }
}

function Get-Key([string]$value) {
    return (($value.ToCharArray() | Where-Object { [char]::IsAsciiLetterOrDigit($_) }) -join '').ToLowerInvariant()
}
function Get-SupportKey([string]$value) {
    $v = $value.Trim()
    foreach ($prefix in @('Lesser ', 'Greater ')) {
        if ($v.StartsWith($prefix)) { $v = $v.Substring($prefix.Length); break }
    }
    $k = Get-Key $v
    if ($k -eq 'areaofeffect') { return 'increasedareaofeffect' }
    return $k
}
$supportNames = @{}
foreach ($support in $catalog.supports) {
    if (-not $supportNames.ContainsKey($support.canonical)) { $supportNames[$support.canonical] = [System.Collections.Generic.List[string]]::new() }
    $supportNames[$support.canonical].Add($support.name)
}
$skillStats = @{}
$supportStats = [System.Collections.Generic.List[object]]::new()
foreach ($entry in $stats.entries) {
    if ($entry.id.StartsWith('mercenary.skill_')) {
        $k = Get-Key $entry.text
        if (-not $skillStats.ContainsKey($k)) { $skillStats[$k] = [System.Collections.Generic.List[string]]::new() }
        $skillStats[$k].Add($entry.id)
    }
    elseif ($entry.id.StartsWith('mercenary.support_')) {
        if ($entry.text -match '^(.*) \(Tier (\d)\)$') {
            $supportStats.Add([pscustomobject]@{ Id = $entry.id; Key = Get-SupportKey $Matches[1]; Tier = [int]$Matches[2]; Text = $entry.text })
        }
    }
}

# SupportExclude semantics (row field partner_exclude_supports): only the MONEY skill's own mercenarySkills entry
# is checked, and the listing is rejected if any support on it (any tier) is one of the named canonical supports.
function Test-PartnerRule($listing, $rule, [string]$moneySkill) {
    $item = $listing.item
    $skills = @()
    if ($null -ne $item -and $item.PSObject.Properties['mercenarySkills']) { $skills = @($item.mercenarySkills) }
    $names = @($skills | ForEach-Object { Get-Key ([string]$_.name) })
    foreach ($r in $rule.Require) { if ($names -notcontains (Get-Key $r)) { return $false } }
    foreach ($x in $rule.Exclude) { if ($names -contains (Get-Key $x)) { return $false } }
    if ($rule.SupportExclude.Count -gt 0) {
        $wanted = Get-Key $moneySkill
        foreach ($skill in $skills) {
            if ((Get-Key ([string]$skill.name)) -ne $wanted -or -not $skill.PSObject.Properties['supports']) { continue }
            $supportKeys = @($skill.supports | ForEach-Object { Get-SupportKey (([string]$_.name) -replace ' \(Tier \d\)$', '') })
            foreach ($canonical in $rule.SupportExclude) {
                if (-not $supportNames.ContainsKey($canonical)) { continue }
                foreach ($n in $supportNames[$canonical]) { if ($supportKeys -contains (Get-SupportKey $n)) { return $false } }
            }
        }
    }
    return $true
}

function Resolve-SkillId([string]$name) {
    $k = Get-Key $name
    if (-not $skillStats.ContainsKey($k)) { throw "no official trade stat for skill $name" }
    $ids = $skillStats[$k]
    if ($ids.Count -ne 1) { throw "multiple official trade stats for skill $name" }
    return $ids[0]
}

# Returns @{ Id; Tier; Text } for the wanted tier, falling back to the highest
# existing lower tier (recorded, never silent). Throws when nothing resolves.
function Resolve-SupportId([string]$canonical, [int]$wantedTier, [string]$skillName) {
    if (-not $supportNames.ContainsKey($canonical)) { throw "unknown support identity $canonical" }
    $keys = @($supportNames[$canonical] | ForEach-Object { Get-SupportKey $_ })
    for ($tier = $wantedTier; $tier -ge 1; $tier--) {
        $found = @($supportStats | Where-Object { $_.Tier -eq $tier -and $keys -contains $_.Key })
        if ($found.Count -eq 0) { continue }
        if ($found.Count -gt 1) {
            if ($canonical -eq 'gilded_extra_targets') {
                $hash = switch (Get-Key $skillName) {
                    'smite' { '37259' }
                    'vaalsmite' { '37259' }
                    'lightningarrow' { '58471' }
                    'greaterlightningarrow' { '58471' }
                    default { throw "no skill-bound Extra Targets trade stat for $skillName" }
                }
                $found = @($found | Where-Object { $_.Id.EndsWith($hash) })
                if ($found.Count -ne 1) { throw "missing Extra Targets hash $hash" }
            }
            else {
                throw "multiple official trade stats for support $canonical at tier $tier on $skillName"
            }
        }
        return @{ Id = $found[0].Id; Tier = $tier; Text = $found[0].Text }
    }
    throw "no official trade stat for support $canonical at any tier <= $wantedTier"
}


function Get-Median([double[]]$values) {
    if ($values.Count -eq 0) { return 0.0 }
    $s = @($values | Sort-Object)
    $n = $s.Count
    if ($n % 2 -eq 1) { return [double]$s[[int](($n - 1) / 2)] }
    return ([double]$s[$n / 2 - 1] + [double]$s[$n / 2]) / 2.0
}
function Get-Percentile([double[]]$values, [double]$p) {
    if ($values.Count -eq 0) { return 0.0 }
    $s = @($values | Sort-Object)
    $rank = $p * ($s.Count - 1)
    $lo = [int][math]::Floor($rank); $hi = [int][math]::Ceiling($rank)
    if ($lo -eq $hi) { return [double]$s[$lo] }
    return [double]$s[$lo] + ([double]$s[$hi] - [double]$s[$lo]) * ($rank - $lo)
}
function Round3([double]$v) { return [math]::Round($v, 3) }

$selectedFamilies = @(if ($Families) { $Families | ForEach-Object { $_.Trim().ToLowerInvariant() } } else { $catalogFamilies })
foreach ($f in $selectedFamilies) { if (-not $familyTable.Contains($f)) { throw "unknown family '$f'" } }

$skipped = [System.Collections.Generic.List[object]]::new()
$rowFailures = [System.Collections.Generic.List[string]]::new()   # runtime failures; any entry aborts before the asset is written
$work = [System.Collections.Generic.List[object]]::new()
foreach ($family in $selectedFamilies) {
    $def = $familyTable[$family]
    $variants = @()
    if ($def.Types[0]) { $variants += , @{ Infamous = $false; Type = $def.Types[0] } }
    if ($def.Types[1]) { $variants += , @{ Infamous = $true; Type = $def.Types[1] } }
    foreach ($variant in $variants) {
        if ($Packages -contains 'base' -and -not $PartnerSweep) {
            $work.Add(@{ Family = $family; Infamous = $variant.Infamous; Type = $variant.Type; Package = 'base'; Skill = $null; Gates = @(); Filters = @() })
        }
        if (-not $def.Money -or $def.Money.Count -eq 0) {
            foreach ($pkg in @('money', 'money_gates')) {
                if ($Packages -contains $pkg) { $skipped.Add("$family/$(if ($variant.Infamous) {'infamous'} else {'ordinary'})/$pkg`: no money skill in scorer") }
            }
            continue
        }
        for ($m = 0; $m -lt $def.Money.Count; $m++) {
            $money = $def.Money[$m]
            $tag = "$family/$(if ($variant.Infamous) {'infamous'} else {'ordinary'})"
            try { $skillId = Resolve-SkillId $money.Skill }
            catch { $skipped.Add("$tag/money+money_gates [$($money.Skill)]: $($_.Exception.Message)"); continue }
            # The primary money skill carries the plain "money" package; alternates only add money_gates.
            if ($m -eq 0 -and $Packages -contains 'money' -and (-not $PartnerSweep -or $moneyRuleFamilies -contains $family)) {
                $work.Add(@{ Family = $family; Infamous = $variant.Infamous; Type = $variant.Type; Package = 'money'; Skill = $money.Skill; Gates = @(); Filters = @($skillId) })
            }
            if ($Packages -contains 'money_gates' -and (-not $PartnerSweep -or $null -ne (Get-PartnerRule $family $money.Skill))) {
                if ($money.Gates.Count -eq 0) { $skipped.Add("$tag/money_gates [$($money.Skill)]: scorer lists no positive gate supports"); continue }
                $gates = @(); $filters = @($skillId); $failed = $null
                foreach ($canonical in $money.Gates) {
                    if ($filters.Count -ge $groupFilterLimit) { Write-Host "    $tag/money_gates [$($money.Skill)]: dropping gate $canonical (6-filter cap)"; continue }
                    try {
                        $resolved = Resolve-SupportId $canonical 3 $money.Skill
                        if ($resolved.Tier -ne 3) { Write-Host "    $tag/money_gates [$($money.Skill)]: $canonical has no Tier 3 stat, using '$($resolved.Text)'" }
                        $gates += , @{ canonical = $canonical; tier = $resolved.Tier }
                        $filters += $resolved.Id
                    }
                    catch { $failed = "$canonical`: $($_.Exception.Message)"; break }
                }
                if ($failed) { $skipped.Add("$tag/money_gates [$($money.Skill)]: $failed"); continue }
                $work.Add(@{ Family = $family; Infamous = $variant.Infamous; Type = $variant.Type; Package = 'money_gates'; Skill = $money.Skill; Gates = $gates; Filters = $filters })
            }
        }
    }
}
Write-Host ("Planned {0} searches across {1} families; {2} rows skipped up front." -f $work.Count, $selectedFamilies.Count, $skipped.Count)
foreach ($s in $skipped) { Write-Host "  skip: $s" }
if ($Merge -and -not $PartnerSweep) {
    $needFilter = @($work | Where-Object { $_.Package -ne 'base' -and $null -ne (Get-PartnerRule $_.Family $_.Skill) -and ($_.Package -eq 'money_gates' -or $moneyRuleFamilies -contains $_.Family) })
    if ($needFilter.Count) {
        throw ("-Merge would replace partner-filtered rows with unfiltered ones; use -PartnerSweep for: " + (($needFilter | ForEach-Object { "$($_.Family)/$($_.Package)/$($_.Skill)" }) -join ', '))
    }
}
if ($PlanOnly) {
    $floorTries = 0
    foreach ($w in $work) { $k = "$($w.Family)|$($w.Package)|$($w.Skill)"; if ($PartnerSweep -and $priceFloorRows.ContainsKey($k)) { $floorTries += @($priceFloorRows[$k]).Count - 1 } }
    Write-Host ("PlanOnly: {0} rows = {1} searches minimum, up to {2} with every price-floor retry." -f $work.Count, $work.Count, ($work.Count + $floorTries))
    Write-Host ("by package: " + (($work | Group-Object { $_.Package } | ForEach-Object { "$($_.Name)=$($_.Count)" }) -join ' '))
    Write-Host "network requests made: search=$($script:Counts.search) fetch=$($script:Counts.fetch) other=$($script:Counts.other)"
    exit 0
}

$outPath = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $repo $Out }
$outDir = Split-Path -Parent $outPath
# The checkpoint is only valid for the same query plan: hash every planned row plus the sweep knobs.
$planText = (@($work | ForEach-Object {
    $pr = if ($PartnerSweep -and $_.Package -ne 'base') { Get-PartnerRule $_.Family $_.Skill } else { $null }
    $partner = if ($pr) { "req=$($pr.Require -join ',');ex=$($pr.Exclude -join ',');sup=$($pr.SupportExclude -join ',')" } else { '' }
    "$($_.Family)|$($_.Infamous)|$($_.Type)|$($_.Package)|$($_.Skill)|$($_.Filters -join ',')|$partner|floor=$($priceFloorRows["$($_.Family)|$($_.Package)|$($_.Skill)"] -join '/')"
}) -join "`n") +
    "`nscan=$ScanSize/$DeepScanSize deep=$($deepScanRows -join ';') floors=$(($priceFloorRows.Keys | Sort-Object | ForEach-Object { "$_=$($priceFloorRows[$_] -join '/')" }) -join ';')"
$planHash = [BitConverter]::ToString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($planText))).Replace('-', '').ToLowerInvariant()
if ($PartnerSweep -or $Merge) {
    # Validate the snapshot we will merge into before any network call.
    if (-not (Test-Path -LiteralPath $outPath)) { throw "-PartnerSweep/-Merge merges into an existing file, but $outPath does not exist" }
    $snapRaw = Get-Content -LiteralPath $outPath -Raw
    $snap = $snapRaw | ConvertFrom-Json
    $snapGenerated = [regex]::Match($snapRaw, '"generated_at":\s*"([^"]+)"').Groups[1].Value   # ConvertFrom-Json would turn it into a DateTime
    $snapWant = [ordered]@{ league = $League; min_level = $MinLevel; patch = $patchShort }
    foreach ($k in $snapWant.Keys) {
        if (-not $snap.PSObject.Properties[$k] -or "$($snap.$k)" -ne "$($snapWant[$k])") {
            throw "snapshot $outPath has $k='$(if ($snap.PSObject.Properties[$k]) { $snap.$k } else { '<missing>' })' but this run has $k='$($snapWant[$k])'; refusing to merge"
        }
    }
}
if (-not (Test-Path -LiteralPath $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }

# poe.ninja retired /api/data/currencyoverview in mid-2026; the stash overview
# keeps the same line shape (currencyTypeName / chaosEquivalent / detailsId)
# and, unlike the exchange overview, reflects stash-tab asks like trade listings.
# Everything below is network: -PlanOnly has already exited above.
$ninjaUri = "https://poe.ninja/poe1/api/economy/stash/current/currency/overview?league=$([uri]::EscapeDataString($League))&type=Currency"
$ninja = Invoke-TradeRequest GET $ninjaUri $null 'poe.ninja'
if ($ninja.Status -ne 200) { throw "poe.ninja returned HTTP $($ninja.Status)" }
$ninjaLines = ($ninja.Content | ConvertFrom-Json).lines
$rateByDetails = @{}
foreach ($line in $ninjaLines) {
    if ($line.PSObject.Properties['detailsId'] -and $line.chaosEquivalent) { $rateByDetails[$line.detailsId] = [double]$line.chaosEquivalent }
}
$divineLine = $ninjaLines | Where-Object { $_.currencyTypeName -eq 'Divine Orb' } | Select-Object -First 1
if ($null -eq $divineLine) { throw 'poe.ninja has no Divine Orb line' }
$chaosPerDivine = [double]$divineLine.chaosEquivalent
if (-not [double]::IsFinite($chaosPerDivine) -or $chaosPerDivine -le 0) { throw "poe.ninja Divine Orb rate is not a positive number: $($divineLine.chaosEquivalent)" }
Write-Host ("chaos_per_divine = {0}" -f $chaosPerDivine)

# Trade listings price in short currency tags ("chrome", "alch"). The official
# static data maps every tag to its display name, which poe.ninja lines carry
# as currencyTypeName; the hard-coded detailsId map below is only the fallback.
$rateByTag = @{}
$static = Invoke-TradeRequest GET "$TradeApi/data/static" $null 'trade-static'
if ($static.Status -eq 200) {
    $rateByName = @{}
    foreach ($line in $ninjaLines) { if ($line.chaosEquivalent) { $rateByName[$line.currencyTypeName] = [double]$line.chaosEquivalent } }
    foreach ($group in @(($static.Content | ConvertFrom-Json).result)) {
        foreach ($entry in @($group.entries)) {
            if ($entry.PSObject.Properties['id'] -and $entry.PSObject.Properties['text'] -and $rateByName.ContainsKey($entry.text)) {
                if (-not $rateByTag.ContainsKey($entry.id)) { $rateByTag[$entry.id] = $rateByName[$entry.text] }
            }
        }
    }
    Write-Host ("{0} trade currency tags mapped to poe.ninja chaos rates" -f $rateByTag.Count)
}
else {
    Write-Host "trade static data returned HTTP $($static.Status); using the built-in tag map only"
}
$currencyTag = @{
    'exalted' = 'exalted-orb'; 'mirror' = 'mirror-of-kalandra'; 'alch' = 'orb-of-alchemy'; 'vaal' = 'vaal-orb'
    'fusing' = 'orb-of-fusing'; 'chrome' = 'chromatic-orb'; 'jewellers' = 'jewellers-orb'; 'chance' = 'orb-of-chance'
    'gcp' = 'gemcutters-prism'; 'regal' = 'regal-orb'; 'annul' = 'orb-of-annulment'; 'regret' = 'orb-of-regret'
    'scour' = 'orb-of-scouring'; 'blessed' = 'blessed-orb'; 'alt' = 'orb-of-alteration'; 'aug' = 'orb-of-augmentation'
    'trans' = 'orb-of-transmutation'; 'silver' = 'silver-coin'; 'wisdom' = 'scroll-of-wisdom'; 'portal' = 'portal-scroll'
    'bauble' = 'glassblowers-bauble'; 'whetstone' = 'blacksmiths-whetstone'; 'scrap' = 'armourers-scrap'
    'divine-orb' = 'divine-orb'; 'exalted-orb' = 'exalted-orb'; 'mirror-of-kalandra' = 'mirror-of-kalandra'
    'awakened-sextant' = 'awakened-sextant'; 'elevated-sextant' = 'elevated-sextant'; 'orb-of-unmaking' = 'orb-of-unmaking'
    'veiled-chaos-orb' = 'veiled-chaos-orb'; 'ancient-orb' = 'ancient-orb'; 'orb-of-binding' = 'orb-of-binding'
    'orb-of-horizons' = 'orb-of-horizons'; 'harbingers-orb' = 'harbingers-orb'; 'engineers-orb' = 'engineers-orb'
    'hinekoras-lock' = 'hinekoras-lock'; 'fracturing-orb' = 'fracturing-orb'; 'tainted-divine-teardrop' = 'tainted-divine-teardrop'
}
function ConvertTo-Chaos([double]$amount, [string]$currency) {
    switch ($currency) {
        'chaos' { return $amount }
        # Both spellings use the run's pinned rate (a merge or resume must not mix in the live one).
        { $_ -in 'divine', 'divine-orb' } { return $amount * $chaosPerDivine }
        default {
            if ($rateByTag.ContainsKey($currency)) { return $amount * $rateByTag[$currency] }
            $details = if ($currencyTag.ContainsKey($currency)) { $currencyTag[$currency] } else { $currency }
            if ($rateByDetails.ContainsKey($details)) { return $amount * $rateByDetails[$details] }
            return $null
        }
    }
}

if ($PartnerSweep -or $Merge) {
    # Rows merge into a snapshot priced at its own rate; convert at that same rate so the file stays consistent.
    $snapRate = [double]$snap.chaos_per_divine
    if (-not [double]::IsFinite($snapRate) -or $snapRate -le 0) { throw "snapshot $outPath has no valid chaos_per_divine" }
    Write-Host ("Merging: using the snapshot's chaos_per_divine = {0} (live poe.ninja rate {1} ignored)" -f $snapRate, $chaosPerDivine)
    $chaosPerDivine = $snapRate
}
$searchUri = "$TradeApi/search/$([uri]::EscapeDataString($League))"
$rows = [System.Collections.Generic.List[object]]::new()

if (-not $Checkpoint) { $Checkpoint = "$outPath.checkpoint.json" }
$done = @{}
if (Test-Path -LiteralPath $Checkpoint) {
    $cp = Get-Content -LiteralPath $Checkpoint -Raw | ConvertFrom-Json
    $want = [ordered]@{ league = $League; min_level = $MinLevel; partner_sweep = [bool]$PartnerSweep; sample_size = $SampleSize; plan_hash = $planHash }
    foreach ($k in $want.Keys) {
        if (-not $cp.PSObject.Properties[$k] -or "$($cp.$k)" -ne "$($want[$k])") {
            throw "checkpoint $Checkpoint was written with $k='$(if ($cp.PSObject.Properties[$k]) { $cp.$k } else { '<missing>' })' but this run has $k='$($want[$k])'; delete the checkpoint to start over"
        }
    }
    # The live rate drifts between runs; prices already in the checkpoint used the stored one, so keep it for the whole run.
    $storedRate = if ($cp.PSObject.Properties['chaos_per_divine']) { [double]$cp.chaos_per_divine } else { 0.0 }
    if (-not [double]::IsFinite($storedRate) -or $storedRate -le 0) { throw "checkpoint $Checkpoint has no valid chaos_per_divine; delete the checkpoint to start over" }
    Write-Host ("Resuming with the checkpoint's chaos_per_divine = {0} (live poe.ninja rate {1} ignored)" -f $storedRate, $chaosPerDivine)
    if (($PartnerSweep -or $Merge) -and [math]::Abs($storedRate - $snapRate) -gt 1e-9) {
        throw "checkpoint rate $storedRate differs from the destination snapshot's chaos_per_divine $snapRate; delete the checkpoint to start over"
    }
    $chaosPerDivine = $storedRate
    foreach ($r in @($cp.rows)) {
        $ordered = [ordered]@{}
        foreach ($prop in $r.PSObject.Properties) { $ordered[$prop.Name] = $prop.Value }
        $rows.Add($ordered)
        $done["$($r.family)|$($r.infamous)|$($r.package)|$($r.money_skill)"] = $true
    }
    Write-Host ("Resuming: {0} rows loaded from {1}" -f $rows.Count, $Checkpoint)
}
$runStart = [datetime]::UtcNow
$index = 0
foreach ($item in $work) {
    $index++
    if ($done.ContainsKey("$($item.Family)|$($item.Infamous)|$($item.Package)|$($item.Skill)")) { continue }
    $tag = "{0}/{1}/{2}{3}" -f $item.Family, $(if ($item.Infamous) { 'infamous' } else { 'ordinary' }), $item.Package, $(if ($item.Skill) { " [$($item.Skill)]" } else { '' })
    $rowKey = "$($item.Family)|$($item.Package)|$($item.Skill)"
    $floors = @($null)
    if ($PartnerSweep -and $priceFloorRows.ContainsKey($rowKey)) { $floors = @($priceFloorRows[$rowKey]) }
    $priceFloorChaos = $null
    $searchFailed = $false
    foreach ($floorDiv in $floors) {
        $query = [ordered]@{
            query = [ordered]@{
                status = @{ option = $listingStatus }
                type = [ordered]@{ option = $item.Type; discriminator = 'mercenary_warrant' }
                stats = @()
            }
            sort = @{ price = 'asc' }
        }
        if ($item.Filters.Count -gt 0) {
            $query.query.stats = @(, [ordered]@{ type = 'mercenary'; filters = @($item.Filters | ForEach-Object { @{ id = $_ } }) })
        }
        $query.query.filters = [ordered]@{}
        if ($MinLevel -gt 0) {
            $query.query.filters.misc_filters = @{ filters = @{ ilvl = @{ min = $MinLevel } } }
        }
        if ($null -ne $floorDiv) {
            # Chaos-equivalent floor: a plain trade filter, so anonymous-safe.
            $priceFloorChaos = [double][math]::Round($floorDiv * $chaosPerDivine, 2)
            $query.query.filters.trade_filters = @{ filters = @{ price = @{ min = $priceFloorChaos } } }
            Write-Host "    price floor $floorDiv div = $priceFloorChaos chaos"
        }
        $body = $query | ConvertTo-Json -Depth 8 -Compress
        Write-Host "[$index/$($work.Count)] $tag"
        $search = Invoke-TradeRequest POST $searchUri $body 'trade-search-request-limit' 'search'
        Test-Forbidden $search "search $tag"
        if ($search.Status -ne 200) {
            $rowFailures.Add("$tag`: search HTTP $($search.Status) $($search.Content)")
            $searchFailed = $true
            break
        }
        $payload = $search.Content | ConvertFrom-Json
        $total = [int]$payload.total
        $rule = Get-PartnerRule $item.Family $item.Skill
        $applyRule = [bool]($PartnerSweep -and $null -ne $rule -and $item.Package -ne 'base')
        $scanLimit = if (-not $applyRule) { $SampleSize } elseif ($deepScanRows -contains "$($item.Family)|$($item.Package)|$($item.Skill)") { $DeepScanSize } else { $ScanSize }
        $ids = @($payload.result | Select-Object -First $scanLimit)

        $prices = @(); $ages = @(); $scanned = 0; $passed = 0; $unpriced = 0; $fetchFailed = $false
        if ($total -gt 0 -and $ids.Count -gt 0) {
            $now = [datetime]::UtcNow
            for ($offset = 0; $offset -lt $ids.Count -and $prices.Count -lt $SampleSize; $offset += 10) {
                $chunk = @($ids[$offset..([math]::Min($offset + 9, $ids.Count - 1))])
                $fetchUri = "$TradeApi/fetch/$($chunk -join ',')?query=$($payload.id)"
                $fetch = Invoke-TradeRequest GET $fetchUri $null 'trade-fetch-request-limit' 'fetch'
                Test-Forbidden $fetch "fetch $tag"
                if ($fetch.Status -ne 200) {
                    Write-Host "    fetch failed with HTTP $($fetch.Status); stopping this row at scanned=$scanned"
                    $fetchFailed = $true
                    break
                }
                foreach ($listing in @(($fetch.Content | ConvertFrom-Json).result)) {
                    if ($null -eq $listing -or $null -eq $listing.listing) { continue }
                    $scanned++
                    if ($applyRule) {
                        if (-not (Test-PartnerRule $listing $rule $item.Skill)) { continue }
                        $passed++
                    }
                    if ($prices.Count -ge $SampleSize) { continue }
                    $price = $listing.listing.price
                    if ($null -eq $price -or $null -eq $price.amount -or [string]::IsNullOrEmpty($price.currency)) { $unpriced++; continue }
                    $chaos = ConvertTo-Chaos ([double]$price.amount) ([string]$price.currency)
                    if ($null -eq $chaos) { $unpriced++; Write-Host "    unconvertible currency '$($price.currency)' skipped"; continue }
                    $prices += [double]$chaos
                    if ($listing.listing.indexed) {
                        $indexed = [datetime]::Parse($listing.listing.indexed, [cultureinfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::AdjustToUniversal)
                        $ages += [double](($now - $indexed).TotalDays)
                    }
                }
            }
        }
        if ($fetchFailed) { $rowFailures.Add("${tag}: fetch HTTP failure after scanned=$scanned"); $searchFailed = $true; break }
        if ($null -eq $floorDiv -or $passed -gt 0) { break }
        Write-Host "    no listing passed at this floor; trying the next one"
    }
    if ($searchFailed) { continue }
    $fresh = @($ages | Where-Object { $_ -le 7.0 })
    $row = [ordered]@{
        family          = $item.Family
        infamous        = [bool]$item.Infamous
        package         = $item.Package
        money_skill     = $item.Skill
        gates           = @($item.Gates | ForEach-Object { [ordered]@{ canonical = $_.canonical; tier = [int]$_.tier } })
        listings        = $total
        sampled         = $prices.Count
        lowest_chaos    = [double](Round3 $(if ($prices.Count) { ($prices | Measure-Object -Minimum).Minimum } else { 0.0 }))
        median_chaos    = [double](Round3 (Get-Median $prices))
        p75_chaos       = [double](Round3 (Get-Percentile $prices 0.75))
        median_age_days = [double](Round3 (Get-Median $ages))
        fresh_share     = [double](Round3 $(if ($ages.Count) { $fresh.Count / [double]$ages.Count } else { 0.0 }))
        query_id        = [string]$payload.id
        sampled_at      = [datetime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'")
    }
    if ($PartnerSweep) {
        # listings = search total while at least one scanned listing passes the
        # partner rule; 0 when none did (search_total keeps the raw count).
        $row.search_total = $total
        $row.scanned = $scanned
        $row.passed = $(if ($applyRule) { $passed } else { $scanned })
        $row.partner_rule = $(if ($applyRule) { Format-PartnerRule $rule } else { $null })
        # Structured copy of the rule for the app: catalog skill names the warrant must / must not also have.
        if ($applyRule) { $row.partner_require = @($rule.Require); $row.partner_exclude = @($rule.Exclude) }
        if ($applyRule -and $rule.SupportExclude.Count -gt 0) { $row.partner_exclude_supports = @($rule.SupportExclude) }
        if ($applyRule -and $passed -eq 0) { $row.listings = 0 }
        if ($null -ne $priceFloorChaos) { $row.price_floor_chaos = $priceFloorChaos }
    }
    $rows.Add($row)
    $script:consecutive403 = 0
    [System.IO.File]::WriteAllText($Checkpoint, (([ordered]@{ league = $League; min_level = [int]$MinLevel; partner_sweep = [bool]$PartnerSweep; sample_size = $SampleSize; plan_hash = $planHash; chaos_per_divine = $chaosPerDivine; rows = @($rows) } | ConvertTo-Json -Depth 8)), [System.Text.UTF8Encoding]::new($false))
    Write-Host ("    => listings={0} sampled={1} low={2}c med={3}c p75={4}c age={5}d fresh={6}{7}" -f $row.listings, $row.sampled, $row.lowest_chaos, $row.median_chaos, $row.p75_chaos, $row.median_age_days, $row.fresh_share, $(if ($applyRule) { " scanned=$scanned passed=$passed" } else { '' }))
}

if ($rowFailures.Count) {
    throw ("{0} row(s) failed and the asset was NOT written; the checkpoint {1} is kept, rerun to resume:`n  {2}" -f $rowFailures.Count, $Checkpoint, ($rowFailures -join "`n  "))
}
if ($rows.Count -ne $work.Count) { throw "expected $($work.Count) rows but have $($rows.Count); the asset was NOT written" }
$finalRows = $rows
if ($PartnerSweep -or $Merge) {
    if (-not (Test-Path -LiteralPath $outPath)) { throw "-PartnerSweep/-Merge merges into an existing file, but $outPath does not exist" }
    $baseRaw = Get-Content -LiteralPath $outPath -Raw
    # Anything that rewrote the file during this run (a full refresh, or another merge, which keeps generated_at)
    # would be lost or mis-rated.
    if ($baseRaw -cne $snapRaw) {
        throw "$outPath changed while this run was in progress; the asset was NOT written. Delete $Checkpoint and rerun."
    }
    $base = $baseRaw | ConvertFrom-Json
    $byKey = @{}
    foreach ($r in $rows) { $byKey["$($r.family)|$($r.infamous)|$($r.package)|$($r.money_skill)"] = $r }
    $merged = [System.Collections.Generic.List[object]]::new()
    foreach ($r in @($base.rows)) {
        $k = "$($r.family)|$($r.infamous)|$($r.package)|$($r.money_skill)"
        if ($byKey.ContainsKey($k)) { $merged.Add($byKey[$k]); $byKey.Remove($k) } else { $merged.Add($r) }
    }
    foreach ($r in $rows) {
        $k = "$($r.family)|$($r.infamous)|$($r.package)|$($r.money_skill)"
        if ($byKey.ContainsKey($k)) { $merged.Add($r); $byKey.Remove($k) }
    }
    $finalRows = $merged
    Write-Host ("Merged {0} re-run rows into {1} existing rows" -f $rows.Count, @($base.rows).Count)
}
$document = [ordered]@{
    # A partial merge must not advance the snapshot-wide timestamp; each row carries its own sampled_at.
    generated_at     = $(if (($PartnerSweep -or $Merge) -and $snapGenerated) { $snapGenerated } else { [datetime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'") })
    league           = $League
    patch            = $patchShort
    source           = "https://www.pathofexile.com/api/trade/search (status $listingStatus, ilvl >= $MinLevel, sort price asc; cheapest $SampleSize fetched; one mercenary stats group per package; listings caps at 10000; chaos rates from poe.ninja stash currency overview)"
    chaos_per_divine = [double]$chaosPerDivine
    min_level        = [int]$MinLevel
    placeholder      = $false
    rows             = @($finalRows)
}
$json = $document | ConvertTo-Json -Depth 8
$null = $json | ConvertFrom-Json   # must round-trip before it can replace anything
$tmpPath = Join-Path $outDir ((Split-Path -Leaf $outPath) + ".tmp-$PID")
[System.IO.File]::WriteAllText($tmpPath, $json + "`r`n", [System.Text.UTF8Encoding]::new($false))
Move-Item -LiteralPath $tmpPath -Destination $outPath -Force
if (Test-Path -LiteralPath $Checkpoint) { Remove-Item -LiteralPath $Checkpoint -Force }

$elapsed = [datetime]::UtcNow - $runStart
Write-Host ''
Write-Host ("Wrote {0} rows to {1} in {2:n0} min: {3} searches, {4} fetches, {5} other requests, {6} HTTP 429." -f $finalRows.Count, $outPath, $elapsed.TotalMinutes, $script:Counts.search, $script:Counts.fetch, $script:Counts.other, $script:Counts.r429)
if ($skipped.Count) {
    Write-Host "Skipped rows ($($skipped.Count)):"
    foreach ($s in $skipped) { Write-Host "  $s" }
}
$rows | ForEach-Object {
    [pscustomobject]@{
        family   = $_.family
        infamous = $_.infamous
        package  = $_.package
        skill    = $_.money_skill
        listings = $_.listings
        sampled  = $_.sampled
        low_div  = [math]::Round($_.lowest_chaos / $chaosPerDivine, 2)
        med_div  = [math]::Round($_.median_chaos / $chaosPerDivine, 2)
        age_days = $_.median_age_days
        fresh    = $_.fresh_share
        scanned  = $(if ($_.Contains('scanned')) { $_.scanned } else { $null })
        passed   = $(if ($_.Contains('passed')) { $_.passed } else { $null })
    }
} | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
