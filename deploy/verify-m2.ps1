#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

$KindContext = 'kind-aiforall-local'
$ForbiddenContext = 'kind-apparatus-p4-it'

function Test-HasCommand {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

$missing = @()
if (-not (Test-HasCommand 'kind')) { $missing += 'kind' }
if (-not (Test-HasCommand 'docker')) { $missing += 'docker' }
if (-not (Test-HasCommand 'kubectl')) { $missing += 'kubectl' }

if ($missing.Count -gt 0) {
    Write-Host ("M2 skip : outil manquant ({0}) ; M1 reste la preuve" -f ($missing -join ', '))
    exit 0
}

Write-Host "M2 : contexte $KindContext (jamais $ForbiddenContext)" -ForegroundColor Cyan

$clusterOut = & kind get clusters 2>&1
$hasLocal = $false
foreach ($line in @($clusterOut)) {
    if (([string]$line).Trim() -eq 'aiforall-local') {
        $hasLocal = $true
        break
    }
}
if (-not $hasLocal) {
    Write-Host "Creation du cluster kind aiforall-local..."
    & kind create cluster --config deploy/kind/cluster.yaml
    if ($LASTEXITCODE -ne 0) {
        Write-Host "M2 echec : kind create cluster" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}
else {
    Write-Host "Cluster aiforall-local deja present."
}

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host ("M2 echec : kubectl {0}" -f ($KubectlArgs -join ' ')) -ForegroundColor Red
        exit $code
    }
}

Invoke-Kubectl @('apply', '-k', 'deploy/apps/overlays/kind')
Invoke-Kubectl @('apply', '-k', 'deploy/p4')
Invoke-Kubectl @('wait', '--for=condition=available', 'deployment/lazaret', '-n', 'aiforall-gateway', '--timeout=180s')
Invoke-Kubectl @('delete', 'job', 'invoke-probe', '-n', 'aiforall-plugins', '--ignore-not-found=true')
Invoke-Kubectl @('apply', '-k', 'deploy/apps/overlays/kind')
Invoke-Kubectl @('wait', '--for=condition=complete', 'job/invoke-probe', '-n', 'aiforall-plugins', '--timeout=240s')

Write-Host "NetworkPolicy + Services :" -ForegroundColor Cyan
Invoke-Kubectl @('get', 'netpol,svc', '-A')

Write-Host "M2 OK : probe HTTP /lazaret/invoke = 200. Cluster conserve (kind delete cluster --name aiforall-local pour detruire)." -ForegroundColor Green
exit 0
