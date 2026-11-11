# Small, closed Windows entrypoint. Never starts/stops/recreates the lab.
param(
    [Parameter(Mandatory = $true)][ValidateSet('prepare', 'run', 'clean')][string]$Action,
    [Parameter(Mandatory = $true)][string]$Directory,
    [string]$Config
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../../..'))
$context = 'kind-aiforall-local-full'
$namespace = 'aiforall-local-full'
$utf8 = [Text.UTF8Encoding]::new($false)
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User

function AbsoluteFile([string]$Path) {
    if (-not [IO.Path]::IsPathRooted($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw 'Explicit existing absolute file required.' }
    return [IO.Path]::GetFullPath($Path)
}
function Save($Value, [string]$Name) {
    [IO.File]::WriteAllText((Join-Path $Directory $Name), (ConvertTo-Json $Value -Depth 12), $utf8)
}
function PrivateDirectory {
    $item = Get-Item -LiteralPath $Directory
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Campaign reparse points are not supported.' }
    $acl = Get-Acl -LiteralPath $Directory
    if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { throw 'Foreign campaign owner.' }
    foreach ($rule in $acl.Access) {
        $id = $rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
        if ($rule.AccessControlType -eq 'Allow' -and $id -notin @($sid.Value, 'S-1-5-18')) { throw 'Campaign directory is not private.' }
    }
}
function Pin($Tool) {
    $path = AbsoluteFile $Tool.path
    if ($Tool.sha256 -cnotmatch '^[a-f0-9]{64}$' -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $Tool.sha256) {
        throw 'Tool pin mismatch; no acquisition or fallback.'
    }
    return $path
}
function Native([string]$File, [string[]]$Arguments) {
    $old = $ErrorActionPreference
    try { $ErrorActionPreference = 'Continue'; $output = & $File @Arguments 2>$null; $code = $LASTEXITCODE }
    finally { $ErrorActionPreference = $old }
    if ($code -ne 0) { throw 'Native preflight failed; no automatic repair (raw diagnostic suppressed).' }
    return $output
}
function K([string[]]$Arguments) { Native $kubectl ($prefix + @('--request-timeout=10s') + $Arguments) }
function Target {
    if ($env:DOCKER_HOST) { throw 'DOCKER_HOST override is not permitted.' }
    $endpoint = ((Native $docker @('--context', 'desktop-linux', 'context', 'inspect', 'desktop-linux', '--format', '{{.Endpoints.docker.Host}}')) -join '').Trim()
    if (-not $endpoint.StartsWith('npipe://') -or ((Native $docker @('--context', 'desktop-linux', 'info', '--format', '{{.OSType}}')) -join '').Trim() -cne 'linux') {
        throw 'Expected the local Windows Linux Engine.'
    }
    $ids = @(Native $docker @('--context', 'desktop-linux', 'ps', '-aq', '--no-trunc', '--filter', 'label=io.x-k8s.kind.cluster=aiforall-local-full'))
    if ($ids.Count -ne 3) { throw 'Expected three running nodes; runtime was not started.' }
    $roles = @(); $names = @()
    foreach ($id in $ids) {
        if ($id -cnotmatch '^[a-f0-9]{64}$') { throw 'Invalid node ID.' }
        if (((Native $docker @('--context', 'desktop-linux', 'inspect', '--format', '{{.State.Status}}', $id)) -join '').Trim() -cne 'running') { throw 'A lab node is not running; no startup attempted.' }
        $labels = ConvertFrom-Json ((Native $docker @('--context', 'desktop-linux', 'inspect', '--format', '{{json .Config.Labels}}', $id)) -join '')
        if ($labels.'io.x-k8s.kind.cluster' -cne 'aiforall-local-full') { throw 'Node cluster mismatch.' }
        $roles += $labels.'io.x-k8s.kind.role'
        $name = ((Native $docker @('--context', 'desktop-linux', 'inspect', '--format', '{{.Name}}', $id)) -join '').Trim()
        if (($name -ceq '/aiforall-local-full-control-plane') -ne ($labels.'io.x-k8s.kind.role' -ceq 'control-plane')) { throw 'Kind node name/role mismatch.' }
        $names += $name
    }
    if (@($roles | Where-Object { $_ -eq 'control-plane' }).Count -ne 1 -or @($roles | Where-Object { $_ -eq 'worker' }).Count -ne 2 -or
        @($names | Where-Object { $_ -notin @('/aiforall-local-full-control-plane', '/aiforall-local-full-worker', '/aiforall-local-full-worker2') }).Count -or
        @($names | Select-Object -Unique).Count -ne 3) { throw 'Unexpected Kind topology.' }
    $server = ((Native $kubectl @('--kubeconfig', $cfg.kubeconfig, '--context', $context, 'config', 'view', '--minify', '-o', 'jsonpath={.clusters[0].cluster.server}')) -join '').Trim()
    if ($server -cnotmatch '^https://127\.0\.0\.1:[0-9]+$') { throw 'Expected a local Kubernetes API.' }
    $control = $ids[[Array]::IndexOf($roles, 'control-plane')]
    $apiPort = ((Native $docker @('--context', 'desktop-linux', 'port', $control, '6443/tcp')) -join '').Trim()
    if ($apiPort -cne ('127.0.0.1:' + ([Uri]$server).Port)) { throw 'Kubeconfig is not bound to this control-plane port.' }
    $uid = ((K @('get', 'namespace', $namespace, '-o', 'jsonpath={.metadata.uid}')) -join '').Trim()
    if ($uid -notmatch '^[a-f0-9-]{36}$') { throw 'Missing namespace identity.' }
    $deployment = ConvertFrom-Json ((K @('get', 'deployment', 'envoy-mesh', '-o', 'json')) -join "`n")
    if ($deployment.metadata.labels.'aiforall.dev/kind-minimal-compute' -cne 'true' -or $deployment.status.readyReplicas -ne 1) { throw 'Native edge is not ready.' }
    $pods = ConvertFrom-Json ((K @('get', 'pods', '-l', 'app.kubernetes.io/name=envoy-mesh', '-o', 'json')) -join "`n")
    if ($pods.items.Count -ne 1 -or $pods.items[0].status.phase -cne 'Running' -or
        $pods.items[0].metadata.labels.'aiforall.dev/kind-minimal' -cne 'true') { throw 'Expected one native edge Pod.' }
    $pod = $pods.items[0]
    $rs = ConvertFrom-Json ((K @('get', 'replicaset', $pod.metadata.ownerReferences[0].name, '-o', 'json')) -join "`n")
    if ($pod.metadata.ownerReferences[0].uid -cne $rs.metadata.uid -or $rs.metadata.ownerReferences[0].uid -cne $deployment.metadata.uid) { throw 'Edge ownership mismatch.' }
    return [pscustomobject]@{ namespace_uid = $uid; node_ids = @($ids | Sort-Object); edge_uid = $deployment.metadata.uid; pod = $pod.metadata.name; pod_uid = $pod.metadata.uid }
}
function K6([string]$Phase, [string]$Origin) {
    Save @{version = 1; run_id = $campaign.run_id; origin = $Origin} 'runtime.json'
    $summary = Join-Path $Directory ($Phase + '.json')
    if (Test-Path -LiteralPath $summary) { throw 'Phase already attempted; use a fresh campaign rather than overwrite proof.' }
    $attempt = [IO.File]::Open((Join-Path $Directory ($Phase + '.attempt')), [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    $attempt.Dispose() # A killed/failed phase cannot silently be retried under the same run_id.
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = [Diagnostics.ProcessStartInfo]::new($k6, ('run --quiet --no-usage-report --address 127.0.0.1:0 --config "' + (Join-Path $Directory 'k6-options.json') + '" "' + (Join-Path $PSScriptRoot 'functional.js') + '"'))
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    # Do not inherit K6_*, proxy, output or debug options from the calling shell.
    $process.StartInfo.EnvironmentVariables.Clear()
    foreach ($name in @('SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'APPDATA', 'LOCALAPPDATA')) {
        if ([Environment]::GetEnvironmentVariable($name)) { $process.StartInfo.EnvironmentVariables[$name] = [Environment]::GetEnvironmentVariable($name) }
    }
    $process.StartInfo.EnvironmentVariables['IAM_CAMPAIGN_FILE'] = Join-Path $Directory 'runtime.json'
    $process.StartInfo.EnvironmentVariables['IAM_SUMMARY_FILE'] = $summary
    $process.StartInfo.EnvironmentVariables['IAM_PHASE'] = $Phase
    $started = $false; $stdout = $null; $stderr = $null
    try {
        $started = $process.Start()
        $stdout = $process.StandardOutput.ReadToEndAsync(); $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(150000)) { throw 'k6 timed out.' }
        if ($process.ExitCode -ne 0) { throw 'k6 failed; inspect the private phase diagnostic.' }
    } finally {
        try {
            if ($started -and -not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
            if ($null -ne $stdout) { [IO.File]::WriteAllText((Join-Path $Directory ($Phase + '.stdout.log')), $stdout.GetAwaiter().GetResult(), $utf8) }
            if ($null -ne $stderr) { [IO.File]::WriteAllText((Join-Path $Directory ($Phase + '.stderr.log')), $stderr.GetAwaiter().GetResult(), $utf8) }
        } finally { $process.Dispose() }
    }
    $result = ConvertFrom-Json ([IO.File]::ReadAllText($summary))
    $expected = if ($Phase -eq 'preflight') { 1 } else { 12 }
    if ($result.run_id -cne $campaign.run_id -or $result.phase -cne $Phase -or $result.expected -ne $expected -or
        $result.finished -ne 1 -or $result.requests -ne $expected -or $result.checks.Count -ne $expected -or
        @($result.checks.name | Select-Object -Unique).Count -ne $expected -or
        @($result.checks | Where-Object { $_.passes -ne 1 -or $_.fails -ne 0 }).Count) { throw 'Incomplete or mismatched k6 summary.' }
}

if (-not [IO.Path]::IsPathRooted($Directory) -or $Directory.StartsWith('\\')) { throw 'Use an absolute local campaign directory.' }
$Directory = [IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
if ($Directory.Equals($root, [StringComparison]::OrdinalIgnoreCase) -or $Directory.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Campaign must be outside the repository.' }
$ancestor = [IO.DirectoryInfo]::new($Directory)
while ($null -ne $ancestor) {
    if ($ancestor.Exists -and ($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Reparse points in campaign ancestry are not supported.' }
    $ancestor = $ancestor.Parent
}
if ($Action -eq 'prepare') {
    $cfg = ConvertFrom-Json ([IO.File]::ReadAllText((AbsoluteFile $Config)))
    if (Test-Path -LiteralPath $Directory) { throw 'Prepare requires a new campaign directory.' }
    $null = New-Item -ItemType Directory -Path $Directory
    $acl = [Security.AccessControl.DirectorySecurity]::new()
    $acl.SetOwner($sid); $acl.SetAccessRuleProtection($true, $false)
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    Set-Acl -LiteralPath $Directory -AclObject $acl
    $campaign = [pscustomobject]@{ version = 1; run_id = [Guid]::NewGuid().ToString('N'); config = $cfg; prepared = $false; target = $null
        scenario_hash = (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'functional.js') -Algorithm SHA256).Hash
        runner_hash = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash }
    Save $campaign 'campaign.json'
    Save @{} 'k6-options.json'
} else {
    PrivateDirectory
    $campaign = ConvertFrom-Json ([IO.File]::ReadAllText((Join-Path $Directory 'campaign.json')))
    if ($campaign.version -ne 1 -or $campaign.run_id -cnotmatch '^[a-f0-9]{32}$') { throw 'Invalid campaign.' }
    $cfg = $campaign.config
}
$lockPath = Join-Path $Directory '.active.lock'
# Never remove/adopt a stale lock automatically; another invocation may own it.
$lock = [IO.File]::Open($lockPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
try {
if ($Action -eq 'clean') {
    # No persistent fixture is created by this tranche; keep result evidence.
    foreach ($name in @('runtime.json', 'k6-options.json')) {
        $file = Join-Path $Directory $name
        if (Test-Path -LiteralPath $file) { Remove-Item -LiteralPath $file }
    }
    $campaign.prepared = $false; Save $campaign 'campaign.json'
    Save @{run_id = $campaign.run_id; status = 'temporary-inputs-removed'; business_data_deleted = $false; business_effects_checked = $false; at = [DateTime]::UtcNow.ToString('o')} 'cleanup.json'
    Write-Output 'IAM cleanup: temporary inputs removed; results retained. Lab untouched.'
    return
}
PrivateDirectory
if ($campaign.scenario_hash -cne (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'functional.js') -Algorithm SHA256).Hash -or
    $campaign.runner_hash -cne (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash) { throw 'Sources changed since prepare; use a fresh campaign.' }
$k6 = Pin $cfg.k6; $kubectl = Pin $cfg.kubectl; $docker = Pin $cfg.docker
$null = AbsoluteFile $cfg.kubeconfig
$pem = [IO.File]::ReadAllText((AbsoluteFile $cfg.ca_file))
if ($pem -notmatch '(?s)^\s*-----BEGIN CERTIFICATE-----\s*([A-Za-z0-9+/=\s]+)-----END CERTIFICATE-----\s*$') { throw 'Select one PEM CA certificate, not a key or bundle.' }
$ca = [Security.Cryptography.X509Certificates.X509Certificate2]::new([Convert]::FromBase64String($Matches[1]))
if (-not (Test-Path ('Cert:\CurrentUser\Root\' + $ca.Thumbprint)) -and -not (Test-Path ('Cert:\LocalMachine\Root\' + $ca.Thumbprint))) {
    throw 'CA must already be explicitly trusted by Windows. No certificate-store import or insecure fallback is performed.'
}
$prefix = @('--kubeconfig', $cfg.kubeconfig, '--context', $context, '--namespace', $namespace)
$target = Target
    if ($Action -eq 'run' -and (-not $campaign.prepared -or $campaign.target.namespace_uid -cne $target.namespace_uid -or
    $campaign.target.edge_uid -cne $target.edge_uid -or (($campaign.target.node_ids -join ',') -cne ($target.node_ids -join ',')))) { throw 'Campaign target changed or prepare did not pass.' }
$forward = [Diagnostics.Process]::new()
$forward.StartInfo = [Diagnostics.ProcessStartInfo]::new($kubectl, ('--kubeconfig "' + $cfg.kubeconfig + '" --context ' + $context + ' --namespace ' + $namespace + ' port-forward --address 127.0.0.1 pod/' + $target.pod + ' :10000'))
$forward.StartInfo.UseShellExecute = $false
$forward.StartInfo.RedirectStandardOutput = $true; $forward.StartInfo.RedirectStandardError = $true
$forwardStarted = $false
try {
    $forwardStarted = $forward.Start()
    $errors = $forward.StandardError.ReadToEndAsync()
    $line = $forward.StandardOutput.ReadLineAsync()
    if (-not $line.Wait(15000) -or $line.Result -cnotmatch '^Forwarding from 127\.0\.0\.1:([0-9]+) -> 10000$') { throw 'Owned edge forward unavailable.' }
    $origin = 'https://127.0.0.1:' + $Matches[1]
    if (((K @('get', 'pod', $target.pod, '-o', 'jsonpath={.metadata.uid}')) -join '').Trim() -cne $target.pod_uid) { throw 'Edge Pod changed during forward admission.' }
    $tcp = [Net.Sockets.TcpClient]::new()
    $tls = $null; $chain = $null
    try {
        $connect = $tcp.ConnectAsync('127.0.0.1', ([Uri]$origin).Port)
        if (-not $connect.Wait(10000)) { throw 'TLS connection timed out.' }
        $tls = [Net.Security.SslStream]::new($tcp.GetStream(), $false)
        $tls.ReadTimeout = 10000; $tls.WriteTimeout = 10000
        $tls.AuthenticateAsClient('127.0.0.1') # System chain and SAN verification, no permissive callback.
        $chain = [Security.Cryptography.X509Certificates.X509Chain]::new()
        $chain.ChainPolicy.RevocationMode = 'NoCheck'
        $leaf = [Security.Cryptography.X509Certificates.X509Certificate2]::new($tls.RemoteCertificate)
        if (-not $chain.Build($leaf) -or $chain.ChainElements[$chain.ChainElements.Count - 1].Certificate.Thumbprint -cne $ca.Thumbprint) { throw 'Edge chain does not terminate at the selected CA.' }
    } finally {
        if ($null -ne $chain) { $chain.Dispose() }
        if ($null -ne $tls) { $tls.Dispose() }
        $tcp.Dispose()
    }
    # The actual k6 process proves TLS trust/SAN before the functional phase.
    K6 $(if ($Action -eq 'prepare') { 'preflight' } else { 'functional' }) $origin
    if ($Action -eq 'prepare') { $campaign.target = $target; $campaign.prepared = $true; Save $campaign 'campaign.json' }
    Write-Output "IAM $Action passed (partial stateless tranche only)."
} finally {
    try {
        if ($forwardStarted -and -not $forward.HasExited) { $forward.Kill(); $forward.WaitForExit() }
        if ($forwardStarted) { $null = $errors.GetAwaiter().GetResult() }
    } finally { $forward.Dispose() }
}
} finally {
    $lock.Dispose()
    Remove-Item -LiteralPath $lockPath
}
