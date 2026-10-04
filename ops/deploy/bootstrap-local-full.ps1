#Requires -Version 5.1
param([Parameter(Mandatory = $true)][string]$LeasePath, [switch]$AfterFinalIT)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
Set-Location (Get-AIForAllRoot $PSScriptRoot)
Initialize-LocalKind -Cluster 'aiforall-local-full' -Config 'ops/deploy/kind/cluster-local-full.yaml' -LeasePath $LeasePath -AfterFinalIT:$AfterFinalIT
