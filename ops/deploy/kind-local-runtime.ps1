# Operator lifecycle only: no build, creation, reset, deletion or Python.
param([Parameter(Mandatory = $true)][ValidateSet('start', 'stop', 'status', 'apps-stop')][string]$Action)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$cluster = 'aiforall-local-full'
$context = 'kind-aiforall-local-full'
$namespace = 'aiforall-local-full'
$prefix = @('--context', $context, '--namespace', $namespace)

function Native([string]$File, [string[]]$Arguments) {
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & $File @Arguments
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previous }
    if ($code -ne 0) { throw "$File failed (exit $code); no automatic rollback or retry." }
    return $output
}
function K([string[]]$Arguments) { Native 'kubectl.exe' ($prefix + $Arguments) }
function Nodes {
    $ids = @(Native 'docker.exe' @('ps', '-aq', '--no-trunc', '--filter', "label=io.x-k8s.kind.cluster=$cluster"))
    if (-not $ids.Count) { return @() }
    # No embedded quote arguments: Windows PowerShell 5.1 strips them when
    # forwarding native argv. Read only public labels and lifecycle metadata.
    $rows = @(foreach ($id in $ids) {
        $fields = (((Native 'docker.exe' @('inspect', '--format', '{{.Id}} {{.Name}} {{.State.Status}} {{.State.ExitCode}}', $id)) -join '').Trim()).Split(' ')
        $labels = ConvertFrom-Json ((Native 'docker.exe' @('inspect', '--format', '{{json .Config.Labels}}', $id)) -join "`n")
        [pscustomobject]@{ id = $fields[0]; name = $fields[1]; state = $fields[2]; exitCode = $fields[3]
            cluster = $labels.'io.x-k8s.kind.cluster'; role = $labels.'io.x-k8s.kind.role' }
    })
    $expected = @("/$cluster-control-plane", "/$cluster-worker", "/$cluster-worker2")
    if ($rows.Count -ne 3 -or @($rows | Where-Object {
        $_.cluster -cne $cluster -or $_.name -cnotin $expected -or $_.id -cnotmatch '^[a-f0-9]{64}$' -or
        $_.role -cne $(if ($_.name -ceq "/$cluster-control-plane") { 'control-plane' } else { 'worker' }) -or
        $_.state -notin @('running', 'exited', 'created')
    }).Count -or @($rows.name | Select-Object -Unique).Count -ne 3) {
        throw 'Unexpected local Kind topology or state; no container is adopted, forced or replaced.'
    }
    return $rows
}
function Record([string]$Event, $Node) {
    $directory = Join-Path $root '.tmp'
    $null = [IO.Directory]::CreateDirectory($directory)
    $line = @{ ts = [DateTime]::UtcNow.ToString('o'); event = $Event; kind = 'container'; id = $Node.id
        safe_to_delete = 'no'; task = 'kind-local-runtime'; cluster = $cluster; reason = 'Explicit operator lifecycle command; persistent data retained' }
    [IO.File]::AppendAllText((Join-Path $directory ('docker-resources-' + [DateTime]::UtcNow.ToString('yyyyMMdd') + '.log')),
        ((ConvertTo-Json $line -Compress) + "`n"), [Text.UTF8Encoding]::new($false))
}
function Need([string]$Value, [string]$Name) {
    if (-not $Value -or -not [IO.Path]::IsPathRooted($Value) -or -not (Test-Path -LiteralPath $Value -PathType Leaf)) {
        throw "$Name must explicitly select an existing absolute file. See ops/deploy/kind-minimal.md."
    }
}

# Desktop status does not start a stopped daemon. Never silently select a remote Engine.
$desktop = (Native 'docker.exe' @('desktop', 'status')) -join "`n"
if ($desktop -notmatch '(?i)running' -or $desktop -match '(?i)not running|stopped') {
    if ($Action -eq 'status') { Write-Output 'Docker Desktop is stopped; lab status unavailable (runtime not started).'; return }
    throw 'Start Docker Desktop explicitly before operating the lab.'
}
if ($env:DOCKER_HOST) { throw 'DOCKER_HOST override is not supported for the fixed local lab.' }
$endpoint = ((Native 'docker.exe' @('context', 'inspect', '--format', '{{.Endpoints.docker.Host}}')) -join '').Trim()
if (-not $endpoint.StartsWith('npipe://') -or ((Native 'docker.exe' @('info', '--format', '{{.OSType}}')) -join '').Trim() -cne 'linux') {
    throw 'The operator commands require a local Windows named-pipe Linux Engine.'
}
$nodes = @(Nodes)
if (-not $nodes.Count) {
    if ($Action -eq 'status') { Write-Output 'Local lab absent. Install with just kind-local.'; return }
    throw 'Local lab absent. Use just kind-local; local-start never creates a cluster.'
}
$nodes | Select-Object name, state, exitCode | Format-Table -AutoSize
if ($Action -eq 'status') {
    if (@($nodes | Where-Object state -ne 'running').Count) { Write-Output 'Lab stopped or partially stopped; Kubernetes was not queried.'; return }
    $previous = $ErrorActionPreference
    try { $ErrorActionPreference = 'Continue'; $null = & kubectl.exe @prefix --request-timeout=5s get --raw=/readyz 2>$null; $code = $LASTEXITCODE }
    finally { $ErrorActionPreference = $previous }
    if ($code -ne 0) { throw 'Nodes are running but the configured Kubernetes API is unavailable. No kubeconfig repair, restart or recreation was attempted.' }
    $null = K @('get', 'nodes', '--request-timeout=10s') | Out-Host
    $null = K @('get', 'deployments,pods,pvc,jobs', '--request-timeout=10s') | Out-Host
    return
}
if ($Action -eq 'apps-stop') {
    $null = K @('scale', 'deployment', '--selector', 'aiforall.dev/kind-minimal-compute=true', '--replicas=0') | Out-Host
    return
}
if ($Action -eq 'stop') {
    Write-Host 'Stopping only this lab. Volatile LocalStack queues/messages will be lost; containers and volumes are retained.'
    $running = @($nodes | Where-Object state -eq 'running')
    foreach ($node in $running) { Record 'leased' $node }
    if ($running.Count) { $null = Native 'docker.exe' (@('stop', '--timeout', '120') + @($running.id)) | Out-Host }
    $after = @(Nodes)
    foreach ($node in $after) { if ($node.state -eq 'running') { throw 'A node is still running.' }; Record 'stopped' $node }
    $after | Select-Object name, state, exitCode | Format-Table -AutoSize
    return
}

# Preflight all operator inputs before starting any stopped node. No latest-file lookup.
Need $env:KIND_LOCAL_BAO 'KIND_LOCAL_BAO (official bao.exe 2.6.2)'
Need $env:KIND_LOCAL_OPENBAO_CA_FILE 'KIND_LOCAL_OPENBAO_CA_FILE'
Need $env:KIND_LOCAL_OPENBAO_UNSEAL_INPUT 'KIND_LOCAL_OPENBAO_UNSEAL_INPUT'
$helm = if ($env:AIFORALL_HELM) { $env:AIFORALL_HELM } else { 'helm.exe' }
$chartArgs = @('template', 'kind-local', (Join-Path $PSScriptRoot 'charts/kind-minimal'), '--namespace', $namespace)
if ($env:KIND_LOCAL_PUBLIC_VALUES) { $chartArgs += @('-f', $env:KIND_LOCAL_PUBLIC_VALUES) }
# Reuse the native initialization recipe, not a second list of queue names.
$jobYaml = Native $helm ($chartArgs + @('--set', 'localstack.create=false', '--set', 'localstack.initialize=true', '--show-only', 'templates/localstack.yaml'))
$stopped = @($nodes | Where-Object state -ne 'running')
foreach ($node in $stopped) { Record 'leased' $node }
if ($stopped.Count) { $null = Native 'docker.exe' (@('start') + @($stopped.id)) | Out-Host }
foreach ($node in $stopped) { Record 'restarted' $node }
$clock = [Diagnostics.Stopwatch]::StartNew()
do {
    $previous = $ErrorActionPreference
    try { $ErrorActionPreference = 'Continue'; $ready = & kubectl.exe --context $context --request-timeout=5s get --raw=/readyz 2>$null; $code = $LASTEXITCODE }
    finally { $ErrorActionPreference = $previous }
    if ($code -eq 0 -and ($ready -join '').Trim() -eq 'ok') { break }
    if ($clock.Elapsed.TotalSeconds -ge 180) { throw 'Kubernetes API did not become ready; nodes are retained, no reset.' }
    Start-Sleep -Seconds 2
} while ($true)
# Even client dry-run may perform REST mapping/discovery: only use it once
# this lab's API is ready, never the ambient context or a warm-cache assumption.
$jobJson = $jobYaml | & kubectl.exe @prefix --request-timeout=10s create --dry-run=client --validate=false -f - -o json
if ($LASTEXITCODE -ne 0) { throw 'Cannot decode the public native SQS recipe.' }
$job = ConvertFrom-Json ($jobJson -join "`n")
if ($job.kind -cne 'Job' -or $job.metadata.name -cne 'kind-local-sqs-initialize') { throw 'Unexpected SQS recipe.' }
$queues = @(ConvertFrom-Json $job.metadata.annotations.'aiforall.dev/queues')
if (-not $queues.Count -or @($queues | Where-Object { $_ -isnot [string] -or $_ -cnotmatch '^[A-Za-z0-9_-]+(\.fifo)?$' }).Count -or
    @($queues | Select-Object -Unique).Count -ne $queues.Count) { throw 'Invalid queue metadata in the native recipe.' }
$null = K @('wait', '--for=condition=Ready', 'nodes', '--all', '--timeout=180s') | Out-Host
$null = K @('wait', '--for=jsonpath={.status.phase}=Running', 'pods', '-l', 'app.kubernetes.io/name=openbao', '--timeout=180s') | Out-Host
$pod = ((K @('get', 'pods', '-l', 'app.kubernetes.io/name=openbao', '-o', 'jsonpath={.items[0].metadata.name}')) -join '').Trim()
$forward = [Diagnostics.Process]::new()
$forward.StartInfo = [Diagnostics.ProcessStartInfo]::new('kubectl.exe', "--context $context --namespace $namespace port-forward --address 127.0.0.1 pod/$pod 18444:8443")
$forward.StartInfo.UseShellExecute = $false
$forward.StartInfo.RedirectStandardOutput = $true
$forward.StartInfo.RedirectStandardError = $true
$started = $false
try {
    $started = $forward.Start()
    $errors = $forward.StandardError.ReadToEndAsync()
    $line = $forward.StandardOutput.ReadLineAsync()
    if (-not $line.Wait(30000) -or $line.Result -notlike 'Forwarding from 127.0.0.1:*') { throw 'Bao port-forward unavailable; no retry or alternate endpoint.' }
    Import-Module (Join-Path $PSScriptRoot 'openbao-cli.psm1') -Force
    $connection = @{ Executable = $env:KIND_LOCAL_BAO; Address = 'https://127.0.0.1:18444'; CaFile = $env:KIND_LOCAL_OPENBAO_CA_FILE; ServerName = 'openbao.aiforall-local-full.svc.cluster.local' }
    $null = Unlock-KindOpenBao -Connection $connection -InputFile $env:KIND_LOCAL_OPENBAO_UNSEAL_INPUT
} finally {
    if ($started -and -not $forward.HasExited) { $forward.Kill(); $forward.WaitForExit() }
    if ($started) { $null = $errors.GetAwaiter().GetResult() }
    $forward.Dispose()
}
$null = K @('rollout', 'status', 'deployment/openbao', '--timeout=180s') | Out-Host
$null = K @('rollout', 'status', 'deployment/localstack', '--timeout=180s') | Out-Host
$aws = @('exec', 'deployment/localstack', '--', 'aws', '--endpoint-url=https://localhost:4566', '--ca-bundle=/tls/ca.crt', 'sqs')
$listed = ConvertFrom-Json ((K ($aws + @('list-queues', '--output', 'json'))) -join "`n")
if ($listed.PSObject.Properties['NextToken']) { throw 'Incomplete queue inventory; no queues are created.' }
$existing = @()
if ($listed.PSObject.Properties['QueueUrls']) {
    $existing = @($listed.QueueUrls | ForEach-Object { ([Uri]$_).AbsolutePath.TrimEnd('/').Split('/')[-1] })
}
foreach ($queue in $queues) {
    if ($existing -cnotcontains $queue) { $null = K ($aws + @('create-queue', '--queue-name', $queue)) }
}
$deployments = ConvertFrom-Json ((K @('get', 'deployments', '-o', 'json')) -join "`n")
$active = @($deployments.items | Where-Object { $_.spec.replicas -gt 0 })
$inactive = @($deployments.items | Where-Object { $_.spec.replicas -eq 0 })
foreach ($deployment in $inactive) { Write-Output "Left scaled down: $($deployment.metadata.name)" }
foreach ($deployment in $active) { $null = K @('rollout', 'status', "deployment/$($deployment.metadata.name)", '--timeout=300s') | Out-Host }
Write-Output 'Active deployments are available. No images rebuilt; scaled-down applications were not enabled. SQS messages are not restored.'
