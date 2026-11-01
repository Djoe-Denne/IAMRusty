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
        [string]$LeasePath = $env:AIFORALL_RUNTIME_LEASE,
        [Parameter(ValueFromPipeline = $true)][AllowNull()][object]$InputObject
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
    $root = Get-AIForAllRoot $PSScriptRoot
    $arguments = @('-B', (Join-Path $root 'ops/deploy/bootstrap_local_kind.py'),
        '--lease', $LeasePath, '--cluster', $Cluster, '--config', $Config)
    if ($AfterFinalIT) { $arguments += '--after-final-it' }
    # Shared Python retains CNI/CIDR, Calico 3.29.7, FELIX, CoreDNS/Ready and leasev2.
    & python @arguments
    if ($LASTEXITCODE -ne 0) { throw 'STOP: guarded Kind/CNI bootstrap failed; no adoption or reset.' }
}
