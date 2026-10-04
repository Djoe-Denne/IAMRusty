#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
$RepoRoot = Get-AIForAllRoot $PSScriptRoot
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

Initialize-LocalKind

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host ("M2 echec : kubectl {0}" -f ($KubectlArgs -join ' ')) -ForegroundColor Red
        exit $code
    }
}

Invoke-Kubectl @('apply', '-k', 'ops/deploy/apps/overlays/kind')
Invoke-Kubectl @('apply', '-k', 'ops/deploy/p4')
Invoke-Kubectl @('wait', '--for=condition=available', 'deployment/lazaret', '-n', 'aiforall-gateway', '--timeout=180s')
Invoke-Kubectl @('delete', 'job', 'invoke-probe', '-n', 'aiforall-plugins', '--ignore-not-found=true')
Invoke-Kubectl @('apply', '-k', 'ops/deploy/apps/overlays/kind')
Invoke-Kubectl @('wait', '--for=condition=complete', 'job/invoke-probe', '-n', 'aiforall-plugins', '--timeout=240s')

Write-Host "NetworkPolicy + Services :" -ForegroundColor Cyan
Invoke-Kubectl @('get', 'netpol,svc', '-A')

Write-Host "M2 OK : probe HTTP /lazaret/invoke = 200. Ressources conservees sous bail du parent." -ForegroundColor Green
exit 0
