#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
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

$clusterOut = & kind get clusters 2>&1
$hasLocal = $false
foreach ($line in @($clusterOut)) {
    if (([string]$line).Trim() -eq $ClusterName) { $hasLocal = $true }
}
if (-not $hasLocal) {
    Write-Host 'mesh STOP : cluster kind aiforall-local absent. just deploy-m2 le cree.' -ForegroundColor Red
    exit 1
}

Write-Host "mesh : contexte $KindContext (jamais $ForbiddenContext)" -ForegroundColor Cyan
Write-Host 'mesh : cargo build hote = target/. Image Kind = compilation Linux dans Docker, comme J3.' -ForegroundColor Cyan

Write-Host 'mesh : certificats platform-mesh' -ForegroundColor Cyan
& docker run --rm `
    -v "${PWD}/certs/platform-mesh:/certs" `
    -v "${PWD}/scripts/generate-platform-mesh-certs.sh:/generate-platform-mesh-certs.sh:ro" `
    alpine:3.20 `
    sh -c "apk add --no-cache openssl >/dev/null && sh /generate-platform-mesh-certs.sh"
if ($LASTEXITCODE -ne 0) {
    Write-Host 'mesh echec : certificats' -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "mesh : docker build $Image" -ForegroundColor Cyan
& docker build --platform linux/amd64 -t $Image -f ext-authz/Dockerfile .
if ($LASTEXITCODE -ne 0) {
    Write-Host 'mesh echec : docker build ext-authz' -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "mesh : kind load $Image --name $ClusterName" -ForegroundColor Cyan
& kind load docker-image $Image --name $ClusterName
if ($LASTEXITCODE -ne 0) {
    Write-Host 'mesh echec : kind load' -ForegroundColor Red
    exit $LASTEXITCODE
}

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host ("mesh echec : kubectl {0}" -f ($KubectlArgs -join ' ')) -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Invoke-Kubectl @('apply', '-f', 'deploy/apps/base/namespaces.yaml')

$certDir = Join-Path $RepoRoot 'certs/platform-mesh'
$secretArgs = @(
    '--context', $KindContext,
    '-n', $Namespace,
    'create', 'secret', 'generic', 'platform-mesh-certs',
    '--from-file=ca.crt',
    '--from-file=envoy-mesh.crt',
    '--from-file=envoy-mesh.key',
    '--from-file=ext-authz.crt',
    '--from-file=ext-authz.key',
    '--dry-run=client', '-o', 'yaml'
)
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
    Write-Host 'mesh echec : secret platform-mesh-certs' -ForegroundColor Red
    exit $LASTEXITCODE
}

Invoke-Kubectl @('apply', '-k', 'deploy/apps/overlays/kind-mesh')

Invoke-Kubectl @('rollout', 'status', 'deployment/ext-authz', '-n', $Namespace, '--timeout=180s')
Invoke-Kubectl @('rollout', 'status', 'deployment/envoy-mesh', '-n', $Namespace, '--timeout=180s')

Write-Host 'mesh : ext-authz et envoy-mesh sont Ready sur aiforall-local.' -ForegroundColor Green
Write-Host 'IAM Kind (aiforall-platform/iam) reste l image pause M1. Le JWT reel est celui du monolithe J3 ou du profil Compose.' -ForegroundColor Yellow
