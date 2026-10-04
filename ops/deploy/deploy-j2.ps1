#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
$RepoRoot = Get-AIForAllRoot $PSScriptRoot
Set-Location $RepoRoot

$KindContext = 'kind-aiforall-local'
$ClusterName = 'aiforall-local'
$Image = 'aiforall-apparatus-controller:j2'
$Dockerfile = 'workers/apparatus-operator/Dockerfile.controller'
foreach ($command in @('kind', 'docker', 'kubectl')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "Missing tool: $command" }
}
Initialize-LocalKind

Write-Host "J2 : docker build $Image" -ForegroundColor Cyan
& docker build --platform linux/amd64 --provenance=false --sbom=false -t $Image -f $Dockerfile .
if ($LASTEXITCODE -ne 0) { throw 'J2: controller image build failed.' }
& kind load docker-image $Image --name $ClusterName
if ($LASTEXITCODE -ne 0) { throw 'J2: controller image load failed.' }

$ZotImage = 'aiforall-zot:gold'
& docker image inspect $ZotImage *> $null
if ($LASTEXITCODE -ne 0) {
    & docker build --platform linux/amd64 --provenance=false --sbom=false -t $ZotImage -f ops/deploy/p4/Dockerfile.zot ops/deploy/p4
    if ($LASTEXITCODE -ne 0) { throw 'J2: Zot image build failed.' }
}
& kind load docker-image $ZotImage --name $ClusterName
if ($LASTEXITCODE -ne 0) { throw 'J2: Zot image load failed.' }

Invoke-LocalKubectl @('apply', '-k', 'ops/deploy/p4')
Invoke-LocalKubectl @('wait', '--for=condition=Established', 'crd/admissionrecords.apparatus.aiforall.dev', '--timeout=60s')
# Operator and its init container receive the same immutable content image.
$configuration = Get-Content -Raw -LiteralPath 'ops/deploy/p4/operator.yaml'
Set-LocalDeploymentIdentity -Namespace aiforall-apparatus -Deployment apparatus-operator -Container operator -SourceImage $Image -Configuration $configuration -InitContainerNames @('fix-store-perms')
Invoke-LocalKubectl @('rollout', 'status', 'deployment/apparatus-operator', '-n', 'aiforall-apparatus', '--timeout=180s')
Invoke-LocalKubectl @('rollout', 'status', 'deployment/zot', '-n', 'aiforall-platform', '--timeout=120s')
Invoke-LocalKubectl @('get', 'pod', '-n', 'aiforall-apparatus', '-l', 'app.kubernetes.io/name=apparatus-operator')
Write-Host 'J2: local controller Ready; parent retains the runtime lease. No cluster/CNI reset.' -ForegroundColor Green
