# Host OpenFGA store + LocalStack `sentinel-sync-events`.
# Dot-source from `just monolith` / `just sentinel-sync` so env overlays stick.
# Compose `openfga-migrate` only creates the Postgres schema — not a store.
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$api = "http://127.0.0.1:8090"
$modelPath = Join-Path $PSScriptRoot "model.json"

function Test-OpenFgaUp {
    try {
        $code = & curl.exe -sS -o NUL -w "%{http_code}" --max-time 3 "$api/healthz"
        return "$code" -eq "200"
    } catch {
        return $false
    }
}

if (-not (Test-OpenFgaUp)) {
    throw "OpenFGA is not reachable at $api/healthz. Run: just up-infra"
}

if (-not (Test-Path -LiteralPath $modelPath)) {
    throw "Missing OpenFGA model JSON: $modelPath"
}

$storesJson = & curl.exe -sS --max-time 10 "$api/stores"
if ($LASTEXITCODE -ne 0 -or -not $storesJson) {
    throw "GET $api/stores failed"
}
$storesDoc = $storesJson | ConvertFrom-Json
$storeList = @()
if ($storesDoc.stores) {
    $storeList = @($storesDoc.stores)
}
$store = $storeList | Where-Object { $_.name -eq "aiforall" } | Select-Object -First 1
if (-not $store) {
    $store = $storeList | Select-Object -First 1
}
if (-not $store) {
    $createBody = Join-Path $env:TEMP ("openfga-create-store-" + [guid]::NewGuid().ToString() + ".json")
    [System.IO.File]::WriteAllText($createBody, '{"name":"aiforall"}')
    try {
        $created = & curl.exe -sS --max-time 15 -X POST "$api/stores" -H "Content-Type: application/json" --data-binary "@$createBody"
        if ($LASTEXITCODE -ne 0 -or -not $created) {
            throw "POST $api/stores failed: $created"
        }
        $store = $created | ConvertFrom-Json
    } finally {
        Remove-Item -LiteralPath $createBody -Force -ErrorAction SilentlyContinue
    }
}
$storeId = [string]$store.id
if (-not $storeId) {
    throw "OpenFGA store id missing after list/create"
}

$modelsJson = & curl.exe -sS --max-time 10 "$api/stores/$storeId/authorization-models"
if ($LASTEXITCODE -ne 0 -or -not $modelsJson) {
    throw "GET authorization-models failed"
}
$modelsDoc = $modelsJson | ConvertFrom-Json
$modelList = @()
if ($modelsDoc.authorization_models) {
    $modelList = @($modelsDoc.authorization_models)
}
$modelId = $null
if ($modelList.Count -gt 0) {
    $modelId = [string]$modelList[0].id
}
if (-not $modelId) {
    $written = & curl.exe -sS --max-time 30 -X POST "$api/stores/$storeId/authorization-models" -H "Content-Type: application/json" --data-binary "@$modelPath"
    if ($LASTEXITCODE -ne 0 -or -not $written) {
        throw "POST authorization-models failed: $written"
    }
    $writtenDoc = $written | ConvertFrom-Json
    $modelId = [string]$writtenDoc.authorization_model_id
}
if (-not $modelId) {
    throw "OpenFGA authorization_model_id missing after list/write"
}

$env:MANIFESTO_OPENFGA__STORE_ID = $storeId
$env:MANIFESTO_OPENFGA__AUTHORIZATION_MODEL_ID = $modelId
$env:HIVE_OPENFGA__STORE_ID = $storeId
$env:HIVE_OPENFGA__AUTHORIZATION_MODEL_ID = $modelId
$env:TELEGRAPH_OPENFGA__STORE_ID = $storeId
$env:TELEGRAPH_OPENFGA__AUTHORIZATION_MODEL_ID = $modelId
$env:SENTINEL_SYNC_OPENFGA__STORE_ID = $storeId
$env:SENTINEL_SYNC_OPENFGA__AUTHORIZATION_MODEL_ID = $modelId
$env:SENTINEL_SYNC_OPENFGA__SCHEME = "http"
$env:SENTINEL_SYNC_OPENFGA__HOST = "127.0.0.1"
$env:SENTINEL_SYNC_OPENFGA__PORT = "8090"

$envFile = Join-Path $repoRoot ".env.openfga"
@(
    "MANIFESTO_OPENFGA__STORE_ID=$storeId"
    "MANIFESTO_OPENFGA__AUTHORIZATION_MODEL_ID=$modelId"
    "HIVE_OPENFGA__STORE_ID=$storeId"
    "HIVE_OPENFGA__AUTHORIZATION_MODEL_ID=$modelId"
    "TELEGRAPH_OPENFGA__STORE_ID=$storeId"
    "TELEGRAPH_OPENFGA__AUTHORIZATION_MODEL_ID=$modelId"
    "SENTINEL_SYNC_OPENFGA__STORE_ID=$storeId"
    "SENTINEL_SYNC_OPENFGA__AUTHORIZATION_MODEL_ID=$modelId"
    "SENTINEL_SYNC_OPENFGA__SCHEME=http"
    "SENTINEL_SYNC_OPENFGA__HOST=127.0.0.1"
    "SENTINEL_SYNC_OPENFGA__PORT=8090"
) | Set-Content -LiteralPath $envFile -Encoding utf8

Write-Host "OpenFGA store_id=$storeId model_id=$modelId (wrote $envFile)" -ForegroundColor Green

Push-Location $repoRoot
try {
    docker compose exec -T localstack awslocal sqs create-queue --queue-name sentinel-sync-events | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "sentinel-sync-events CreateQueue exit=$LASTEXITCODE (ok if the queue already exists)" -ForegroundColor Yellow
    } else {
        Write-Host "LocalStack queue sentinel-sync-events ready" -ForegroundColor Green
    }
} finally {
    Pop-Location
}
