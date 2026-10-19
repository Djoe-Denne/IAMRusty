#Requires -Version 5.1
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('iam', 'hive', 'manifesto', 'telegraph', 'lazaret')]
    [string]$Suite
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../deploy/common.ps1')
$root = Get-AIForAllRoot $PSScriptRoot
$packages = @{
    iam = 'iam-service'
    hive = 'hive-service'
    manifesto = 'manifesto-service'
    telegraph = 'telegraph-service'
    lazaret = 'lazaret-service'
}
# The caller owns the single Cargo slot and the testcontainers lifecycle lease.
Get-Command cargo -ErrorAction Stop | Out-Null
$previousTarget = $env:CARGO_TARGET_DIR
$env:CARGO_TARGET_DIR = 'E:\cargo-target\AIForAll'
Push-Location $root
try {
    & cargo test -p $packages[$Suite] --tests
    if ($LASTEXITCODE -ne 0) { throw "Native integration suite failed: $Suite (exit $LASTEXITCODE)" }
}
finally {
    Pop-Location
    $env:CARGO_TARGET_DIR = $previousTarget
}
