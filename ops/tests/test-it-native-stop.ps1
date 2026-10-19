#Requires -Version 5.1
# Durable, committed version of the proven ephemeral smoke .tmp/f1-hermetic.ps1
# (mission 20261019-it-fixtures-lifecycle; QA plan gaps G-1/G-2/G-4/G-5; review
# findings T-1 HIGH, T-3d, T-6).
# Hermetic end to end: a file-driven fake docker.exe/cargo.exe shim
# (support/it-native-fake.ps1) answers every invocation; no Docker daemon, no
# Cargo build, no Kind. Fail-closed branches live in
# test-it-native-stop-failclosed.ps1.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'support\it-native-fake.ps1')

$launcher = Join-Path $PSScriptRoot 'it-native.ps1'

# T-3d static half: no command AST in the launcher may even spell a prune
# subcommand, whatever the docker/cargo indirection used.
Assert-ItLauncherHasNoDestructiveCall -LauncherPath $launcher
Write-Host 'PASS static: no prune subcommand anywhere in the launcher command ASTs (T-3d)'

function Assert-ItEnvironmentRestored {
    # The launcher must restore the caller's process environment (sentinels).
    foreach ($check in @(
            @('CARGO_TARGET_DIR', 'sentinel-target'),
            @('RUST_TEST_THREADS', 'sentinel-threads'),
            @('RUSTYCOG_TEST_RUNNER_MODE', 'sentinel-mode'),
            @('RUSTYCOG_TEST_RUNNER_HOST', 'sentinel-host'),
            @('RUSTYCOG_TEST_RUN_ID', 'sentinel-run'),
            @('RUSTYCOG_TEST_LEDGER_DIR', 'sentinel-ledger'))) {
        if ([Environment]::GetEnvironmentVariable($check[0], 'Process') -ne $check[1]) {
            throw "Environment variable $($check[0]) was not restored to its sentinel value"
        }
    }
}

function Assert-ItRegistryLastEventWins {
    # G-5: last event per owned ID must be removed/safe_to_delete=yes.
    param([string]$RegistryPath, [string[]]$OwnedIds)
    $lastEvent = @{}
    foreach ($line in (Get-Content -LiteralPath $RegistryPath)) {
        $record = $line | ConvertFrom-Json
        $lastEvent[[string]$record.id] = $record
    }
    foreach ($id in $OwnedIds) {
        if ($lastEvent[$id].event -ne 'removed' -or $lastEvent[$id].safe_to_delete -ne 'yes') {
            $last = $lastEvent[$id]
            throw "Registry last event for $id is not removed/safe_to_delete=yes (got event=$($last.event), safety=$($last.safe_to_delete))"
        }
    }
}

Invoke-ItFakeScenario 'G-1/G-2/G-4/G-5/T-6: success batch then failing batch, same sandbox' {
    param($sb)
    $oldEnv = @{}
    foreach ($name in @('CARGO_TARGET_DIR', 'RUST_TEST_THREADS', 'RUSTYCOG_TEST_RUNNER_MODE', 'RUSTYCOG_TEST_RUNNER_HOST', 'RUSTYCOG_TEST_RUN_ID', 'RUSTYCOG_TEST_LEDGER_DIR', 'AIFORALL_IT_TEST_THREADS', 'ITFAKE_FAIL', 'ITFAKE_OPAQUE')) {
        $oldEnv[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    }
    try {
        # Unrelated malformed legacy row in an older registry file: must be
        # tolerated without granting ownership or blocking unrelated IDs.
        $yesterday = Join-Path $sb.Tmp ('docker-resources-' + [DateTime]::UtcNow.AddDays(-1).ToString('yyyyMMdd') + '.log')
        [IO.File]::WriteAllText($yesterday, 'legacy malformed unrelated row')
        Add-ItFakeForeignContainer $sb
        foreach ($pair in @(
                @('CARGO_TARGET_DIR', 'sentinel-target'),
                @('RUST_TEST_THREADS', 'sentinel-threads'),
                @('RUSTYCOG_TEST_RUNNER_MODE', 'sentinel-mode'),
                @('RUSTYCOG_TEST_RUNNER_HOST', 'sentinel-host'),
                @('RUSTYCOG_TEST_RUN_ID', 'sentinel-run'),
                @('RUSTYCOG_TEST_LEDGER_DIR', 'sentinel-ledger'))) {
            [Environment]::SetEnvironmentVariable($pair[0], $pair[1], 'Process')
        }
        $env:AIFORALL_IT_TEST_THREADS = '3' # T-6: override exercised end-to-end

        # --- Invocation 1: success batch ------------------------------------
        $failed = $null
        try { & $sb.Launcher -Suite iam,hive } catch { $failed = $_ }
        if ($null -ne $failed) { throw "Success campaign failed: $($failed | Out-String)" }

        $ownedA = Get-ItFakePackageId iam
        $ownedB = Get-ItFakePackageId hive

        $calls1 = Get-ItFakeCalls $sb
        $cargoCalls = @($calls1 | Where-Object { $_ -like 'cargo *' })
        if ($cargoCalls.Count -ne 2) { throw "Batch did not execute exactly two packages (got $($cargoCalls.Count) cargo lines)" }
        if ($cargoCalls[0] -notmatch 'threads=3') { throw "AIFORALL_IT_TEST_THREADS=3 was not propagated end-to-end into the cargo environment: $($cargoCalls[0])" }
        if (Test-Path -LiteralPath (Join-Path $sb.State $ownedA)) { throw "Owned fixture $ownedA leaked" }
        if (Test-Path -LiteralPath (Join-Path $sb.State $ownedB)) { throw "Owned fixture $ownedB leaked" }
        Assert-ItForeignUntouched $sb
        Assert-ItEnvironmentRestored
        Assert-ItNoPrune $sb
        Assert-ItStopAfterCargo -Calls $calls1
        Assert-ItDockerTargets $sb -Action 'stop' -RequiredIds @($ownedA, $ownedB)
        Assert-ItDockerTargets $sb -Action 'rm' -RequiredIds @($ownedA, $ownedB)

        $registry = Get-ItFakeRegistryPath $sb
        foreach ($line in (Get-Content -LiteralPath $registry)) { $null = $line | ConvertFrom-Json }
        Assert-ItRegistryLastEventWins -RegistryPath $registry -OwnedIds @($ownedA, $ownedB)

        $report = Get-ItFakeReport $sb -RunIndex 1
        if ($report.created -ne 2 -or $report.removed -ne 2 -or $report.still_running -ne 0 -or
            $report.owned_running -ne 0 -or $report.new_unowned -ne 0 -or $report.peak_running_owned -ne 2 -or
            $report.cargo_exit_code -ne 0 -or $report.identity_unknown -or @($report.cleanup_issues).Count -ne 0) {
            throw "Success inventory counters wrong: $($report | ConvertTo-Json -Depth 4)"
        }
        $runs = Get-ItFakeRunDirs $sb
        if ($runs.Count -ne 1) { throw "Expected 1 campaign run directory, got $($runs.Count)" }
        if ($runs[0].Name -notmatch '^itfixtures-[A-Za-z0-9-]{1,40}$') { throw "Invalid bounded run ID: $($runs[0].Name)" }
        $registryPrefix = [IO.File]::ReadAllText($registry)
        Write-Host 'PASS G-2/G-4/G-5/T-6: success batch - exact full-ID stop+rm, foreign preserved, env restored, override propagated, registry last-event-wins, counters exact'

        # --- Invocation 2: cargo fails (exit 7) in the same sandbox ---------
        $env:AIFORALL_IT_TEST_THREADS = $null
        $env:ITFAKE_FAIL = '1'
        $thrown = $null
        try { & $sb.Launcher -Suite iam,hive } catch { $thrown = $_ }
        if ($null -eq $thrown) { throw 'Cargo failure was hidden' }
        if (($thrown | Out-String) -notlike '*exit 7*') { throw "Failure did not preserve the cargo exit code: $thrown" }
        if (Test-Path -LiteralPath (Join-Path $sb.State $ownedA)) { throw "Owned fixture $ownedA leaked after cargo failure" }
        if (Test-Path -LiteralPath (Join-Path $sb.State $ownedB)) { throw "Owned fixture $ownedB leaked after cargo failure" }
        Assert-ItForeignUntouched $sb
        Assert-ItEnvironmentRestored
        Assert-ItNoPrune $sb
        $calls2 = Get-ItFakeCalls $sb
        $cargoCalls2 = @($calls2 | Where-Object { $_ -like 'cargo *' })
        Assert-ItStopAfterCargo -Calls $calls2 -StartIndex $calls1.Count
        # Fail-fast batch: the failing suite ends the cargo line; only what was
        # already created (iam) is stopped and removed, never the later suites.
        Assert-ItDockerTargets $sb -Action 'stop' -RequiredIds @($ownedA) -ForbiddenIds @($ownedB) -StartIndex $calls1.Count
        Assert-ItDockerTargets $sb -Action 'rm' -RequiredIds @($ownedA) -ForbiddenIds @($ownedB) -StartIndex $calls1.Count
        if (-not [IO.File]::ReadAllText($registry).StartsWith($registryPrefix)) { throw 'Registry was rewritten (append-only violated across failure)' }
        foreach ($line in (Get-Content -LiteralPath $registry)) { $null = $line | ConvertFrom-Json }

        if ($cargoCalls2.Count -ne 3) { throw "Expected 3 cargo lines (fail-fast batch ends at the first failing suite), got $($cargoCalls2.Count)" }
        if ($cargoCalls2[2] -notmatch 'threads=2') { throw "Unset AIFORALL_IT_TEST_THREADS must fall back to the default cap 2 end-to-end: $($cargoCalls2[2])" }

        $runs2 = Get-ItFakeRunDirs $sb
        if ($runs2.Count -ne 2) { throw 'Run directories were reused across invocations' }
        $runIds = @($runs2 | ForEach-Object { $_.Name -replace '^itfixtures-', '' })
        if ($runIds[0] -eq $runIds[1]) { throw 'Run ID reused across invocations' }
        $report2 = Get-ItFakeReport $sb -RunIndex 2
        if ($report2.cargo_exit_code -ne 7 -or $report2.created -ne 1 -or $report2.removed -ne 1 -or
            $report2.still_running -ne 0 -or @($report2.cleanup_issues).Count -ne 0) {
            throw "Failure inventory counters wrong: $($report2 | ConvertTo-Json -Depth 4)"
        }
        Write-Host 'PASS G-4: cargo exit 7 propagated, cleanup executed anyway, registry append-only, distinct bounded run IDs (G-1)'
    }
    finally {
        foreach ($name in $oldEnv.Keys) { [Environment]::SetEnvironmentVariable($name, $oldEnv[$name], 'Process') }
    }
}

# G-4 exact propagation through a real child powershell process: success must
# exit 0, cargo failure must exit nonzero with the cargo code in the report.
Invoke-ItFakeScenario 'G-4: child-process exit codes 0 and 7 propagated exactly' {
    param($sb)
    $ownedA = Get-ItFakePackageId iam
    $ownedB = Get-ItFakePackageId hive
    Add-ItFakeForeignContainer $sb
    $oldEnv = @{}
    foreach ($name in @('ITFAKE_FAIL', 'ITFAKE_OPAQUE')) {
        $oldEnv[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    }
    try {
        foreach ($case in @(@{ Fail = $false; ExpectExit = 0 }, @{ Fail = $true; ExpectExit = 1 })) {
            $startLine = (Get-ItFakeCalls $sb).Count
            if ($case.Fail) { $env:ITFAKE_FAIL = '1' } else { $env:ITFAKE_FAIL = $null }
            # -Command, not -File: -File would pass 'iam,hive' as one literal
            # string and fail ValidateSet before the launcher even runs.
            $command = "& '$($sb.Launcher)' -Suite iam,hive"
            $prevEap = $ErrorActionPreference
            $ErrorActionPreference = 'Continue'
            try {
                $output = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $command 2>&1)
                $exitCode = $LASTEXITCODE
            }
            finally { $ErrorActionPreference = $prevEap }
            if ($exitCode -ne $case.ExpectExit) {
                throw "Child launcher exit code $exitCode, expected $($case.ExpectExit) (fail=$($case.Fail))"
            }
            if ($case.Fail) {
                if (($output | Out-String) -notlike '*exit 7*') { throw 'Child failure output lost the cargo exit code' }
            }
            if ((Test-Path -LiteralPath (Join-Path $sb.State $ownedA)) -or (Test-Path -LiteralPath (Join-Path $sb.State $ownedB))) {
                throw "Owned fixtures leaked in child run (fail=$($case.Fail))"
            }
            Assert-ItForeignUntouched $sb
            Assert-ItNoPrune $sb
            $callsNow = Get-ItFakeCalls $sb
            Assert-ItStopAfterCargo -Calls $callsNow -StartIndex $startLine
        }
        Assert-ItDockerTargets $sb -Action 'rm' -RequiredIds @($ownedA, $ownedB)
    }
    finally {
        foreach ($name in $oldEnv.Keys) { [Environment]::SetEnvironmentVariable($name, $oldEnv[$name], 'Process') }
    }
}

Write-Host 'PASS: hermetic launcher tests - G-2 stop by full ID with foreign preservation, G-4 exit-code propagation both branches (in-process + child), G-5 append-only last-event-wins registry, G-1 distinct bounded run IDs, T-3d prune never invoked (static + dynamic), T-6 thread override end-to-end'
