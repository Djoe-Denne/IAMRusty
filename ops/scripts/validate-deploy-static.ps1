#Requires -Version 5.1
# Static only: no Docker, Kind, kubectl, cargo or secret reads.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../deploy/common.ps1')
$root = Get-AIForAllRoot $PSScriptRoot
$failures = @()
foreach ($script in (Get-ChildItem -LiteralPath (Join-Path $root 'ops/deploy') -Recurse -Filter '*.ps1')) {
    $tokens = $null
    $errors = $null
    [void][Management.Automation.Language.Parser]::ParseFile($script.FullName, [ref]$tokens, [ref]$errors)
    foreach ($error in $errors) { $failures += "$($script.Name):$($error.Extent.StartLineNumber): $($error.Message)" }
    if ((Get-AIForAllRoot $script.DirectoryName).Path -ne $root.Path) { $failures += "Wrong repository root: $($script.FullName)" }
}
foreach ($relative in @(
    'workers/apparatus-operator/Dockerfile.controller',
    'workers/ext-authz/Dockerfile',
    'runtime/monolith/Dockerfile',
    'ops/scripts/generate-platform-mesh-certs.sh',
    'ops/deploy/kind/cluster.yaml',
    'ops/deploy/kind/cluster-local-full.yaml',
    'ops/deploy/apps/overlays/kind/kustomization.yaml',
    'ops/deploy/apps/overlays/kind-mesh/kustomization.yaml',
    'ops/deploy/apps/overlays/kind-demo-monolith/kustomization.yaml',
    'crates/apparatus-contracts/Cargo.toml',
    'crates/apparatus-reference-kv/Dockerfile.http'
)) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) { $failures += "Missing nominal path: $relative" }
}
if ($failures.Count -gt 0) { throw ($failures -join "`n") }
Write-Host 'PASS: deployment PowerShell syntax, root discovery and nominal paths (no runtime used).'
