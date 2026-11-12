# Closed helpers for this campaign, not a deployment/profile framework.
Set-StrictMode -Version Latest

function Save-NativeJson($Value, [string]$Name) {
    [IO.File]::WriteAllText((Join-Path $Directory $Name), ($Value | ConvertTo-Json -Depth 80), $utf8)
}
function Save-NativeText([string]$Text, [string]$Name) {
    [IO.File]::WriteAllText((Join-Path $Directory $Name), $Text, $utf8)
}
function Read-NativeJson([string]$Name) { ConvertFrom-Json ([IO.File]::ReadAllText((Join-Path $Directory $Name))) }
function Save-NativeState { Save-NativeJson $script:state 'state.json' }
function Pin-NativeTool([string]$Name) {
    $pin = $cfg.tools.$Name
    if (-not [IO.Path]::IsPathRooted($pin.path) -or $pin.sha256 -cnotmatch '^[a-f0-9]{64}$' -or
        (Get-FileHash -LiteralPath $pin.path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $pin.sha256) { throw 'TOOL_PIN_INVALID' }
    if ($Name -ceq 'chromium') {
        if (-not $pin.PSObject.Properties['bundle_manifest'] -or -not $pin.PSObject.Properties['bundle_sha256']) { throw 'COMPLETE_CHROMIUM_BUNDLE_REQUIRED' }
        if (-not [IO.Path]::IsPathRooted($pin.bundle_manifest) -or $pin.bundle_sha256 -cnotmatch '^[a-f0-9]{64}$' -or
            (Get-FileHash -LiteralPath $pin.bundle_manifest -Algorithm SHA256).Hash.ToLowerInvariant() -cne $pin.bundle_sha256) { throw 'CHROMIUM_BUNDLE_PIN_INVALID' }
        $bundle = ConvertFrom-Json ([IO.File]::ReadAllText($pin.bundle_manifest))
        $lock = (ConvertFrom-Json ([IO.File]::ReadAllText((Join-Path $PSScriptRoot '../config/tools.windows.lock.json')))).chromium
        if ($bundle.version -ne 1 -or $bundle.archive_sha256 -cne $lock.archive_sha256 -or $pin.sha256 -cne $lock.executable_sha256 -or -not $bundle.files.Count) { throw 'CHROMIUM_DISTRIBUTION_INVALID' }
        $base = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($pin.bundle_manifest)) + '\'
        foreach ($file in $bundle.files) {
            $path = [IO.Path]::GetFullPath((Join-Path $base $file.path))
            if (-not $path.StartsWith($base, [StringComparison]::OrdinalIgnoreCase) -or $file.sha256 -cnotmatch '^[a-f0-9]{64}$' -or
                (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.sha256) { throw 'CHROMIUM_BUNDLE_MEMBER_INVALID' }
        }
    }
    # Child attests the Windows image path; normalize separators before comparing
    # it with QueryFullProcessImageName (public input JSON may use forward slashes).
    return [IO.Path]::GetFullPath($pin.path)
}
function New-NativeEnvironment {
    $e = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($n in @('SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'APPDATA', 'LOCALAPPDATA')) {
        $v = [Environment]::GetEnvironmentVariable($n)
        if ($v) { $e.Add($n, $v) }
    }
    $e.Add('PATH', (Join-Path ([Environment]::GetFolderPath('Windows')) 'System32'))
    return ,$e
}
function Invoke-NativeTool([string]$Name, [string[]]$Arguments, [int]$Timeout = 60000, [int]$ExpectedExit = 0) {
    $child = [K6Windows.Child]::new((Pin-NativeTool $Name), $Arguments, (New-NativeEnvironment), $Directory)
    try {
        $r = $child.Finish($Timeout, 5000, $null)
        $script:sequence++
        Save-NativeText $r.Stderr ($Action + '-cli-' + $script:sequence + '.stderr.log')
        if (-not $r.Cleanup -or -not $r.CaptureComplete -or $r.Overflow -or $r.ExitCode -ne $ExpectedExit -or $r.Reason -cne 'NONE') {
            throw ('NATIVE_' + $Name.ToUpperInvariant() + '_FAILED')
        }
        return $r.Stdout.Trim()
    } finally { $child.Dispose() }
}
function K([string[]]$Arguments, [int]$Timeout = 60000) {
    Invoke-NativeTool 'kubectl' ($prefix + @('--request-timeout=30s') + $Arguments) $Timeout
}
function Get-NativePod([string]$Role) {
    $name = $state.name + '-' + $Role
    $receipt = @($state.resources | Where-Object { $_.kind -ceq 'Deployment' -and $_.name -ceq $name })
    if ($receipt.Count -ne 1) { throw 'MISSING_DEPLOYMENT_RECEIPT' }
    $d = ConvertFrom-Json (K @('get', 'deployment', $name, '-o', 'json'))
    if ($d.metadata.uid -cne $receipt[0].uid) { throw 'DEPLOYMENT_REPLACED' }
    $pods = ConvertFrom-Json (K @('get', 'pods', '-l', ('aiforall.dev/k6-run=' + $state.run_id + ',aiforall.dev/k6-role=' + $Role), '-o', 'json'))
    $live = @($pods.items | Where-Object { -not $_.metadata.PSObject.Properties['deletionTimestamp'] })
    if ($live.Count -ne 1 -or $live[0].status.phase -cne 'Running') { throw 'ONE_RUNNING_FIXTURE_POD_REQUIRED' }
    $p = $live[0]
    $owner = $p.metadata.ownerReferences[0]
    $rs = ConvertFrom-Json (K @('get', 'replicaset', $owner.name, '-o', 'json'))
    if ($owner.kind -cne 'ReplicaSet' -or $owner.uid -cne $rs.metadata.uid -or
        $rs.metadata.ownerReferences[0].uid -cne $receipt[0].uid) { throw 'POD_OWNER_MISMATCH' }
    return $p
}
function Get-NativeIdentity([string]$Role) {
    $p = Get-NativePod $Role
    $s = @($p.status.containerStatuses | Where-Object name -ceq $Role)
    if ($s.Count -ne 1 -or -not $s[0].ready) { throw 'FIXTURE_NOT_READY' }
    return [ordered]@{ pod = $p.metadata.name; uid = $p.metadata.uid; container = $s[0].containerID; image = $s[0].imageID; restarts = $s[0].restartCount }
}
function Assert-NativeTarget {
    if ($env:DOCKER_HOST -or $env:KUBECONFIG) { throw 'AMBIENT_TARGET_OVERRIDE_REFUSED' }
    $ids = (Invoke-NativeTool 'docker' @('--context', 'desktop-linux', 'ps', '-aq', '--no-trunc', '--filter', 'label=io.x-k8s.kind.cluster=aiforall-local-full')) -split '\r?\n'
    if ($ids.Count -ne 3) { throw 'THREE_EXISTING_NODES_REQUIRED' }
    $roles = @(); $control = $null
    foreach ($id in $ids) {
        if ($id -cnotmatch '^[a-f0-9]{64}$') { throw 'INVALID_NODE_ID' }
        $node = ConvertFrom-Json (Invoke-NativeTool 'docker' @('--context', 'desktop-linux', 'inspect', '--format', '{"labels":{{json .Config.Labels}},"name":{{json .Name}},"status":{{json .State.Status}}}', $id))
        $role = $node.labels.'io.x-k8s.kind.role'
        if ($node.status -cne 'running' -or $node.labels.'io.x-k8s.kind.cluster' -cne 'aiforall-local-full' -or
            $node.name -cnotin @('/aiforall-local-full-control-plane', '/aiforall-local-full-worker', '/aiforall-local-full-worker2')) { throw 'FOREIGN_OR_STOPPED_NODE' }
        $roles += $role
        if ($role -ceq 'control-plane') { $control = $id }
    }
    if (@($roles | Where-Object { $_ -ceq 'control-plane' }).Count -ne 1 -or @($roles | Where-Object { $_ -ceq 'worker' }).Count -ne 2) { throw 'INVALID_TOPOLOGY' }
    $server = Invoke-NativeTool 'kubectl' @('--kubeconfig', $cfg.kubeconfig, '--context', 'kind-aiforall-local-full', 'config', 'view', '--minify', '-o', 'jsonpath={.clusters[0].cluster.server}')
    if ($server -cnotmatch '^https://127\.0\.0\.1:[0-9]+$' -or
        (Invoke-NativeTool 'docker' @('--context', 'desktop-linux', 'port', $control, '6443/tcp')) -cne ('127.0.0.1:' + ([Uri]$server).Port)) { throw 'API_BINDING_MISMATCH' }
    $ns = K @('get', 'namespace', 'aiforall-local-full', '-o', 'jsonpath={.metadata.uid}')
    if ($ns -cnotmatch '^[a-f0-9-]{36}$') { throw 'NAMESPACE_UID_INVALID' }
    $nodes = ($ids | Sort-Object) -join ','
    if ($state.target -and ($state.target.namespace_uid -cne $ns -or $state.target.nodes -cne $nodes)) { throw 'TARGET_REPLACED' }
    return @{ namespace_uid = $ns; nodes = $nodes; server = $server }
}
function Create-NativeObject($Object) {
    $kind = $Object.kind; $name = $Object.metadata.name
    if ($name -cnotmatch ('^' + [regex]::Escape($state.name) + '-[a-z-]+$') -or
        $Object.metadata.labels.'aiforall.dev/k6-run' -cne $state.run_id) { throw 'OBJECT_SCOPE_INVALID' }
    Save-NativeJson $Object 'create.json'
    # create (never apply) refuses collisions. Output is only the returned UID.
    $uid = K @('create', '-f', (Join-Path $Directory 'create.json'), '-o', 'jsonpath={.metadata.uid}')
    if ($uid -cnotmatch '^[a-f0-9-]{36}$') { throw 'CREATE_RECEIPT_UNCERTAIN' }
    $state.resources += @{kind = $kind; name = $name; uid = $uid; deleted = $false}
    Save-NativeState
}
function Create-NativeSecret([string]$Suffix, $Data) {
    $encoded = @{}
    foreach ($key in $Data.Keys) { $encoded[$key] = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Data[$key])) }
    Create-NativeObject @{apiVersion = 'v1'; kind = 'Secret'; metadata = @{name = $state.name + '-' + $Suffix; labels = @{'aiforall.dev/k6-run' = $state.run_id}}; type = 'Opaque'; data = $encoded}
}
function Remove-NativeObject($Receipt) {
    if ($Receipt.deleted) { return }
    $group = switch ($Receipt.kind) { 'Deployment' { 'apis/apps/v1' }; 'Secret' { 'api/v1' }; 'ConfigMap' { 'api/v1' }; 'Service' { 'api/v1' }; default { throw 'DELETE_KIND_REFUSED' } }
    $plural = switch ($Receipt.kind) { 'Deployment' { 'deployments' }; 'Secret' { 'secrets' }; 'ConfigMap' { 'configmaps' }; 'Service' { 'services' } }
    # Only a Deployment owns writers/children. Leaf objects use immediate
    # background deletion; in particular never GET a Secret via kubectl wait.
    $propagation = if ($Receipt.kind -ceq 'Deployment') { 'Foreground' } else { 'Background' }
    Save-NativeJson @{apiVersion = 'v1'; kind = 'DeleteOptions'; preconditions = @{uid = $Receipt.uid}; propagationPolicy = $propagation} 'delete.json'
    $null = K @('delete', '--raw', ('/' + $group + '/namespaces/aiforall-local-full/' + $plural + '/' + $Receipt.name), '-f', (Join-Path $Directory 'delete.json'))
    if ($Receipt.kind -cne 'Secret') { $null = K @('wait', '--for=delete', ($plural + '/' + $Receipt.name), '--timeout=90s') 120000 }
    $Receipt.deleted = $true; Save-NativeState
}
function Invoke-NativeSql([string[]]$Statements, [string]$Database = 'postgres') {
    $pod = ConvertFrom-Json (K @('get', 'pod', $state.postgres.pod, '-o', 'json'))
    if ($pod.metadata.uid -cne $state.postgres.uid) { throw 'POSTGRES_POD_REPLACED' }
    # The nominal readiness probe sets PGHOSTADDR=127.0.0.1. libpq would keep
    # that TCP address even with an explicit socket host. Remove only these
    # inherited selectors for this invocation; no pg_hba or server change.
    $a = @('exec', $state.postgres.pod, '-c', 'postgres', '--', 'env', '-u', 'PGHOSTADDR', '-u', 'PGHOST', 'psql', '-X', '-A', '-t', '-w', '-v', 'ON_ERROR_STOP=1', '-h', '/var/run/postgresql', '-U', 'postgres', '-d', $Database)
    foreach ($s in $Statements) { $a += @('-c', $s) }
    K $a
}
function Get-NativeHttpJson([string]$Url) {
    $handler = [Net.Http.HttpClientHandler]::new(); $handler.UseProxy = $false; $handler.AllowAutoRedirect = $false
    $client = [Net.Http.HttpClient]::new($handler); $client.Timeout = [TimeSpan]::FromSeconds(10); $client.MaxResponseContentBufferSize = 1048576
    try { ConvertFrom-Json ($client.GetStringAsync($Url).GetAwaiter().GetResult()) } finally { $client.Dispose(); $handler.Dispose() }
}
function Start-NativeForward([string]$Role, [int]$Port, [int]$Destination) {
    $pod = Get-NativePod $Role
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, $Port)
    try { $listener.Start() } finally { $listener.Stop() }
    $child = [K6Windows.Child]::new((Pin-NativeTool 'kubectl'), ($prefix + @('port-forward', '--address', '127.0.0.1', ('pod/' + $pod.metadata.name), ($Port.ToString() + ':' + $Destination))), (New-NativeEnvironment), $Directory)
    $forward = @{ child = $child; task = $child.FinishAsync(600000, 5000, $null); role = $Role; port = $Port }
    $script:forwards += $forward
    $clock = [Diagnostics.Stopwatch]::StartNew()
    do {
        $connections = @(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
        if ($connections.Count -eq 1 -and $connections[0].OwningProcess -in @($child.Identities.Pid)) { return }
        if ($forward.task.IsCompleted) { throw 'FORWARD_EXITED' }
        Start-Sleep -Milliseconds 100
    } while ($clock.Elapsed.TotalSeconds -lt 15)
    throw 'FORWARD_BINDING_UNPROVEN'
}
function Assert-NativeForwards {
    if ($script:forwards.Count -ne 3) { throw 'THREE_OWNED_FORWARDS_REQUIRED' }
    foreach ($f in $script:forwards) {
        if ($f.task.IsCompleted) { throw 'FORWARD_EXITED_BEFORE_STIMULUS' }
        $listeners = @(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $f.port -State Listen -ErrorAction SilentlyContinue)
        if ($listeners.Count -ne 1 -or $listeners[0].OwningProcess -notin @($f.child.Identities.Pid)) { throw 'FORWARD_LISTENER_OWNERSHIP_LOST' }
    }
}
