# Opt-in API profile only. No provisioning or application mutation in this helper.
Set-StrictMode -Version Latest
$script:linkReceiptHash = $null

function Assert-NativeLinkReceipt {
    $path = Join-Path $Directory 'link-setup-private.json'
    $item = Get-Item -LiteralPath $path -ErrorAction Stop
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -or $item.Length -gt 65536) { throw 'LINK_SETUP_RECEIPT_INVALID' }
    $attempt = Get-Item -LiteralPath (Join-Path $Directory 'auth-link-conflict-setup.json.attempt')
    if ($item.LastWriteTimeUtc -lt $attempt.LastWriteTimeUtc -or $item.LastWriteTimeUtc -gt [DateTime]::UtcNow) { throw 'LINK_SETUP_RECEIPT_NOT_FRESH' }
    $acl = Get-Acl -LiteralPath $path
    $owner = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -cne $owner) { throw 'LINK_SETUP_RECEIPT_OWNER_INVALID' }
    foreach ($rule in $acl.Access) {
        if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value -notin @($owner, 'S-1-5-18')) { throw 'LINK_SETUP_RECEIPT_NOT_PRIVATE' }
    }
    $receipt = Read-NativeJson 'link-setup-private.json'
    Assert-K6Fields $receipt @('version', 'run_id', 'access_token')
    Assert-K6Integer $receipt.version 1
    if ($receipt.run_id -cne $state.run_id -or $receipt.access_token -isnot [string] -or $receipt.access_token -cnotmatch '^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$') { throw 'LINK_SETUP_RECEIPT_INVALID' }
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    if ($script:linkReceiptHash -and $hash -cne $script:linkReceiptHash) { throw 'LINK_SETUP_RECEIPT_CHANGED' }
    $script:linkReceiptHash = $hash
    # Deliberately do not return the bearer, write it elsewhere, or print it.
}

function Get-NativeLinkState([string]$Phase) {
    if ($state.database -cnotmatch '^k6_[a-f0-9]{32}$' -or -not $state.database_oid) { throw 'LINK_DATABASE_RECEIPT_REQUIRED' }
    $tables = @('users', 'user_emails', 'provider_tokens', 'identities', 'refresh_tokens', 'rustycog_outbox_events')
    $fields = foreach ($table in $tables) {
        # Full rows (all address flags and sign-in material), ordered inside the
        # database; only counts and SHA-256 fingerprints cross the SQL boundary.
        "'$table',(SELECT json_build_object('count',count(*),'sha256',encode(sha256(convert_to(COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text)::text,'[]'),'UTF8')),'hex')) FROM $table t)"
    }
    $sql = "SELECT json_build_object('database',current_database(),'oid',(SELECT oid FROM pg_database WHERE datname=current_database()),'tables',json_build_object(" + ($fields -join ',') + "),'transactions',(SELECT json_build_object('total',count(*),'consumed',count(*) FILTER (WHERE consumed_at IS NOT NULL),'pending',count(*) FILTER (WHERE consumed_at IS NULL),'secrets',count(*) FILTER (WHERE consumed_at IS NOT NULL AND (pkce_verifier IS NOT NULL OR authentication_nonce IS NOT NULL))) FROM oauth_transactions))"
    $value = ConvertFrom-Json (Invoke-NativeSql @($sql) $state.database)
    if ($value.database -cne $state.database -or [long]$value.oid -ne [long]$state.database_oid) { throw 'LINK_DATABASE_IDENTITY_CHANGED' }
    Save-NativeJson $value ('link-state-' + $Phase + '.json')
    return $value
}

function Assert-NativeLinkRefusalState($Before, $After) {
    if ($Before.database -cne $After.database -or $Before.oid -ne $After.oid) { throw 'LINK_DATABASE_IDENTITY_CHANGED' }
    foreach ($table in @('users', 'user_emails', 'provider_tokens', 'identities', 'refresh_tokens', 'rustycog_outbox_events')) {
        if ($Before.tables.$table.sha256 -cnotmatch '^[a-f0-9]{64}$' -or
            $Before.tables.$table.sha256 -cne $After.tables.$table.sha256 -or
            $Before.tables.$table.count -ne $After.tables.$table.count) { throw 'LINK_REFUSAL_CHANGED_ACCOUNT_STATE' }
    }
    if (($After.transactions.total - $Before.transactions.total) -ne 1 -or
        ($After.transactions.consumed - $Before.transactions.consumed) -ne 1 -or
        $Before.transactions.pending -ne 0 -or $After.transactions.pending -ne 0 -or
        $Before.transactions.secrets -ne 0 -or $After.transactions.secrets -ne 0) { throw 'LINK_TRANSACTION_CONSUMPTION_UNPROVEN' }
}

function Assert-NativeLinkTraffic($Before, $After, $ProviderBefore, $ProviderAfter, [int]$Iam, [int]$Provider, [switch]$ClassificationRed, [string]$Phase = 'setup') {
    if ($Phase -cnotin @('setup', 'email', 'identity') -or ($ClassificationRed -and $Phase -ceq 'setup')) { throw 'LINK_TRAFFIC_PHASE_INVALID' }
    foreach ($name in @('authorize', 'token', 'profile', 'pkce_validated')) {
        if ($ProviderBefore.run_id -cne $state.run_id -or $ProviderAfter.run_id -cne $state.run_id -or
            ($ProviderAfter.$name - $ProviderBefore.$name) -ne $Provider) { throw 'LINK_PROVIDER_CORRELATION_FAILED' }
    }
    $serverErrors = Get-K6AuthDelta $Before $After 'cluster.iam.upstream_rq_5xx'
    $expectedErrors = if ($ClassificationRed -and $Phase -ceq 'identity') { 1 } else { 0 }
    if ((Get-K6AuthDelta $Before $After 'cluster.iam.upstream_rq_total') -ne $Iam -or
        $serverErrors -ne $expectedErrors) { throw 'LINK_EDGE_CARDINALITY_FAILED' }
    foreach ($cluster in @('iam', 'ext_authz', 'hive', 'telegraph', 'manifesto')) {
        foreach ($counter in @('upstream_cx_connect_fail', 'upstream_cx_connect_timeout', 'ssl.fail_verify_san', 'ssl.fail_verify_error')) {
            if ((Get-K6AuthDelta $Before $After ('cluster.' + $cluster + '.' + $counter)) -ne 0) { throw 'LINK_TRANSPORT_FAILED' }
        }
    }
}

function Invoke-NativeLinkConflicts {
    try { Invoke-NativeLinkConflictPhases }
    finally {
        $path = Join-Path $Directory 'link-setup-private.json'
        if (Test-Path -LiteralPath $path) {
            $item = Get-Item -LiteralPath $path
            if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'LINK_RECEIPT_CLEANUP_UNPROVEN' }
            Remove-Item -LiteralPath $path
            if (Test-Path -LiteralPath $path) { throw 'LINK_RECEIPT_CLEANUP_UNPROVEN' }
        }
        $script:linkReceiptHash = $null
    }
}

function Invoke-NativeLinkConflictPhases {
    # Readiness is checked before snapshots/business writes, so this parent-only
    # observation cannot be mistaken for scenario traffic.
    $ready = Get-NativeHttpJson 'https://127.0.0.1:18443/iam/ready'
    if ($ready.checks.queue_publisher.status -cne 'disabled') { throw 'LINK_EVENT_TRANSPORT_NOT_DISABLED' }
    $fresh = Get-NativeLinkState 'fresh'
    foreach ($table in $fresh.tables.PSObject.Properties) {
        if ($table.Value.count -ne 0) { throw 'LINK_DATABASE_NOT_FRESH' }
    }
    if ($fresh.transactions.total -ne 0) { throw 'LINK_DATABASE_NOT_FRESH' }
    if (Test-Path -LiteralPath (Join-Path $Directory 'link-setup-private.json')) { throw 'LINK_SETUP_RECEIPT_ALREADY_EXISTS' }
    Save-NativeJson @{run_id = $state.run_id; database_oid = $state.database_oid; fresh = $true; event_transport = 'disabled'} 'link-isolation.json'
    $edgeBefore = Snapshot 'link-setup-before'
    $providerBefore = Get-NativeHttpJson 'https://127.0.0.1:18445/observations'
    Invoke-Scenario 'auth-link-conflict-setup'
    Assert-NativeLinkReceipt
    $setup = Get-NativeLinkState 'setup-after'
    if ($setup.tables.users.count -ne 2 -or $setup.transactions.pending -ne 0 -or $setup.transactions.secrets -ne 0) { throw 'LINK_SETUP_STATE_INVALID' }
    $edgeAfter = Snapshot 'link-setup-after'
    $providerAfter = Get-NativeHttpJson 'https://127.0.0.1:18445/observations'
    Assert-NativeLinkTraffic $edgeBefore $edgeAfter $providerBefore $providerAfter 6 2
    $failed = @()
    foreach ($phase in @('email', 'identity')) {
        Assert-NativeLinkReceipt
        $before = Get-NativeLinkState ($phase + '-before')
        $edgeBefore = Snapshot ('link-' + $phase + '-before')
        $providerBefore = Get-NativeHttpJson 'https://127.0.0.1:18445/observations'
        $classificationFailure = $false
        try { Invoke-Scenario ('auth-link-conflict-' + $phase) }
        catch {
            # Continue only after proving a complete, bounded classification-only
            # red. Process, arrangement and missing-summary failures still stop.
            $value = Read-NativeJson ('auth-link-conflict-' + $phase + '.json')
            $process = Read-NativeJson ('auth-link-conflict-' + $phase + '.process.json')
            Assert-NativeLinkClassificationRed $value $process ('auth-link-conflict-' + $phase)
            $classificationFailure = $true
        } finally {
            # Even failed child classification must leave a private post-state.
            $after = Get-NativeLinkState ($phase + '-after')
        }
        Assert-NativeLinkRefusalState $before $after
        $edgeAfter = Snapshot ('link-' + $phase + '-after')
        $providerAfter = Get-NativeHttpJson 'https://127.0.0.1:18445/observations'
        # A baseline identity mapper may return 500; preserve its counters and
        # classify separately, never turn that red into a transport failure.
        Assert-NativeLinkTraffic $edgeBefore $edgeAfter $providerBefore $providerAfter 2 1 -ClassificationRed:$classificationFailure -Phase $phase
        if ($classificationFailure) { $failed += $phase }
        Save-NativeJson @{run_id = $state.run_id; phase = $phase; classification = $(if ($classificationFailure) {'FAIL'} else {'PASS'}); state_unchanged = $true; transaction_consumed = $true; provider_before = $providerBefore; provider_after = $providerAfter} ('link-' + $phase + '-evidence.json')
    }
    Save-NativeJson @{run_id = $state.run_id; profile = 'iam-link-conflicts'; phases = @('setup', 'email', 'identity'); failed_classifications = $failed; isolation = 'PASS'; state_unchanged = $true} 'evidence.json'
    if ($failed.Count) { throw 'LINK_CONFLICT_CLASSIFICATION_FAILED' }
}

function Assert-NativeLinkClassificationRed($Value, $Process, [string]$Case) {
    if ($Process.exit -ne 99 -or -not $Process.cleanup -or -not $Process.identities_complete -or
        -not $Process.capture_complete -or $Process.overflow -or $Process.reason -cne 'NONE') { throw 'LINK_CHILD_FAILURE_NOT_CLASSIFICATION' }
    $clone = ConvertFrom-Json ($Value | ConvertTo-Json -Depth 20)
    if ($Case -cnotin @('auth-link-conflict-email', 'auth-link-conflict-identity')) { throw 'LINK_CHILD_FAILURE_NOT_CLASSIFICATION' }
    foreach ($name in @('conflict-http-status', 'conflict-error-code', 'conflict-envelope')) {
        $check = $clone.checks.$name
        $expectedFail = $Case -ceq 'auth-link-conflict-identity' -or $name -cne 'conflict-http-status'
        Assert-K6Fields $check @('passes', 'fails')
        Assert-K6Integer $check.passes $(if ($expectedFail) { 0 } else { 1 })
        Assert-K6Integer $check.fails $(if ($expectedFail) { 1 } else { 0 })
        $check.passes = 1; $check.fails = 0
    }
    # Validate an isolated clone only to recognize the precise historical red;
    # the original summary/process remain unchanged and campaign still fails.
    Assert-K6AuthSummary $clone $state.run_id $Case 0
}
