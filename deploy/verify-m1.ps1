#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

function Test-HasCommand {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

$script:Builder = $null
if (Test-HasCommand 'kubectl') {
    & kubectl kustomize --help 1>$null 2>$null
    if ($LASTEXITCODE -eq 0) {
        $script:Builder = 'kubectl'
    }
}
if ($null -eq $script:Builder -and (Test-HasCommand 'kustomize')) {
    $script:Builder = 'kustomize'
}

if ($null -eq $script:Builder) {
    Write-Host "M1 echec : ni 'kubectl kustomize' ni 'kustomize' sur le PATH." -ForegroundColor Red
    Write-Host "Installer kubectl (https://kubernetes.io/docs/tasks/tools/) ou le binaire kustomize, puis relancer : just deploy-m1"
    exit 1
}

Write-Host "M1 : build kustomize via $script:Builder" -ForegroundColor Cyan

function Invoke-KustomizeBuild {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string[]]$MustContain
    )
    Write-Host "  build $Path"
    if ($script:Builder -eq 'kubectl') {
        $generated = & kubectl kustomize $Path
    }
    else {
        $generated = & kustomize build $Path
    }
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host "M1 echec : kustomize build $Path (exit $code)" -ForegroundColor Red
        exit $code
    }
    $text = ($generated | Out-String)
    if ($text.Trim().Length -lt 20) {
        Write-Host "M1 echec : build $Path a produit un YAML vide" -ForegroundColor Red
        exit 1
    }
    foreach ($needle in $MustContain) {
        if ($text -notmatch [regex]::Escape($needle)) {
            Write-Host "M1 echec : build $Path sans '$needle'" -ForegroundColor Red
            exit 1
        }
    }
}

Invoke-KustomizeBuild 'deploy/apps/overlays/kind' @(
    'aiforall-platform',
    'aiforall-gateway',
    'aiforall-apparatus',
    'aiforall-plugins',
    'aiforall-data',
    'aiforall-secrets',
    'aiforall-gitops',
    'invoke-probe',
    '/lazaret/invoke'
)
Invoke-KustomizeBuild 'deploy/p4' @(
    'apparatus-operator',
    'aiforall-apparatus',
    'controller'
)

Write-Host "M1 OK : overlays kind + deploy/p4 sont sains (pas de cluster requis)." -ForegroundColor Green
Write-Host "OpenTofu : non exige pour M1. Voir cloud/opentofu/README.md (tofu validate sans credentials)."
exit 0
