#Requires -Version 5.1
# Fail-closed branches of the IT launcher (mission 20261019-it-fixtures-lifecycle;
# QA plan gaps G-2 negative/G-3; review findings T-3b, T-3c, T-5, T-6).
# Hermetic: the file-driven fake docker.exe/cargo.exe shim
# (support/it-native-fake.ps1) answers every invocation; no Docker daemon, no
# Cargo build, no Kind. Happy-path and exit-code propagation live in
# test-it-native-stop.ps1.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'support\it-native-fake.ps1')

Assert-ItLauncherHasNoDestructiveCall -LauncherPath (Join-Path $PSScriptRoot 'it-native.ps1')

# T-3c: a foreign-task registry lease (valid JSON, safe_to_delete != yes) and a
# malformed legacy row that mentions an owned ID verbatim (LEGACY_HIT) must both
# block cleanup of that exact ID, fail-closed.
Invoke-ItFakeScenario 'T-3c: foreign lease and malformed legacy row hold owned IDs' {
    param($sb)
    $ownedA = Get-ItFakePackageId iam
    $ownedB = Get-ItFakePackageId hive
    Add-ItFakeForeignContainer $sb
    $lease = [ordered]@{
        ts = '2026-10-18T00:00:00.000Z'; actor = 'other-campaign'; task = 'other-campaign-run'
        kind = 'container'; id = $ownedA; event = 'created'; prior_state = 'absent'
        safe_to_delete = 'no'; safe_reason = 'foreign campaign lease'; session_ledger = 'unrelated'
    } | ConvertTo-Json -Compress
    # Older registry file: the launcher reconciles every docker-resources-*.log.
    $yesterday = Join-Path $sb.Tmp ('docker-resources-' + [DateTime]::UtcNow.AddDays(-1).ToString('yyyyMMdd') + '.log')
    [IO.File]::WriteAllText($yesterday, "$lease`nlegacy malformed row mentioning $ownedB retained for diagnosis")
    $thrown = $null
    try { & $sb.Launcher -Suite iam,hive } catch { $thrown = $_ }
    if ($null -eq $thrown) { throw 'Held owned IDs were cleaned anyway' }
    if (($thrown | Out-String) -notlike '*Foreign registry lease/ask*') { throw "Hold was not reported as a foreign lease: $thrown" }
    if ((Get-Content -LiteralPath (Join-Path $sb.State $ownedA)) -ne 'running') { throw "Held owned ID $ownedA was stopped or removed" }
    if ((Get-Content -LiteralPath (Join-Path $sb.State $ownedB)) -ne 'running') { throw "Held owned ID $ownedB was stopped or removed" }
    $callsH = Get-ItFakeCalls $sb
    $stops = @($callsH | Where-Object { $_ -like 'docker stop *' })
    $rms = @($callsH | Where-Object { $_ -like 'docker rm *' })
    if ($stops.Count -ne 0 -or $rms.Count -ne 0) { throw "Held owned IDs reached docker stop/rm (calls: $($stops | Out-String) $($rms | Out-String))" }
    $report = Get-ItFakeReport $sb -RunIndex 1
    if ($report.created -ne 2 -or $report.removed -ne 0 -or $report.still_running -ne 2) {
        throw "Held-campaign inventory counters wrong: $($report | ConvertTo-Json -Depth 4)"
    }
    # Today's registry stays valid JSONL and records no stop/removal for held IDs.
    $registry = Get-ItFakeRegistryPath $sb
    foreach ($line in (Get-Content -LiteralPath $registry)) {
        $record = $line | ConvertFrom-Json
        foreach ($id in @($ownedA, $ownedB)) {
            if ($record.id -eq $id -and @('stopped', 'removed') -contains $record.event) {
                throw "Held ID $id was recorded as $($record.event)"
            }
        }
    }
    Assert-ItForeignUntouched $sb
    Assert-ItNoPrune $sb
}

# G-3/T-3b: rm is refused unless inspect confirms exited|0. Three fail-closed
# variants: stop succeeds but state stays running (transition), ps says exited
# while inspect contradicts, inspect shows a nonzero exit (kept for diagnosis).
Invoke-ItFakeScenario 'G-3/T-3b: rm refused unless inspect confirms exited|0' {
    param($sb)
    $ownedA = Get-ItFakePackageId iam
    $ownedB = Get-ItFakePackageId hive
    $ownedC = Get-ItFakePackageId manifesto
    $ownedD = Get-ItFakePackageId telegraph
    Add-ItFakeForeignContainer $sb
    New-ItFakeStopOverride $sb -Package hive -Mode stuck
    New-ItFakeInspectOverride $sb -Package manifesto -Response 'running|137'
    New-ItFakeInspectOverride $sb -Package telegraph -Response 'exited|1'
    $thrown = $null
    try { & $sb.Launcher -Suite iam,hive,manifesto,telegraph } catch { $thrown = $_ }
    if ($null -eq $thrown) { throw 'Refused removals were not reported' }
    $message = $thrown | Out-String
    if ($message -notlike '*is not exited; retained*') { throw "Transitioning owned fixture not retained fail-closed: $message" }
    if ($message -notlike '*Cannot verify exited fixture*') { throw "Contradictory inspect not reported fail-closed: $message" }
    if (Test-Path -LiteralPath (Join-Path $sb.State $ownedA)) { throw 'Healthy owned fixture was not cleaned' }
    if ((Get-Content -LiteralPath (Join-Path $sb.State $ownedB)) -ne 'running') { throw 'Transitioning owned fixture was removed' }
    if ((Get-Content -LiteralPath (Join-Path $sb.State $ownedC)) -ne 'exited') { throw 'Contradictory owned fixture was removed' }
    if ((Get-Content -LiteralPath (Join-Path $sb.State $ownedD)) -ne 'exited') { throw 'Nonzero-exit owned fixture was removed' }
    Assert-ItDockerTargets $sb -Action 'stop' -RequiredIds @($ownedA, $ownedB, $ownedC, $ownedD)
    Assert-ItDockerTargets $sb -Action 'rm' -RequiredIds @($ownedA) -ForbiddenIds @($ownedB, $ownedC, $ownedD)
    $report = Get-ItFakeReport $sb -RunIndex 1
    if ($report.created -ne 4 -or $report.removed -ne 1 -or $report.still_running -ne 1) {
        throw "Refusal inventory counters wrong: $($report | ConvertTo-Json -Depth 4)"
    }
    # Diagnostic retention: the nonzero-exit fixture is reclassified ask, not deleted.
    $registry = Get-ItFakeRegistryPath $sb
    $askRows = @()
    foreach ($line in (Get-Content -LiteralPath $registry)) {
        $record = $line | ConvertFrom-Json
        if ($record.id -eq $ownedD -and $record.event -eq 'reclassified' -and $record.safe_to_delete -eq 'ask') { $askRows += $record }
    }
    if ($askRows.Count -eq 0) { throw 'Nonzero-exit owned fixture was not retained with safe_to_delete=ask for diagnosis' }
    Assert-ItForeignUntouched $sb
    Assert-ItNoPrune $sb
}

# G-2 negative: an opaque fixture start failure (no container identity, cargo
# exit 7) must be reported INCONCLUSIVE and never guessed into a stop target.
Invoke-ItFakeScenario 'G-2 negative: opaque fixture failure stays INCONCLUSIVE' {
    param($sb)
    Add-ItFakeForeignContainer $sb
    $env:ITFAKE_OPAQUE = '1'
    $env:ITFAKE_FAIL = '1'
    try {
        $thrown = $null
        try { & $sb.Launcher -Suite iam } catch { $thrown = $_ }
        if ($null -eq $thrown) { throw 'Opaque fixture failure was hidden' }
        $message = $thrown | Out-String
        if ($message -notlike '*exit 7*') { throw "Cargo exit code lost on opaque failure: $message" }
        if ($message -notlike '*INCONCLUSIVE*') { throw "Opaque failure was not reported INCONCLUSIVE: $message" }
        $callsU = Get-ItFakeCalls $sb
        $stops = @($callsU | Where-Object { $_ -like 'docker stop *' })
        $rms = @($callsU | Where-Object { $_ -like 'docker rm *' })
        if ($stops.Count -ne 0 -or $rms.Count -ne 0) { throw 'Cleanup invented a stop/rm target for an opaque failure' }
        $registry = Get-ItFakeRegistryPath $sb
        $unknownRows = @()
        foreach ($line in (Get-Content -LiteralPath $registry)) {
            $record = $line | ConvertFrom-Json
            if ($record.id -eq 'UNKNOWN' -and $record.event -eq 'creation_unknown' -and $record.safe_to_delete -eq 'ask') { $unknownRows += $record }
        }
        if ($unknownRows.Count -eq 0) { throw 'No UNKNOWN/creation_unknown/ask registry row for the opaque failure' }
        $report = Get-ItFakeReport $sb -RunIndex 1
        if (-not $report.identity_unknown -or $report.created -ne 0 -or $report.removed -ne 0 -or $report.still_running -ne 0) {
            throw "Opaque-failure inventory counters wrong: $($report | ConvertTo-Json -Depth 4)"
        }
        Assert-ItForeignUntouched $sb
        Assert-ItNoPrune $sb
    }
    finally {
        $env:ITFAKE_OPAQUE = $null
        $env:ITFAKE_FAIL = $null
    }
}

# T-5: a junctioned ledger parent (.tmp) must be rejected before any cargo run
# or evidence directory creation. Junctions need no elevation on Windows.
Invoke-ItFakeScenario 'T-5: junctioned ledger parent rejected before cargo' {
    param($sb)
    Remove-Item -LiteralPath $sb.Tmp -Recurse -Force
    $junctionTarget = Join-Path $sb.Root 'tmp-target'
    $null = New-Item -ItemType Directory -Path $junctionTarget
    $null = New-Item -ItemType Junction -Path $sb.Tmp -Target $junctionTarget
    $thrown = $null
    try { & $sb.Launcher -Suite iam } catch { $thrown = $_ }
    if ($null -eq $thrown) { throw 'Junctioned ledger parent was accepted' }
    if (($thrown | Out-String) -notlike '*symlink/reparse point*') { throw "Unexpected rejection: $thrown" }
    if (Test-Path -LiteralPath $sb.CallsPath) { throw "Preflight reached docker/cargo despite the junction: $((Get-Content -LiteralPath $sb.CallsPath) -join ' | ')" }
    $runs = @(Get-ChildItem -LiteralPath $sb.Tmp -Directory -Filter 'itfixtures-*' -ErrorAction SilentlyContinue)
    if ($runs.Count -ne 0) { throw 'Campaign evidence directory created under a junctioned parent' }
}

# T-6 negative: an invalid AIFORALL_IT_TEST_THREADS override must fail before
# any cargo/docker invocation, end-to-end through the real launcher entry point.
Invoke-ItFakeScenario 'T-6 negative: invalid thread override rejected before Cargo' {
    param($sb)
    $old = $env:AIFORALL_IT_TEST_THREADS
    try {
        $env:AIFORALL_IT_TEST_THREADS = '0'
        $thrown = $null
        try { & $sb.Launcher -Suite iam } catch { $thrown = $_ }
        if ($null -eq $thrown) { throw 'Invalid thread override accepted' }
        if (($thrown | Out-String) -notlike '*STOP before Cargo*') { throw "Unexpected rejection: $thrown" }
        if (Test-Path -LiteralPath $sb.CallsPath) { throw 'Cargo/docker ran despite the invalid thread override' }
    }
    finally { $env:AIFORALL_IT_TEST_THREADS = $old }
}

Write-Host 'PASS: fail-closed launcher tests - T-3c registry holds (foreign lease + malformed LEGACY_HIT), G-3/T-3b rm refused (transition/contradiction/nonzero diagnosis), G-2 opaque failure INCONCLUSIVE, T-5 junction rejection, T-6 invalid override rejection'
