#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

if ($null -eq (Get-Command helm -ErrorAction SilentlyContinue)) {
    Write-Host "M3 skip : helm absent ; le shim reste dans deploy/third-party/envoy-gateway/"
    exit 0
}

Write-Host "M3 : helm lint deploy/third-party/envoy-gateway" -ForegroundColor Cyan
& helm lint deploy/third-party/envoy-gateway
if ($LASTEXITCODE -ne 0) {
    Write-Host "M3 echec : helm lint" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "M3 OK : shim Envoy Gateway lintable (pas d'install Flux/GKE)." -ForegroundColor Green
exit 0
