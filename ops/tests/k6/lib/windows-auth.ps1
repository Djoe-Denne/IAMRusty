# Auth-specific expectations for the existing Windows parent. These helpers do
# not admit targets, start children, collect Admin data or mutate certificates.
# Do not use a successful child summary as a parent-correlated campaign verdict.
Set-StrictMode -Version Latest

function Get-K6AuthExpectation([string] $Case) {
    if ($Case -ceq 'auth-oauth-browser') {
        return [pscustomobject]@{
            Case = $Case; Phase = 'Chromium'; MaxDuration = '90s'; GracefulStop = '3s'; Requests = 0
            Checks = @(
                'browser-version', 'registration-callback', 'registration-required', 'registration-nonce',
                'registration-completed', 'login-callback', 'login-identity', 'login-authority', 'login-nonce',
                'relink-start', 'relink-callback', 'relink-identity', 'relink-nonce',
                'negative-link', 'negative-relink-start', 'negative-relink-alias',
                'oauth-observations', 'oauth-iam-cardinality', 'login-redirects', 'oauth-cookie-attributes',
                'oauth-callback-headers', 'registration-request-headers', 'relink-start-authorization',
                'provider-authorizations', 'provider-no-credentials', 'negative-headers'
            )
            SanPhase = $null
        }
    }
    if ($Case -cin @('auth-envoy-san-good', 'auth-envoy-san-bad', 'auth-envoy-san-restored')) {
        return [pscustomobject]@{
            Case = $Case; Phase = 'HTTP'; MaxDuration = '15s'; GracefulStop = '2s'; Requests = 1
            Checks = @('san-status', 'san-no-redirect')
            SanPhase = $Case.Substring('auth-envoy-san-'.Length)
        }
    }
    throw 'OPTIONS_INVALID'
}

function New-K6AuthEnvironment([string] $Directory, [string] $RunId, [string] $Chrome, [string[]] $Sentinels, $ExpectedBrowserVersion, [string] $Case) {
    $expected = Get-K6AuthExpectation $Case
    if ($RunId -cnotmatch '^[a-f0-9]{32}$') { throw 'INPUT_INVALID' }
    $environment = New-K6Environment $Directory $RunId $Chrome $Sentinels $ExpectedBrowserVersion
    if ($expected.Phase -ceq 'Chromium') { Assert-K6BrowserEligibility $environment $ExpectedBrowserVersion }
    $environment.Add('K6_ORIGIN', 'http://127.0.0.1:18080')
    if ($null -ne $expected.SanPhase) { $environment.Add('K6_AUTH_SAN_PHASE', $expected.SanPhase) }
    return ,$environment
}

function Assert-K6AuthInspect($Value, [string] $Case) {
    $expected = Get-K6AuthExpectation $Case
    if ($null -eq $Value -or $null -eq $Value.scenarios) { throw 'OPTIONS_INVALID' }
    Assert-K6Fields $Value.scenarios @($expected.Case)
    $scenario = $Value.scenarios.($expected.Case)
    Assert-K6Integer $scenario.vus 1; Assert-K6Integer $scenario.iterations 1
    # k6 v2.3.0 NullDuration.MarshalJSON delegates to time.Duration.String():
    # the source's 90s is serialized as 1m30s. Both closed forms mean exactly90s.
    $maxDurations = @($expected.MaxDuration)
    if ($Case -ceq 'auth-oauth-browser') { $maxDurations += '1m30s' }
    if ($scenario.executor -isnot [string] -or $scenario.executor -cne 'per-vu-iterations' -or
        $scenario.maxDuration -isnot [string] -or $scenario.maxDuration -cnotin $maxDurations -or
        $scenario.gracefulStop -isnot [string] -or $scenario.gracefulStop -cne $expected.GracefulStop) { throw 'OPTIONS_INVALID' }
    $thresholds = @('checks', 'case_started', 'case_completed')
    if ($expected.Phase -ceq 'HTTP') { $thresholds += 'http_reqs' }
    Assert-K6Fields $Value.thresholds $thresholds
    foreach ($name in $thresholds) {
        $threshold = if ($name -ceq 'checks') { 'rate==1' } else { 'count==1' }
        if ($Value.thresholds.$name -isnot [array] -or @($Value.thresholds.$name).Count -ne 1 -or
            @($Value.thresholds.$name)[0] -isnot [string] -or @($Value.thresholds.$name)[0] -cne $threshold) { throw 'OPTIONS_INVALID' }
    }
    if ($expected.Phase -ceq 'HTTP') {
        Assert-K6Integer $Value.maxRedirects 0
        if ($Value.discardResponseBodies -isnot [bool] -or -not $Value.discardResponseBodies) { throw 'OPTIONS_INVALID' }
    } elseif ($scenario.options.browser.type -isnot [string] -or $scenario.options.browser.type -cne 'chromium') {
        throw 'OPTIONS_INVALID'
    }
}

function Assert-K6AuthSummary($Value, [string] $RunId, [string] $Case, [int] $ExitCode) {
    $expected = Get-K6AuthExpectation $Case
    Assert-K6Fields $Value @('version', 'run_id', 'phase', 'cases', 'expected', 'started', 'completed', 'iterations', 'requests', 'checks')
    Assert-K6Integer $Value.version 1
    if ($ExitCode -ne 0 -or $RunId -cnotmatch '^[a-f0-9]{32}$' -or
        $Value.run_id -isnot [string] -or $Value.run_id -cne $RunId -or
        $Value.phase -isnot [string] -or $Value.phase -cne $expected.Phase) { throw 'SUMMARY_INVALID' }
    if ($Value.cases -isnot [array] -or $Value.cases.Count -ne 1 -or
        $Value.cases[0] -isnot [string] -or $Value.cases[0] -cne $expected.Case) { throw 'COMPLETENESS_INVALID' }
    foreach ($name in @('expected', 'started', 'completed', 'iterations')) { Assert-K6Integer $Value.$name 1 }
    Assert-K6Integer $Value.requests $expected.Requests
    Assert-K6Fields $Value.checks $expected.Checks
    foreach ($name in $expected.Checks) {
        Assert-K6Fields $Value.checks.$name @('passes', 'fails')
        Assert-K6Integer $Value.checks.$name.passes 1
        Assert-K6Integer $Value.checks.$name.fails 0
    }
}

function Read-K6AuthConfig([string] $Path, [string] $Root) {
    Assert-K6OutsideGit $Path
    $value = Read-K6PrivateJson $Path
    Assert-K6Fields $value @('version', 'profile', 'run_id', 'lease', 'cert_directory', 'tools', 'source_sha256', 'envoy_config_sha256')
    Assert-K6Integer $value.version 1
    if ($value.profile -cne 'local-full-oauth-proof' -or $value.run_id -cnotmatch '^[a-f0-9]{32}$' -or
        $value.envoy_config_sha256 -cnotmatch '^[a-f0-9]{64}$') { throw 'TARGET_REFUSED' }
    foreach ($name in @('lease', 'cert_directory')) { Assert-K6OutsideGit $value.$name }
    Assert-K6Fields $value.tools @('python', 'openssl', 'docker', 'kubectl')
    foreach ($name in @('python', 'openssl', 'docker', 'kubectl')) {
        $tool = $value.tools.$name
        Assert-K6Fields $tool @('path', 'sha256')
        Assert-K6OutsideGit $tool.path
        if ([IO.Path]::GetFileName($tool.path) -cne ($name + '.exe') -or $tool.sha256 -cnotmatch '^[a-f0-9]{64}$' -or
            [K6Windows.Native]::Hash($tool.path) -cne $tool.sha256) { throw 'PIN_INVALID' }
    }
    Assert-K6Fields $value.source_sha256 @('target')
    if ($value.source_sha256.target -cnotmatch '^[a-f0-9]{64}$' -or
        [K6Windows.Native]::Hash((Join-Path $Root 'ops\deploy\local_full_target.py')) -cne $value.source_sha256.target) { throw 'PIN_INVALID' }
    return $value
}

function New-K6AuthOpsEnvironment($Config, [string] $Directory) {
    # The anchor needs the operator's real home for its approved lease root and
    # Docker context, not the disposable Chrome home. No ambient target overrides.
    $environment = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    $system = [Environment]::GetFolderPath('Windows')
    $environment.Add('SystemRoot', $system); $environment.Add('WINDIR', $system)
    $paths = @('docker', 'kubectl', 'python', 'openssl') | ForEach-Object { [IO.Path]::GetDirectoryName($Config.tools.$_.path) }
    $environment.Add('PATH', (($paths + @((Join-Path $system 'System32'), (Join-Path $system 'System32\WindowsPowerShell\v1.0'))) -join ';'))
    foreach ($name in @('TEMP', 'TMP')) { $environment.Add($name, $Directory) }
    $homePath = [Environment]::GetFolderPath('UserProfile')
    $environment.Add('USERPROFILE', $homePath); $environment.Add('HOME', $homePath)
    $environment.Add('APPDATA', [Environment]::GetFolderPath('ApplicationData'))
    $environment.Add('LOCALAPPDATA', [Environment]::GetFolderPath('LocalApplicationData'))
    $environment.Add('PYTHONUTF8', '1')
    return ,$environment
}

function Save-K6AuthProcess($ChildResult, [string] $Directory) {
    $records = @($ChildResult.Identities | ForEach-Object { @{ pid = $_.Pid; created = $_.Created; image = $_.Image; ended = $_.Ended } })
    $value = @{ identities = $records; total = $ChildResult.Total; active = $ChildResult.Active; cleanup = $ChildResult.Cleanup; exit = $ChildResult.ExitCode; capture_complete = $ChildResult.CaptureComplete; overflow = $ChildResult.Overflow; reason = $ChildResult.Reason }
    [IO.File]::WriteAllText((Join-Path $Directory ('process-' + [Guid]::NewGuid().ToString('N') + '.json')), ($value | ConvertTo-Json -Depth 6 -Compress), [Text.UTF8Encoding]::new($false))
}

function Save-K6AuthCreated($Child, [string] $Directory) {
    $records = @($Child.Identities | ForEach-Object { @{ pid = $_.Pid; created = $_.Created; image = $_.Image; ended = $false } })
    $value = @{ status = 'created-suspended'; identities = $records; observed_utc = [DateTime]::UtcNow.ToString('o') }
    [IO.File]::WriteAllText((Join-Path $Directory ('process-' + [Guid]::NewGuid().ToString('N') + '.json')), ($value | ConvertTo-Json -Depth 6 -Compress), [Text.UTF8Encoding]::new($false))
}

function Invoke-K6AuthOps($Context, [string] $Action, [string] $Phase = 'good') {
    if ($Action -cnotin @('admit', 'snapshot', 'provider', 'bad', 'restore') -or
        $Phase -cnotin @('good', 'bad', 'restored', 'oauth-before', 'oauth-after')) { throw 'INPUT_INVALID' }
    $Context.Owner.AssertBound()
    if ([K6Windows.Native]::Hash($Context.Script) -cne $Context.ScriptHash -or
        [K6Windows.Native]::Hash($Context.Config.tools.python.path) -cne $Context.Config.tools.python.sha256) { throw 'PIN_INVALID' }
    $arguments = @('-I', '-B', $Context.Script, $Action, '--config', $Context.ConfigPath, '--directory', $Context.Directory, '--phase', $Phase)
    $state = $Context.State
    $state.Cleanup = $false; $state.Privacy = $false
    $child = [K6Windows.Child]::new($Context.Config.tools.python.path, $arguments, $Context.Environment, $Context.Directory)
    $state.Executed++
    try {
        Save-K6AuthCreated $child $Context.Directory
        $result = $child.Finish(600000, 20000, $null)
        $state.Cleanup = $result.Cleanup
        Save-K6AuthProcess $result $Context.Directory
        foreach ($stream in @('Stdout', 'Stderr')) {
            [IO.File]::WriteAllText((Join-Path $Context.Directory ('child-' + $state.Executed + '.' + $stream.ToLowerInvariant() + '.log')), $result.$stream, [Text.UTF8Encoding]::new($false))
        }
        if (-not $result.CaptureComplete -or $result.Overflow) { throw 'PRIVACY_REFUSED' }
        Assert-K6Privacy $result.Stdout $Context.Sentinels; Assert-K6Privacy $result.Stderr $Context.Sentinels
        $state.Privacy = $true
        if (-not $result.Cleanup -or -not $result.IdentityComplete) { throw 'CLEANUP_UNPROVEN' }
        if ($result.ExitCode -ne 0 -or $result.Reason -cne 'NONE') { throw 'AUTH_OPS_REFUSED' }
        $response = ConvertFrom-K6Json $result.Stdout
        Assert-K6Fields $response @('ok')
        if (-not (Test-K6BooleanField $response 'ok' $true)) { throw 'AUTH_OPS_REFUSED' }
    } finally {
        if (-not $state.Cleanup) { $child.Cancel(); [void]$child.Finish(1, 20000, $null) }
        $child.Dispose()
    }
}

function Invoke-K6AuthCase($Context, [string] $Case) {
    $expected = Get-K6AuthExpectation $Case
    $environment = New-K6AuthEnvironment $Context.Directory $Context.RunId $Context.Chrome.Exe $Context.Sentinels $Context.Chrome.Pin.version $Case
    $summary = Join-Path $Context.Directory ($Case + '.json')
    $environment['K6_SUMMARY_PATH'] = $summary
    if (Test-Path -LiteralPath $summary) { throw 'SUMMARY_INVALID' }
    $file = if ($expected.Phase -ceq 'Chromium') { 'oauth-browser.js' } else { 'envoy-san.js' }
    $arguments = @('run', '--quiet', '--address', '127.0.0.1:0', '--system-tags', 'name,check,scenario,group', (Join-Path $Context.Directory ('scenarios\auth\' + $file)))
    $chrome = $null; $timeout = 30000
    if ($expected.Phase -ceq 'Chromium') { Assert-K6PinnedTool $Context.Chrome; $chrome = $Context.Chrome.Exe; $timeout = 110000 }
    $child = Invoke-K6Child $Context.K6 $arguments $environment $Context.Directory $Context.Sentinels $timeout $Context.State $chrome
    $value = Read-K6PrivateJson $summary $Context.Owner
    try { Assert-K6AuthSummary $value $Context.RunId $Case $child.ExitCode } catch {
        # Reuse the already-retained private diagnostic name, never publish it.
        if ([IO.FileInfo]::new($summary).Length -le 65536) { [IO.File]::WriteAllBytes((Join-Path $Context.Directory 'parent-summary.json'), [IO.File]::ReadAllBytes($summary)) }
        throw
    }
}

function Read-K6AuthSnapshot($Context, [string] $Phase) {
    Invoke-K6AuthOps $Context 'snapshot' $Phase
    return Read-K6PrivateJson (Join-Path $Context.Directory ('auth-snapshot-' + $Phase + '.json')) $Context.Owner
}

function Get-K6AuthDelta($Before, $After, [string] $Name) {
    if ($Before.restart_epoch -ne $After.restart_epoch -or
        ($Before.identity | ConvertTo-Json -Depth 10 -Compress) -cne ($After.identity | ConvertTo-Json -Depth 10 -Compress)) { throw 'AUTH_CORRELATION_INVALID' }
    $left = $Before.stats.$Name; $right = $After.stats.$Name
    if (($left -isnot [int] -and $left -isnot [long]) -or ($right -isnot [int] -and $right -isnot [long]) -or
        $left -lt 0 -or $right -lt $left) { throw 'AUTH_CORRELATION_INVALID' }
    return ($right - $left)
}

function Close-K6AuthForward($Context, $Forward) {
    $Forward.Child.Cancel()
    $joined = $false; $privateCapture = $false
    try {
        $result = $Forward.Task.GetAwaiter().GetResult()
        $joined = $result.Cleanup -and $result.IdentityComplete
        Save-K6AuthProcess $result $Context.Directory
        if (-not $result.CaptureComplete -or $result.Overflow) { throw 'PRIVACY_REFUSED' }
        Assert-K6Privacy $result.Stdout $Context.Sentinels; Assert-K6Privacy $result.Stderr $Context.Sentinels
        $privateCapture = $true
        if (-not $joined) { throw 'CLEANUP_UNPROVEN' }
        if ($result.ExitCode -ne 0 -or $result.Reason -cnotin @('NONE', 'INTERRUPTED')) { throw 'AUTH_OPS_REFUSED' }
    } finally {
        if (-not $joined) { $Context.State.Cleanup = $false }
        if (-not $privateCapture) { $Context.State.Privacy = $false }
        $Forward.Child.Dispose()
    }
}

function Invoke-K6AuthCampaign($Context, $Projection) {
    $lockPath = $Context.Config.lease + '.local-full-lock'
    $lock = $null; $changed = $false; $restored = $false; $privacy = $true; $forward = $null
    try {
        # Same exclusive marker and lifetime as local-full-storage.operation_lock.
        $lock = [IO.FileStream]::new($lockPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
        $bytes = [Text.Encoding]::UTF8.GetBytes((@{ action = 'k6-auth'; pid = $PID; run_id = $Context.RunId } | ConvertTo-Json -Compress))
        $lock.Write($bytes, 0, $bytes.Length); $lock.Flush()
        Invoke-K6AuthOps $Context 'admit'
        Invoke-K6AuthCase $Context 'auth-envoy-san-good'
        $good = Read-K6AuthSnapshot $Context 'good'
        [void](Get-K6AuthDelta $good $good 'cluster.iam.ssl.handshake')
        if ($good.stats.'cluster.iam.ssl.handshake' -lt 1) { throw 'AUTH_CORRELATION_INVALID' }
        try {
            $changed = $true # An uncertain PATCH response may already have mutated.
            Invoke-K6AuthOps $Context 'bad' 'bad'
            $badAck = Read-K6PrivateJson (Join-Path $Context.Directory 'auth-ack-bad.json') $Context.Owner
            if (-not (Test-K6BooleanField $badAck 'patch' $true) -or -not (Test-K6BooleanField $badAck 'rollout' $true)) { throw 'AUTH_OPS_REFUSED' }
            Invoke-K6AuthCase $Context 'auth-envoy-san-bad'
            $bad = Read-K6AuthSnapshot $Context 'bad'
            Assert-K6Integer (Get-K6AuthDelta $good $bad 'cluster.iam.ssl.fail_verify_san') 1
            Assert-K6Integer (Get-K6AuthDelta $good $bad 'cluster.iam.ssl.fail_verify_error') 0
            $Projection.auth_evidence.san_delta = 1
        } finally {
            $privacy = $privacy -and $Context.State.Privacy
            if ($changed -and $Context.State.Cleanup) {
                Invoke-K6AuthOps $Context 'restore' 'restored'
                $ack = Read-K6PrivateJson (Join-Path $Context.Directory 'auth-ack-restored.json') $Context.Owner
                if (-not (Test-K6BooleanField $ack 'patch' $true) -or -not (Test-K6BooleanField $ack 'rollout' $true)) { throw 'AUTH_RESTORATION_FAILED' }
                Invoke-K6AuthCase $Context 'auth-envoy-san-restored'
                $restoredSnapshot = Read-K6AuthSnapshot $Context 'restored'
                [void](Get-K6AuthDelta $good $restoredSnapshot 'cluster.iam.ssl.fail_verify_san')
                [void](Get-K6AuthDelta $good $restoredSnapshot 'cluster.iam.ssl.fail_verify_error')
                if ((Get-K6AuthDelta $good $restoredSnapshot 'cluster.iam.ssl.handshake') -lt 1) { throw 'AUTH_RESTORATION_FAILED' }
                $restored = $true
                $Projection.auth_evidence.restoration = 'PASS'
            }
        }
        # Keep the negative oracle separate from successful physical restoration.
        Assert-K6Integer (Get-K6AuthDelta $good $restoredSnapshot 'cluster.iam.ssl.fail_verify_san') 1
        Assert-K6Integer (Get-K6AuthDelta $good $restoredSnapshot 'cluster.iam.ssl.fail_verify_error') 0
        $Projection.certificate = 'PASS'
        $before = Read-K6AuthSnapshot $Context 'oauth-before'
        $binding = Read-K6PrivateJson (Join-Path $Context.Directory 'auth-binding.json') $Context.Owner
        $target = Join-Path $Context.Root 'ops\deploy\local_full_target.py'
        if ([K6Windows.Native]::Hash($target) -cne $Context.Config.source_sha256.target -or
            [K6Windows.Native]::Hash($Context.Config.tools.python.path) -cne $Context.Config.tools.python.sha256 -or
            [K6Windows.Native]::Hash($Context.Script) -cne $Context.ScriptHash) { throw 'PIN_INVALID' }
        $arguments = @('-I', '-B', $Context.Script, 'forward', '--config', $Context.ConfigPath, '--directory', $Context.Directory)
        $priorCleanup = $Context.State.Cleanup
        $Context.State.Cleanup = $false
        $child = [K6Windows.Child]::new($Context.Config.tools.python.path, $arguments, $Context.Environment, $Context.Directory)
        $Context.State.Executed++
        try {
            Save-K6AuthCreated $child $Context.Directory
            $forward = @{ Child = $child; Task = $child.FinishAsync(300000, 25000, $null) }
        } catch {
            $child.Cancel(); [void]$child.Finish(1, 25000, $null); $child.Dispose()
            throw
        }
        # The forward's retained handle now has its own mandatory finally below.
        $Context.State.Cleanup = $priorCleanup
        try {
            # Await the existing marker and fixed port, never adopt an occupied one.
            $deadline = [DateTime]::UtcNow.AddSeconds(20)
            $ready = $false
            while ([DateTime]::UtcNow -lt $deadline -and -not $forward.Task.IsCompleted) {
                if (Test-Path -LiteralPath ($Context.Config.lease + '.oauth-forward')) {
                    $markerReady = $false
                    try {
                        $marker = Read-K6PrivateJson ($Context.Config.lease + '.oauth-forward')
                        Assert-K6Fields $marker @('task_id', 'context', 'service_uid', 'namespace_uid', 'pid', 'bind', 'mapping', 'ownership')
                        $markerReady = $marker.task_id -ceq $binding.task_id -and $marker.context -ceq 'kind-aiforall-local-full' -and
                            $marker.service_uid -ceq $binding.provider_service_uid -and $marker.namespace_uid -ceq $binding.attestation.namespace_uid -and
                            $marker.bind -ceq '127.0.0.1' -and $marker.mapping -ceq '18081:8080' -and $marker.ownership -ceq 'live-child-handle-only'
                    } catch { } # The exclusive marker is empty until guarded child creation.
                    $socket = [Net.Sockets.TcpClient]::new()
                    try {
                        if ($markerReady) { $socket.Connect('127.0.0.1', 18081); $ready = $socket.Connected }
                    } catch { } finally { $socket.Dispose() }
                    if ($ready) { break }
                }
                [Threading.Thread]::Sleep(100)
            }
            if (-not $ready) { throw 'AUTH_OPS_REFUSED' }
            Invoke-K6AuthOps $Context 'provider' 'oauth-before'
            Invoke-K6AuthCase $Context 'auth-oauth-browser'
            Invoke-K6AuthOps $Context 'provider' 'oauth-after'
        } finally {
            $privacy = $privacy -and $Context.State.Privacy
            $ownedForward = $forward
            $forward = $null
            Close-K6AuthForward $Context $ownedForward
        }
        # The OAuth forward is closed before any Admin snapshot.
        $after = Read-K6AuthSnapshot $Context 'oauth-after'
        Assert-K6Integer (Get-K6AuthDelta $before $after 'cluster.iam.upstream_rq_total') 7
        Assert-K6Integer (Get-K6AuthDelta $before $after 'cluster.ext_authz.upstream_rq_total') 4
        Assert-K6Integer (Get-K6AuthDelta $before $after 'cluster.iam.upstream_rq_5xx') 0
        foreach ($name in @('iam', 'ext_authz', 'hive', 'telegraph', 'manifesto')) {
            foreach ($counter in @('upstream_cx_connect_fail', 'upstream_cx_connect_timeout', 'ssl.fail_verify_san', 'ssl.fail_verify_error')) {
                Assert-K6Integer (Get-K6AuthDelta $before $after ('cluster.' + $name + '.' + $counter)) 0
            }
            if ($name -cin @('hive', 'telegraph', 'manifesto')) { Assert-K6Integer (Get-K6AuthDelta $before $after ('cluster.' + $name + '.upstream_rq_total')) 0 }
        }
        $providerBefore = Read-K6PrivateJson (Join-Path $Context.Directory 'auth-provider-oauth-before.json') $Context.Owner
        $providerAfter = Read-K6PrivateJson (Join-Path $Context.Directory 'auth-provider-oauth-after.json') $Context.Owner
        if ($providerBefore.run_id -cne $Context.RunId -or $providerAfter.run_id -cne $Context.RunId) { throw 'AUTH_CORRELATION_INVALID' }
        foreach ($name in @('authorize', 'token', 'profile', 'pkce_validated')) {
            Assert-K6Integer $providerBefore.$name 0
            Assert-K6Integer $providerAfter.$name 3
        }
        $Projection.auth_evidence.iam_delta = 7; $Projection.auth_evidence.ext_authz_delta = 4; $Projection.auth_evidence.provider_delta = 3
        $Projection.correlation = 'PASS'; $Projection.auth = 'PASS'; $Projection.http = 'PASS'; $Projection.chromium = 'PASS'; $Projection.tools = 'PASS'
        $Projection.versions.chromium = $Context.Chrome.Pin.version
        $Projection.browser_version_evidence = 'K6_API_CACHED_GETVERSION_EQUALITY'
    } finally {
        $Context.State.Privacy = $Context.State.Privacy -and $privacy
        if ($null -ne $forward) { Close-K6AuthForward $Context $forward }
        if ($changed -and -not $restored) { $Context.State.Cleanup = $false; $Projection.auth_evidence.restoration = 'FAIL' }
        if ($null -ne $lock) {
            $lock.Dispose()
            if ($Context.State.Cleanup) { [IO.File]::Delete($lockPath) }
        }
    }
}
