#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
$RepoRoot = Get-AIForAllRoot $PSScriptRoot
Set-Location $RepoRoot

$KindContext = 'kind-aiforall-local'
$ForbiddenContext = 'kind-apparatus-p4-it'
$ClusterName = 'aiforall-local'
$Image = 'aiforall-ext-authz:local'
$Namespace = 'aiforall-gateway'

function Test-HasCommand {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

$missing = @()
if (-not (Test-HasCommand 'kind')) { $missing += 'kind' }
if (-not (Test-HasCommand 'docker')) { $missing += 'docker' }
if (-not (Test-HasCommand 'kubectl')) { $missing += 'kubectl' }
if ($missing.Count -gt 0) {
    Write-Host ("mesh STOP : outil manquant ({0})" -f ($missing -join ', ')) -ForegroundColor Red
    exit 1
}

Initialize-LocalKind

Write-Host "mesh : contexte $KindContext (jamais $ForbiddenContext)" -ForegroundColor Cyan
Write-Host 'mesh : cargo build hote = target/. Image Kind = compilation Linux dans Docker, comme J3.' -ForegroundColor Cyan

Write-Host 'mesh : certificats platform-mesh' -ForegroundColor Cyan
& docker run --rm `
    -v "${PWD}/ops/certs/platform-mesh:/certs" `
    -v "${PWD}/ops/scripts/generate-platform-mesh-certs.sh:/generate-platform-mesh-certs.sh:ro" `
    alpine:3.20 `
    sh -c "apk add --no-cache openssl >/dev/null && sh /generate-platform-mesh-certs.sh"
if ($LASTEXITCODE -ne 0) {
    Write-Host 'mesh echec : certificats' -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "mesh : docker build $Image" -ForegroundColor Cyan
& docker build --platform linux/amd64 -t $Image -f workers/ext-authz/Dockerfile .
if ($LASTEXITCODE -ne 0) {
    Write-Host 'mesh echec : docker build ext-authz' -ForegroundColor Red
    exit $LASTEXITCODE
}

$loadImages = @(
    $Image,
    'aiforall-iam-service:latest',
    'aiforall-hive-service:latest',
    'aiforall-telegraph-service:latest',
    'aiforall-manifesto-service:latest',
    'postgres:15.12-alpine3.21@sha256:ef9d1517df69c4d27dbb9ddcec14f431a2442628603f4e9daa429b92ae6c3cd1',
    'openfga/openfga:v1.8.5@sha256:d023be1d5b75df633c1efa6ab6b86748e77b05d5e78d0f40973a5e47d4264ce7',
    'envoyproxy/envoy:v1.31.2@sha256:c56804e9fc8d5184c45862ad851173a925ff383151cb94156f86bed047da29bc',
    'alpine:3.20'
)
foreach ($img in $loadImages) {
    & docker image inspect $img *> $null
    if ($LASTEXITCODE -ne 0) {
        if ($img -like 'aiforall-*') { throw "Local application image $img missing; parent must build it in Docker first." }
        Write-Host "mesh : docker pull $img" -ForegroundColor Cyan
        & docker pull $img
        if ($LASTEXITCODE -ne 0) {
            Write-Host "mesh echec : image absente $img" -ForegroundColor Red
            exit $LASTEXITCODE
        }
    }
    Write-Host "mesh : kind load $img --name $ClusterName" -ForegroundColor Cyan
    & kind load docker-image $img --name $ClusterName
    if ($LASTEXITCODE -ne 0) {
        # Multi-arch indexes (postgres, openfga, alpine) fail ctr import until
        # the tag is a single linux/amd64 image.
        if ($img -like '*@sha256:*') { throw "Pinned image import failed: $img. Do not rebuild/retag a pinned upstream image." }
        $df = Join-Path $env:TEMP 'kind-single.Dockerfile'
        Set-Content -Path $df -Value "FROM $img`n" -Encoding ascii
        & docker build --platform linux/amd64 -t $img -f $df $env:TEMP
        if ($LASTEXITCODE -ne 0) {
            Write-Host "mesh echec : image mono-arch $img" -ForegroundColor Red
            exit $LASTEXITCODE
        }
        & kind load docker-image $img --name $ClusterName
        if ($LASTEXITCODE -ne 0) {
            Write-Host "mesh echec : kind load $img" -ForegroundColor Red
            exit $LASTEXITCODE
        }
    }
}

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host ("mesh echec : kubectl {0}" -f ($KubectlArgs -join ' ')) -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Invoke-Kubectl @('apply', '-f', 'ops/deploy/apps/base/namespaces.yaml')

$certDir = Join-Path $RepoRoot 'ops/certs/platform-mesh'
$certFiles = @(
    'ca.crt',
    'envoy-mesh.crt', 'envoy-mesh.key',
    'ext-authz.crt', 'ext-authz.key',
    'iam-service.crt', 'iam-service.key',
    'hive-service.crt', 'hive-service.key',
    'telegraph-service.crt', 'telegraph-service.key',
    'manifesto-service.crt', 'manifesto-service.key',
    'mesh-client.crt', 'mesh-client.key'
)
foreach ($ns in @($Namespace, 'aiforall-platform')) {
    $secretArgs = @(
        '--context', $KindContext,
        '-n', $ns,
        'create', 'secret', 'generic', 'platform-mesh-certs',
        '--dry-run=client', '-o', 'yaml'
    )
    foreach ($file in $certFiles) {
        $secretArgs += "--from-file=$file"
    }
    Push-Location $certDir
    try {
        $secretYaml = & kubectl @secretArgs
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    finally {
        Pop-Location
    }
    $secretYaml | kubectl --context $KindContext apply -f -
    if ($LASTEXITCODE -ne 0) {
        Write-Host "mesh echec : secret platform-mesh-certs ($ns)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

$meshYaml = & kubectl kustomize --load-restrictor LoadRestrictionsNone 'ops/deploy/apps/overlays/kind-mesh'
if ($LASTEXITCODE -ne 0) {
    Write-Host 'mesh echec : kustomize kind-mesh' -ForegroundColor Red
    exit $LASTEXITCODE
}
$meshYaml | kubectl --context $KindContext apply -f -
if ($LASTEXITCODE -ne 0) {
    Write-Host 'mesh echec : apply kind-mesh' -ForegroundColor Red
    exit $LASTEXITCODE
}

$configurationIdentity = $meshYaml | Out-String
foreach ($name in @('iam', 'hive', 'telegraph', 'manifesto')) {
    Set-LocalDeploymentIdentity -Namespace aiforall-platform -Deployment $name -Container $name -SourceImage "aiforall-$name-service:latest" -Configuration $configurationIdentity
}
Set-LocalDeploymentIdentity -Namespace $Namespace -Deployment ext-authz -Container ext-authz -SourceImage $Image -Configuration $configurationIdentity
# Envoy's inline ConfigMap is not generated. Include its actual rendered bytes
# in the template identity, so a config change rolls the mounted file too.
$envoyPatch = @{ spec = @{ template = @{ metadata = @{ annotations = @{ 'aiforall.dev/config-sha256' = (& {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($configurationIdentity)))).Replace('-', '').ToLowerInvariant() } finally { $sha.Dispose() }
}) } } } } } | ConvertTo-Json -Depth 10 -Compress
$patchFile = [IO.Path]::GetTempFileName()
try {
    [IO.File]::WriteAllText($patchFile, $envoyPatch, [Text.UTF8Encoding]::new($false))
    Invoke-Kubectl @('-n', $Namespace, 'patch', 'deployment/envoy-mesh', '--type=merge', '--patch-file', $patchFile)
} finally { Remove-Item -LiteralPath $patchFile -ErrorAction SilentlyContinue }

Invoke-Kubectl @('rollout', 'status', 'deployment/postgres', '-n', 'aiforall-platform', '--timeout=180s')
Invoke-Kubectl @('rollout', 'status', 'deployment/openfga', '-n', 'aiforall-platform', '--timeout=180s')
Invoke-Kubectl @('rollout', 'status', 'deployment/iam', '-n', 'aiforall-platform', '--timeout=240s')
Invoke-Kubectl @('rollout', 'status', 'deployment/hive', '-n', 'aiforall-platform', '--timeout=240s')
Invoke-Kubectl @('rollout', 'status', 'deployment/telegraph', '-n', 'aiforall-platform', '--timeout=240s')
Invoke-Kubectl @('rollout', 'status', 'deployment/manifesto', '-n', 'aiforall-platform', '--timeout=240s')
Invoke-Kubectl @('rollout', 'status', 'deployment/ext-authz', '-n', $Namespace, '--timeout=180s')
Invoke-Kubectl @('rollout', 'status', 'deployment/envoy-mesh', '-n', $Namespace, '--timeout=180s')

Write-Host 'mesh : services reels, ext-authz et envoy-mesh sont Ready sur aiforall-local.' -ForegroundColor Green
Write-Host 'Preuve : bash ops/scripts/mesh-authn-kind-e2e.sh' -ForegroundColor Green
