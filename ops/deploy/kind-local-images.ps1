# Thin Windows adapter for the native Bake and Kind CLIs, no deployment state.
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('build', 'import')]
    [string]$Action,
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$')]
    [string]$BuildTag
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$previousTag = $env:BUILD_TAG
Push-Location ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')))
try {
    $env:BUILD_TAG = $BuildTag
    $definition = @('buildx', 'bake', '-f', 'ops/deploy/build/targets.json',
                    '-f', 'ops/deploy/build/docker-bake.hcl')
    if ($Action -eq 'build') {
        & docker.exe @definition --no-cache --load kind-local-images
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } else {
        $rendered = & docker.exe @definition --print kind-local-images
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $plan = ($rendered -join "`n") | ConvertFrom-Json
        $images = @(foreach ($name in $plan.group.'kind-local-images'.targets) {
            $plan.target.$name.tags
        })
        if ($images.Count -eq 0) { throw 'The native Bake plan contains no application images.' }
        & kind.exe load docker-image @images --name aiforall-local-full
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
} finally {
    if ($null -eq $previousTag) { Remove-Item Env:BUILD_TAG -ErrorAction SilentlyContinue }
    else { $env:BUILD_TAG = $previousTag }
    Pop-Location
}
