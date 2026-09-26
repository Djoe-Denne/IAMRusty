# Catalog stub on host :9000, published via Docker so Kind pods can reach the
# Windows host the same way as Postgres (docker-proxy), not a raw host bind.
$ErrorActionPreference = "Stop"
$catalogPath = Join-Path $PSScriptRoot "component-catalog.json"
$serverJs = Join-Path $PSScriptRoot "component-catalog-server.js"
if (-not (Test-Path -LiteralPath $catalogPath)) {
    throw "Missing catalog JSON: $catalogPath"
}
$name = "aiforall-component-catalog"
cmd /c "docker rm -f $name >NUL 2>&1"
Write-Host "component catalog stub (docker -p 9000:9000) /api/components" -ForegroundColor Cyan
& docker run --rm --name $name -p 9000:9000 `
    -v "${catalogPath}:/catalog.json:ro" `
    -v "${serverJs}:/server.js:ro" `
    -e CATALOG_JSON=/catalog.json `
    -e CATALOG_PORT=9000 `
    -e CATALOG_TOKEN=aiforall-gold-catalog `
    node:20-alpine node /server.js
if ($LASTEXITCODE -ne 0) {
    throw "docker catalog exited $LASTEXITCODE"
}
