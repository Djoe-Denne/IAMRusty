#Requires -Version 5.1
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('iam', 'hive', 'manifesto', 'telegraph', 'lazaret')]
    [string[]]$Suite
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../deploy/common.ps1')
$root = Get-AIForAllRoot $PSScriptRoot
$packages = @{
    iam = 'iam-service'
    hive = 'hive-service'
    manifesto = 'manifesto-service'
    telegraph = 'telegraph-service'
    lazaret = 'lazaret-service'
}
function Resolve-ItTestThreads {
    param([string]$Override)
    if ([string]::IsNullOrEmpty($Override)) { return 2 }
    $threads = 0
    if ($Override -notmatch '^[0-9]+$' -or -not [int]::TryParse($Override, [ref]$threads) -or $threads -lt 1) {
        throw 'AIFORALL_IT_TEST_THREADS must be a positive 32-bit integer; STOP before Cargo'
    }
    return $threads
}
# Validate before any runtime/observer startup or Cargo invocation. Ambient
# RUST_TEST_THREADS is deliberately not the override for a reproducible default.
$testThreads = Resolve-ItTestThreads $env:AIFORALL_IT_TEST_THREADS
# The caller owns the single Cargo slot. This invocation owns ONLY IDs proved by
# its fresh fixture ledgers, never a name match or a before/after inventory diff.
Get-Command cargo -ErrorAction Stop | Out-Null
Get-Command docker -ErrorAction Stop | Out-Null
$runId = 'it-' + [guid]::NewGuid().ToString('N')
$tempRoot = Join-Path $root '.tmp'
if ((Test-Path -LiteralPath $tempRoot) -and ((Get-Item -LiteralPath $tempRoot).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
    throw 'Campaign ledger parent must not be a symlink/reparse point; STOP'
}
$evidence = Join-Path $tempRoot "itfixtures-$runId"
$null = New-Item -ItemType Directory -Path $evidence
if (-not [IO.Path]::IsPathRooted($evidence) -or $runId -notmatch '^[A-Za-z0-9-]{1,40}$' -or ((Get-Item -LiteralPath $evidence).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
    throw 'Campaign ledger directory must be absolute, existing and non-symlink; STOP'
}
$registry = Join-Path $root ('.tmp/docker-resources-' + [DateTime]::UtcNow.ToString('yyyyMMdd') + '.log')
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Write-ItJsonLine {
    param([string]$Path, $Record)
    # Close/flush on every append. Never rewrite a fixture ledger or registry.
    [IO.File]::AppendAllText($Path, (($Record | ConvertTo-Json -Compress -Depth 8) + "`n"), $utf8)
}

function Get-ItInventory {
    $lines = @(& docker ps -a --no-trunc --format '{{.ID}}|{{.State}}|{{.Status}}')
    if ($LASTEXITCODE -ne 0) { throw 'Docker inventory failed; cleanup INCONCLUSIVE' }
    foreach ($line in $lines) {
        $parts = $line -split '\|', 3
        if ($parts.Count -ne 3 -or $parts[0] -notmatch '^[a-f0-9]{64}$') {
            throw 'Docker inventory returned an invalid full ID; STOP'
        }
        [pscustomobject]@{ id = $parts[0]; state = $parts[1]; status = $parts[2] }
    }
}

function Write-ItResource {
    param([string]$Id, [string]$Event, [string]$Safety, [string]$Reason, [string]$Ledger)
    Write-ItJsonLine $registry ([ordered]@{
        ts = [DateTime]::UtcNow.ToString('o'); actor = 'it-native'; task = $runId
        kind = 'container'; id = $Id; event = $Event; prior_state = 'absent'
        safe_to_delete = $Safety; safe_reason = $Reason; session_ledger = $Ledger
    })
}

function Get-ItOwned {
    $owned = @{}
    foreach ($file in @(Get-ChildItem -LiteralPath $evidence -Filter 'fixture-*.jsonl')) {
        foreach ($line in [IO.File]::ReadAllLines($file.FullName)) {
            try {
                $record = $line | ConvertFrom-Json
                if ($record.run_id -ne $runId) { throw 'Foreign run in campaign ledger; STOP' }
                if ($record.event -eq 'created') {
                    $id = [string]$record.container_id
                    if ($id -notmatch '^[a-f0-9]{64}$') { throw 'Fixture creation has no full ID; cleanup INCONCLUSIVE' }
                    if ($beforeIds.ContainsKey($id)) { throw 'Fixture claims a pre-existing container; STOP' }
                    if (-not $script:createdWitness.ContainsKey($id)) {
                        throw "Fixture $id has no Engine creation witness; cleanup INCONCLUSIVE"
                    }
                    $owned[$id] = $file.FullName
                }
                if ($record.event -like 'unknown*') {
                    Write-ItResource 'UNKNOWN' 'creation_unknown' 'ask' 'Opaque fixture creation; not owned for cleanup' $file.FullName
                    $script:identityUnknown = $true
                }
            }
            catch { $script:ledgerIssues.Add("$($file.Name): $($_.Exception.Message)") }
        }
    }
    return $owned
}

function Get-ItRegistryHolds {
    param([hashtable]$Owned)
    # Reconcile ALL relevant daily registries, not just today's file. A foreign
    # active/doubtful lease is never overridden by this campaign's claim.
    $latest = @{}
    $holds = @{}
    $legacyLines = 0
    foreach ($file in @(Get-ChildItem -LiteralPath (Join-Path $root '.tmp') -Filter 'docker-resources-*.log' | Sort-Object Name)) {
        foreach ($line in [IO.File]::ReadAllLines($file.FullName)) {
            if (-not $line.Trim()) { continue }
            try { $record = $line | ConvertFrom-Json }
            catch {
                $legacyLines++
                # Historical malformed/unrelated rows cannot grant ownership or
                # globally disable cleanup. If they mention one of our exact
                # IDs, ambiguity DOES block that ID, fail-closed.
                foreach ($id in $Owned.Keys) { if ($line.Contains($id)) { $holds[$id] = $true } }
                continue
            }
            if (-not $record.PSObject.Properties['id'] -or -not $Owned.ContainsKey([string]$record.id)) { continue }
            $task = if ($record.PSObject.Properties['task']) { [string]$record.task } else { '<unknown-task>' }
            if ($task -ne $runId) {
                $latest[([string]$record.id + '/' + $task)] = $record
            }
        }
    }
    foreach ($record in $latest.Values) {
        if (-not $record.PSObject.Properties['safe_to_delete'] -or $record.safe_to_delete -ne 'yes') { $holds[[string]$record.id] = $true }
    }
    if ($legacyLines -gt 0) { Write-Warning "Registry contains $legacyLines legacy malformed rows; exact-ID ambiguities retained, unrelated rows ignored" }
    return $holds
}

# Observe Engine creation independently of the library's readiness wait. An
# Engine event is NOT an ownership grant: initially ask; only a matching created
# ID in this run's fixture ledger may be stopped/reclassified later. Observing
# unrelated creations never adopts them. The cursor uses DAEMON time, not WSL/
# Windows wall-clock time. Raw events contain no inspect environment/secrets.
$observerBody = {
    param($Since, $Directory, $Registry, $Run)
    $ErrorActionPreference = 'Stop'
    $encoding = New-Object System.Text.UTF8Encoding($false)
    $client = New-Object System.Diagnostics.Process
    try {
        $client.StartInfo.FileName = (Get-Command docker -ErrorAction Stop).Source
        $client.StartInfo.Arguments = 'events --filter type=container --since "' + $Since + '" --format "{{json .}}"'
        $client.StartInfo.UseShellExecute = $false
        $client.StartInfo.CreateNoWindow = $true
        $client.StartInfo.RedirectStandardOutput = $true
        $client.StartInfo.RedirectStandardError = $true
        if (-not $client.Start()) { throw 'Cannot start Engine event witness' }
        $stderr = $client.StandardError.ReadToEndAsync()
        $pending = $client.StandardOutput.ReadLineAsync()
        [IO.File]::WriteAllText((Join-Path $Directory 'observer-ready'), 'ready', $encoding)
        while (-not (Test-Path -LiteralPath (Join-Path $Directory 'observer-stop'))) {
            if ($pending.Wait(100)) {
                $line = $pending.Result
                if ($null -eq $line) { throw 'Engine event witness exited unexpectedly' }
                $event = $line | ConvertFrom-Json
                $id = [string]$event.Actor.ID
                if ($id -notmatch '^[a-f0-9]{64}$') { throw 'Engine event witness returned invalid ID' }
                # Persist only identity/action/time (not arbitrary Actor attributes).
                $record = [ordered]@{ id = $id; action = $event.Action; timeNano = $event.timeNano }
                if ($event.Action -eq 'create') {
                    $resource = [ordered]@{
                        ts = [DateTime]::UtcNow.ToString('o'); actor = 'it-native-engine-witness'; task = $Run
                        kind = 'container'; id = $id; event = 'created'; prior_state = 'absent'
                        safe_to_delete = 'ask'; safe_reason = 'Engine creation witness only; ownership awaits fixture ledger ID'
                        session_ledger = (Join-Path $Directory 'engine-events.jsonl')
                    }
                    [IO.File]::AppendAllText($Registry, (($resource | ConvertTo-Json -Compress) + "`n"), $encoding)
                }
                [IO.File]::AppendAllText((Join-Path $Directory 'engine-events.jsonl'), (($record | ConvertTo-Json -Compress) + "`n"), $encoding)
                $pending = $client.StandardOutput.ReadLineAsync()
            }
        }
    }
    catch {
        [IO.File]::WriteAllText((Join-Path $Directory 'observer-error'), $_.Exception.Message, $encoding)
        throw
    }
    finally {
        # Terminate only our read-only docker EVENTS client, never a container.
        if ($client.Id -and -not $client.HasExited) { $client.Kill(); $client.WaitForExit() }
        $client.Dispose()
    }
}

$previous = @{}
foreach ($name in @('CARGO_TARGET_DIR', 'RUST_TEST_THREADS', 'RUSTYCOG_TEST_RUNNER_MODE', 'RUSTYCOG_TEST_RUNNER_HOST', 'RUSTYCOG_TEST_RUN_ID', 'RUSTYCOG_TEST_LEDGER_DIR')) {
    $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
$beforeIds = @{}
$observer = $null
$identityUnknown = $false
$suiteFailure = $null
$cleanupFailure = $null
$daemonTime = @()
$createdWitness = @{}
$ledgerIssues = New-Object 'System.Collections.Generic.List[string]'
$cargoExit = 0
Push-Location $root
try {
    $before = @(Get-ItInventory)
    foreach ($container in $before) { $beforeIds[$container.id] = $true }
    Write-ItJsonLine (Join-Path $evidence 'inventory.jsonl') @{ phase = 'before'; containers = $before }
    $daemonTime = @(& docker info --format '{{.SystemTime}}')
    if ($LASTEXITCODE -ne 0 -or $daemonTime.Count -ne 1 -or -not $daemonTime[0]) { throw 'Cannot obtain daemon event cursor; STOP' }
    $observer = Start-Job -ScriptBlock $observerBody -ArgumentList $daemonTime[0], $evidence, $registry, $runId
    $readyDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (-not (Test-Path -LiteralPath (Join-Path $evidence 'observer-ready'))) {
        if ($observer.State -in @('Failed', 'Completed', 'Stopped') -or [DateTime]::UtcNow -gt $readyDeadline) {
            throw 'Engine event witness unavailable before Cargo; STOP'
        }
        Start-Sleep -Milliseconds 100
    }
    $env:CARGO_TARGET_DIR = 'E:\cargo-target\AIForAll'
    $env:RUSTYCOG_TEST_RUNNER_MODE = 'local'
    $env:RUSTYCOG_TEST_RUNNER_HOST = $null
    $env:RUSTYCOG_TEST_RUN_ID = $runId
    $env:RUSTYCOG_TEST_LEDGER_DIR = $evidence
    $env:RUST_TEST_THREADS = [string]$testThreads
    Write-Host "IT libtest threads per binary: $testThreads (override: AIFORALL_IT_TEST_THREADS)"
    Write-Host "IT campaign $runId; evidence: $evidence; registry: $registry"
    foreach ($selected in $Suite) {
        & cargo test -p $packages[$selected] --tests
        $cargoExit = $LASTEXITCODE
        if ($cargoExit -ne 0) { throw "Native integration suite failed: $selected (exit $cargoExit)" }
        if ($observer.State -ne 'Running' -or (Test-Path -LiteralPath (Join-Path $evidence 'observer-error'))) {
            throw 'Engine event witness failed during suite; cleanup INCONCLUSIVE'
        }
    }
}
catch { $suiteFailure = $_ }
finally {
    try {
        # Drain the exact campaign window using daemon time. This catches events
        # still buffered by the streaming CLI. Do not use our host clock, nor
        # treat the Engine witness alone as permission to stop anything.
        if ($null -ne $observer) {
            $endTime = @(& docker info --format '{{.SystemTime}}')
            if ($LASTEXITCODE -ne 0 -or $endTime.Count -ne 1 -or -not $endTime[0]) { throw 'Cannot obtain end cursor; cleanup INCONCLUSIVE' }
            $events = @(& docker events --since $daemonTime[0] --until $endTime[0] --filter type=container --format '{{json .}}')
            if ($LASTEXITCODE -ne 0) { throw 'Cannot drain Engine witness; cleanup INCONCLUSIVE' }
            $window = @{}
            foreach ($line in $events) {
                $event = $line | ConvertFrom-Json
                $id = [string]$event.Actor.ID
                if ($id -notmatch '^[a-f0-9]{64}$') { throw 'Invalid Engine event ID; cleanup INCONCLUSIVE' }
                $window["$id/$($event.timeNano)/$($event.Action)"] = [pscustomobject]@{ id = $id; action = $event.Action; timeNano = $event.timeNano }
            }
            [IO.File]::WriteAllText((Join-Path $evidence 'observer-stop'), 'stop', $utf8)
            if ($null -eq (Wait-Job $observer -Timeout 10)) { throw 'Engine witness stop timed out; cleanup INCONCLUSIVE' }
            # Engine historical replay is limited (typically last 256 events).
            # Keep the live stream as primary evidence; merge/deduplicate the
            # bounded replay tail rather than losing early campaign creations.
            $livePath = Join-Path $evidence 'engine-events.jsonl'
            if (Test-Path -LiteralPath $livePath) {
                foreach ($line in [IO.File]::ReadAllLines($livePath)) {
                    $event = $line | ConvertFrom-Json
                    $window["$($event.id)/$($event.timeNano)/$($event.action)"] = $event
                }
            }
            $events = @($window.Values | Sort-Object timeNano)
            $active = @{}
            foreach ($container in $before) { if ($container.state -eq 'running') { $active[$container.id] = $true } }
            $peak = $active.Count
            foreach ($event in $events) {
                $id = [string]$event.id
                Write-ItJsonLine (Join-Path $evidence 'engine-window.jsonl') $event
                if ($event.action -eq 'create') { $createdWitness[$id] = $true }
                if ($event.action -eq 'start') { $active[$id] = $true; $peak = [Math]::Max($peak, $active.Count) }
                if ($event.action -in @('die', 'destroy')) { $active.Remove($id) }
            }
            Write-ItJsonLine (Join-Path $evidence 'inventory.jsonl') @{ phase = 'campaign-window'; since = $daemonTime[0]; until = $endTime[0]; peak_running_total = $peak }
        }
        # Finish all Cargo consumers before releasing the campaign lease. Only
        # the chain fresh fixture ledger + Engine creation event grants ownership.
        $owned = Get-ItOwned
        $issues = New-Object 'System.Collections.Generic.List[string]'
        foreach ($issue in $ledgerIssues) { $issues.Add($issue) }
        $holds = Get-ItRegistryHolds $owned
        $registered = @{}
        $witnessPath = Join-Path $evidence 'engine-events.jsonl'
        if (Test-Path -LiteralPath $witnessPath) {
            foreach ($line in [IO.File]::ReadAllLines($witnessPath)) {
                $event = $line | ConvertFrom-Json
                if ($event.action -eq 'create') { $registered[[string]$event.id] = $true }
            }
        }
        foreach ($id in $createdWitness.Keys) {
            if (-not $registered.ContainsKey($id)) {
                Write-ItResource $id 'created' 'ask' 'Engine drain recovered creation missed by live registry; timing INCONCLUSIVE' (Join-Path $evidence 'engine-window.jsonl')
                $issues.Add("Creation $id was not registered live")
            }
            if (-not $owned.ContainsKey($id)) {
                Write-ItResource 'UNKNOWN' 'creation_unknown' 'ask' "Uncorrelated Engine creation $id; not owned for cleanup" (Join-Path $evidence 'engine-window.jsonl')
                $issues.Add("Engine creation $id has no matching fixture creation ledger; retained")
            }
        }
        $preCleanup = @(Get-ItInventory)
        Write-ItJsonLine (Join-Path $evidence 'inventory.jsonl') @{ phase = 'before-cleanup'; containers = $preCleanup; owned_ids = @($owned.Keys) }
        $removedCount = 0
        foreach ($id in $owned.Keys) {
            try {
                if ($holds.ContainsKey($id)) { throw "Foreign registry lease/ask for $id; retained, arbitration required" }
                Write-ItResource $id 'reclassified' 'no' 'Fixture ledger proves campaign creation; cleanup in progress' $owned[$id]
                $present = @((Get-ItInventory) | Where-Object { $_.id -eq $id })
                if ($present.Count -eq 0) {
                    Write-ItResource $id 'removed' 'yes' 'Already removed by fixture teardown' $owned[$id]
                    $removedCount++
                    continue
                }
                if ($present[0].state -eq 'running') {
                    Write-Host "Gracefully stopping owned fixture $id (volatile fixture memory will be lost)"
                    # A negative timeout never sends SIGKILL on expiry. Bound the
                    # CLI wait separately; on timeout leave/report, never force.
                    $stop = New-Object System.Diagnostics.Process
                    try {
                        $stop.StartInfo.FileName = (Get-Command docker).Source
                        $stop.StartInfo.Arguments = 'stop --time -1 ' + $id
                        $stop.StartInfo.UseShellExecute = $false
                        if (-not $stop.Start()) { throw "Cannot start graceful stop for $id" }
                        if (-not $stop.WaitForExit(30000)) {
                            $stop.Kill(); $stop.WaitForExit()
                            throw "Graceful stop timed out for $id; no forced container termination"
                        }
                        if ($stop.ExitCode -ne 0) { throw "Graceful stop failed for $id" }
                    }
                    finally { $stop.Dispose() }
                    Write-ItResource $id 'stopped' 'no' 'Graceful stop completed; campaign cleanup still holds lease' $owned[$id]
                }
                $present = @((Get-ItInventory) | Where-Object { $_.id -eq $id })
                if ($present.Count -eq 0) {
                    Write-ItResource $id 'removed' 'yes' 'Removed by fixture teardown during cleanup' $owned[$id]
                    $removedCount++
                    continue
                }
                if ($present[0].state -ne 'exited') { throw "Owned fixture $id is not exited; retained" }
                $exitState = @(& docker inspect --format '{{.State.Status}}|{{.State.ExitCode}}' $id)
                if ($LASTEXITCODE -ne 0 -or $exitState.Count -ne 1 -or $exitState[0] -notmatch '^exited\|-?\d+$') { throw "Cannot verify exited fixture $id" }
                if ($exitState[0] -eq 'exited|0') {
                    Write-ItResource $id 'reclassified' 'yes' 'Campaign lease released; owned exited(0) fixture has no state to preserve' $owned[$id]
                    & docker rm $id
                    if ($LASTEXITCODE -ne 0) { throw "Removal failed for exited(0) fixture $id" }
                    Write-ItResource $id 'removed' 'yes' 'Owned exited(0) fixture removed by full ID, without force or volumes' $owned[$id]
                    $removedCount++
                }
                else {
                    Write-ItResource $id 'reclassified' 'ask' 'Nonzero exit retained for diagnosis; no automatic removal' $owned[$id]
                    Write-Warning "Retaining owned fixture $id ($($exitState[0])) for diagnosis"
                }
            }
            catch { $issues.Add($_.Exception.Message) }
        }
        $after = @(Get-ItInventory)
        $runningOwned = @($after | Where-Object { $owned.ContainsKey($_.id) -and $_.state -eq 'running' })
        $unknownNew = @($after | Where-Object { -not $beforeIds.ContainsKey($_.id) -and -not $owned.ContainsKey($_.id) })
        $activeOwned = @{}
        $peakOwned = 0
        if ($null -ne $observer) {
            foreach ($event in $events) {
                $id = [string]$event.id
                if (-not $owned.ContainsKey($id)) { continue }
                if ($event.Action -eq 'start') { $activeOwned[$id] = $true; $peakOwned = [Math]::Max($peakOwned, $activeOwned.Count) }
                if ($event.Action -in @('die', 'destroy')) { $activeOwned.Remove($id) }
            }
        }
        Write-ItJsonLine (Join-Path $evidence 'inventory.jsonl') @{ phase = 'after'; containers = $after; owned_ids = @($owned.Keys); created = $owned.Count; removed = $removedCount; still_running = $runningOwned.Count; owned_running = $runningOwned.Count; new_unowned = $unknownNew; identity_unknown = $identityUnknown; cleanup_issues = $issues.ToArray(); peak_running_owned = $peakOwned; cargo_exit_code = $cargoExit }
        Write-Host "IT inventory: before=$($before.Count), after=$($after.Count), owned=$($owned.Count), owned-running=$($runningOwned.Count), new-unowned=$($unknownNew.Count); evidence: $evidence"
        if ($runningOwned.Count -gt 0) { $issues.Add("Owned containers still running: $($runningOwned.id -join ',')") }
        if ($identityUnknown) { $issues.Add('Opaque fixture creation UNKNOWN; ownership/cleanup INCONCLUSIVE (see ledger)') }
        if ($issues.Count -gt 0) { throw ($issues -join '; ') }
    }
    catch {
        $cleanupFailure = $_
        try {
            Write-ItJsonLine (Join-Path $evidence 'inventory.jsonl') @{ phase = 'after-inconclusive'; containers = @(Get-ItInventory); cleanup_error = [string]$cleanupFailure; cargo_exit_code = $cargoExit }
        }
        catch { Write-Warning 'Final inventory unavailable; cleanup INCONCLUSIVE (see campaign evidence)' }
    }
    finally {
        try {
            if ($null -ne $observer) {
                [IO.File]::WriteAllText((Join-Path $evidence 'observer-stop'), 'stop', $utf8)
                $finished = Wait-Job $observer -Timeout 10
                if ($null -eq $finished) { Stop-Job $observer; $cleanupFailure = "$cleanupFailure; Engine event witness did not stop; cleanup INCONCLUSIVE" }
                if ($observer.State -eq 'Failed' -or (Test-Path -LiteralPath (Join-Path $evidence 'observer-error'))) {
                    $cleanupFailure = "$cleanupFailure; Engine event witness failed; creation registry/peak INCONCLUSIVE"
                }
                Remove-Job $observer
            }
        }
        catch { $cleanupFailure = "$cleanupFailure; Witness shutdown failed: $_" }
        finally {
            foreach ($name in $previous.Keys) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
            Pop-Location
        }
    }
}
if ($null -ne $suiteFailure -or $null -ne $cleanupFailure) {
    throw "IT campaign failed. Suite: $suiteFailure; cleanup: $cleanupFailure; evidence: $evidence"
}
