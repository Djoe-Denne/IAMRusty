# Host boot for oodhive-monolith (J1).
# Requires: `just up-infra`. Port 8080 must be free (do not start iam-service).
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

# rustycog-config selects config/{RUN_ENV}.toml (ADR-0500). Pin development so a
# leftover RUN_ENV=test (CI / shell) does not skip config/development.toml.
$env:RUN_ENV = "development"

. (Join-Path $repoRoot "openfga/ensure-host-store.ps1")

Write-Host "oodhive-monolith listens on 0.0.0.0:8080 (tls_enabled=false). RUN_ENV=development." -ForegroundColor Cyan
Write-Host "Stop compose iam-service if it already occupies 8080." -ForegroundColor Yellow
Write-Host "Manifesto SQS on; IAM/Telegraph/Hive/Lazaret queue consumers stay off (do not steal sentinel-sync-events)." -ForegroundColor Yellow

$env:IAM_DATABASE__HOST = "127.0.0.1"
$env:IAM_DATABASE__PORT = "5432"
$env:IAM_DATABASE__DB = "iam_dev"
$env:IAM_DATABASE__CREDS__USERNAME = "postgres"
$env:IAM_DATABASE__CREDS__PASSWORD = "postgres"
$env:TELEGRAPH_DATABASE__HOST = "127.0.0.1"
$env:TELEGRAPH_DATABASE__PORT = "5432"
$env:TELEGRAPH_DATABASE__DB = "telegraph_dev"
$env:TELEGRAPH_DATABASE__CREDS__USERNAME = "postgres"
$env:TELEGRAPH_DATABASE__CREDS__PASSWORD = "postgres"
$env:HIVE_DATABASE__HOST = "127.0.0.1"
$env:HIVE_DATABASE__PORT = "5432"
$env:HIVE_DATABASE__DB = "hive_dev"
$env:HIVE_DATABASE__CREDS__USERNAME = "postgres"
$env:HIVE_DATABASE__CREDS__PASSWORD = "postgres"
$env:MANIFESTO_DATABASE__HOST = "127.0.0.1"
$env:MANIFESTO_DATABASE__PORT = "5432"
$env:MANIFESTO_DATABASE__DB = "manifesto_dev"
$env:MANIFESTO_DATABASE__CREDS__USERNAME = "postgres"
$env:MANIFESTO_DATABASE__CREDS__PASSWORD = "postgres"
$env:LAZARET_DATABASE__HOST = "127.0.0.1"
$env:LAZARET_DATABASE__PORT = "5432"
$env:LAZARET_DATABASE__DB = "lazaret_dev"
$env:LAZARET_DATABASE__CREDS__USERNAME = "postgres"
$env:LAZARET_DATABASE__CREDS__PASSWORD = "postgres"

$env:MANIFESTO_QUEUE__ENABLED = "true"
$env:IAM_QUEUE__ENABLED = "false"
$env:TELEGRAPH_QUEUE__ENABLED = "false"
$env:HIVE_QUEUE__ENABLED = "false"
$env:LAZARET_QUEUE__ENABLED = "false"

cargo run -p oodhive-monolith @args
exit $LASTEXITCODE
