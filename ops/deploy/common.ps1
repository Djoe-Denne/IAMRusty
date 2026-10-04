# Shared helpers; dot-sourcing performs no runtime/API operation.
function Get-AIForAllRoot {
    param([Parameter(Mandatory = $true)][string]$StartPath)
    $dir = Get-Item -LiteralPath $StartPath
    while ($null -ne $dir) {
        if ((Test-Path -LiteralPath (Join-Path $dir.FullName 'Cargo.toml')) -and
            (Test-Path -LiteralPath (Join-Path $dir.FullName 'ops/deploy/kind/cluster.yaml'))) {
            return (Resolve-Path -LiteralPath $dir.FullName)
        }
        $dir = $dir.Parent
    }
    throw "AIForAll root not found from $StartPath"
}

function Invoke-LocalTarget {
    param(
        [string]$Action = 'check',
        [string[]]$Arguments = @(),
        [string]$LeasePath = $env:AIFORALL_RUNTIME_LEASE,
        [ValidateSet('kind-aiforall-local', 'kind-aiforall-local-full')]
        [string]$Context = 'kind-aiforall-local'
    )
    if ([string]::IsNullOrWhiteSpace($LeasePath)) { throw 'STOP: explicit leasev2 path required.' }
    $root = Get-AIForAllRoot $PSScriptRoot
    $cluster = $Context.Substring(5)
    $cli = Join-Path $root 'ops/deploy/local_full_target.py'
    # The shared Python guard binds CP publication/file hash/UID before every API.
    # No shell-level ambient kubectl fallback or raw config/credential output.
    if ($Action -eq 'kubectl') {
        $input | & python -B $cli $Action --lease $LeasePath --cluster $cluster -- @Arguments
    } else {
        & python -B $cli $Action --lease $LeasePath --cluster $cluster @Arguments
    }
    if ($LASTEXITCODE -ne 0) { throw 'STOP: target anchor/operation failed; no adoption or reset.' }
}

function Invoke-LocalKubectl {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [ValidateSet('kind-aiforall-local', 'kind-aiforall-local-full')]
        [string]$Context = 'kind-aiforall-local',
        [string]$LeasePath = $env:AIFORALL_RUNTIME_LEASE
    )
    $input | Invoke-LocalTarget -Action kubectl -Context $Context -LeasePath $LeasePath -Arguments $Arguments
}

function Set-LocalDeploymentIdentity {
    param(
        [Parameter(Mandatory = $true)][string]$Namespace,
        [Parameter(Mandatory = $true)][string]$Deployment,
        [Parameter(Mandatory = $true)][string]$Container,
        [Parameter(Mandatory = $true)][string]$SourceImage,
        [Parameter(Mandatory = $true)][string]$Configuration,
        [string[]]$InitContainerNames = @()
    )
    Invoke-LocalTarget
    $id = [string](& docker image inspect --format '{{.Id}}' $SourceImage)
    if ($LASTEXITCODE -ne 0 -or $id -notmatch '^sha256:[a-f0-9]{64}$') { throw 'Cannot identify local image.' }
    $repository = ($SourceImage -split ':')[0]
    $contentImage = "${repository}:sha256-$($id.Substring(7))"
    & docker tag $SourceImage $contentImage
    if ($LASTEXITCODE -ne 0) { throw 'Content image tagging failed.' }
    Invoke-LocalTarget -Action kind-load -Arguments @('--', $contentImage)
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try { $hash = ([BitConverter]::ToString($hasher.ComputeHash([Text.Encoding]::UTF8.GetBytes($Configuration)))).Replace('-', '').ToLowerInvariant() }
    finally { $hasher.Dispose() }
    $templateSpec = @{ containers = @(@{ name = $Container; image = $contentImage }) }
    if ($InitContainerNames.Count -gt 0) {
        $templateSpec.initContainers = @($InitContainerNames | ForEach-Object { @{ name = $_; image = $contentImage } })
    }
    $patch = @{ spec = @{ template = @{
        metadata = @{ annotations = @{ 'aiforall.dev/config-sha256' = $hash; 'aiforall.dev/image-id' = $id } }
        spec = $templateSpec
    } } } | ConvertTo-Json -Depth 10 -Compress
    $file = [IO.Path]::GetTempFileName()
    try {
        [IO.File]::WriteAllText($file, $patch, [Text.UTF8Encoding]::new($false))
        Invoke-LocalKubectl -Arguments @('-n', $Namespace, 'patch', "deployment/$Deployment", '--type=strategic', '--patch-file', $file)
    } finally { Remove-Item -LiteralPath $file -ErrorAction SilentlyContinue }
    Invoke-LocalKubectl -Arguments @('-n', $Namespace, 'rollout', 'status', "deployment/$Deployment", '--timeout=300s')
}

function Initialize-LocalKind {
    param(
        [string]$Config = 'ops/deploy/kind/cluster.yaml',
        [string]$LeasePath = $env:AIFORALL_RUNTIME_LEASE,
        [ValidateSet('aiforall-local', 'aiforall-local-full')][string]$Cluster = 'aiforall-local',
        [switch]$AfterFinalIT
    )
    if (($Config -like '*cluster-local-full.yaml') -ne ($Cluster -eq 'aiforall-local-full')) { throw 'STOP: full topology/target mismatch.' }
    $Context = "kind-$Cluster"
    $argsForBootstrap = @('--config', $Config)
    if ($AfterFinalIT) { $argsForBootstrap += '--after-final-it' }
    Invoke-LocalTarget -Action bootstrap -Context $Context -LeasePath $LeasePath -Arguments $argsForBootstrap
    # Retain non-destructive pinned CNI logic; every call now uses leasev2 guard.
    $sets = @((Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('-n', 'kube-system', 'get', 'daemonsets', '-o', 'json') | ConvertFrom-Json).items)
    if (@($sets | Where-Object { $_.metadata.name -match 'kindnet|cilium|flannel|weave|antrea|canal|kube-router' }).Count -gt 0) { throw 'STOP: incompatible CNI; no replacement.' }
    $cidrs = Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('get', 'nodes', '-o', 'jsonpath={.items[*].spec.podCIDR}')
    if ([string]::IsNullOrWhiteSpace([string]$cidrs)) { throw 'STOP: CIDR unavailable.' }
    foreach ($cidr in (([string]$cidrs).Trim() -split ' ')) {
        if ($cidr -notmatch '^10\.244\.\d+\.\d+/(1[6-9]|2\d|3[0-2])$') { throw 'STOP: incompatible CIDR.' }
    }
    $calico = @($sets | Where-Object { $_.metadata.name -eq 'calico-node' })
    if ($calico.Count -eq 0) {
        if (@($sets | Where-Object { $_.metadata.name -ne 'kube-proxy' }).Count -gt 0) { throw 'STOP: unknown DaemonSet; no adoption.' }
        $manifest = (Invoke-WebRequest -UseBasicParsing 'https://raw.githubusercontent.com/projectcalico/calico/v3.29.7/manifests/calico.yaml').Content
        if ($manifest -notmatch '# - name: CALICO_IPV4POOL_CIDR\s+#   value: "192.168.0.0/16"') { throw 'STOP: pinned Calico shape changed.' }
        $manifest = $manifest -replace '# - name: CALICO_IPV4POOL_CIDR\s+#   value: "192.168.0.0/16"', "- name: CALICO_IPV4POOL_CIDR`n              value: `"10.244.0.0/16`""
        $manifest | Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('apply', '-f', '-')
        Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('-n', 'kube-system', 'set', 'env', 'daemonset/calico-node', 'FELIX_IGNORELOOSERPF=true')
    } else {
        $images = @($calico[0].spec.template.spec.containers | ForEach-Object { $_.image })
        if (@($images | Where-Object { $_ -notmatch ':v3\.29\.7$' }).Count -gt 0) { throw 'STOP: incompatible Calico pin.' }
        $pools = @( (Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('get', 'ippools', '-o', 'json') | ConvertFrom-Json).items )
        if ($pools.Count -ne 1 -or $pools[0].spec.cidr -ne '10.244.0.0/16') { throw 'STOP: pool mismatch; no patch/delete.' }
    }
    foreach ($resource in @('daemonset/calico-node', 'deployment/calico-kube-controllers', 'deployment/coredns')) {
        Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('-n', 'kube-system', 'rollout', 'status', $resource, '--timeout=600s')
    }
    Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('wait', '--for=condition=Ready', 'nodes', '--all', '--timeout=300s')
    $pools = @((Invoke-LocalKubectl -Context $Context -LeasePath $LeasePath -Arguments @('get', 'ippools', '-o', 'json') | ConvertFrom-Json).items)
    if ($pools.Count -ne 1 -or $pools[0].spec.cidr -ne '10.244.0.0/16') { throw 'STOP: final pool mismatch.' }
}
