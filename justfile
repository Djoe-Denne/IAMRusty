# AIForAll - Root Task Runner
# Install: cargo install just
# Usage: just <task>
#
# This justfile orchestrates the full local docker-compose stack:
#   - postgres, localstack, openfga (infrastructure)
#   - create-databases, openfga-migrate, build-artifacts (one-shot init)
#   - iam-service, telegraph-service, hive-service, manifesto-service (apps)
#
# The four application Dockerfiles all start with
#   FROM local/build-artifacts:latest
# so the artifacts image MUST be built BEFORE Compose tries to build the app
# images. `depends_on` only orders runtime startup, not builds, so we build
# in two explicit steps.

# Use PowerShell on Windows (matches IAMRusty/justfile convention)
set shell := ["powershell.exe", "-c"]

# Default - list available recipes
default:
    @just --list

# === Stack lifecycle =======================================================

# Bring up the full stack: builds the shared artifacts image first, then
# builds + starts every infra and application service in dependency order.
# After this completes you can hit:
#   http://localhost:8080  IAMRusty
#   http://localhost:8081  Telegraph
#   http://localhost:8082  Hive
#   http://localhost:8083  Manifesto
#   http://localhost:8090  OpenFGA HTTP API
#   http://localhost:3000  OpenFGA Playground
up:
    @Write-Host "[1/3] Building shared build-artifacts image (compiles all Rust binaries)..." -ForegroundColor Cyan
    docker compose build build-artifacts
    @Write-Host "[2/3] Building application service images..." -ForegroundColor Cyan
    docker compose build iam-service telegraph-service hive-service manifesto-service
    @Write-Host "[3/3] Starting infrastructure + services..." -ForegroundColor Cyan
    docker compose up -d
    @Write-Host "Stack is up. Tail logs with: just logs" -ForegroundColor Green
    @just ps

# Same as `up` but rebuild every image from scratch (no cache).
rebuild:
    @Write-Host "Rebuilding every image with --no-cache..." -ForegroundColor Cyan
    docker compose build --no-cache build-artifacts
    docker compose build --no-cache iam-service telegraph-service hive-service manifesto-service
    docker compose up -d --force-recreate
    @just ps

# Stop and remove all containers (volumes preserved).
down:
    docker compose down

# Stop and remove all containers AND named volumes (drops Postgres data,
# OpenFGA store, etc.).
nuke:
    docker compose down -v

# Restart the whole stack.
restart: down up

# === Observability =========================================================

# Show every container managed by this compose project.
ps:
    docker compose ps

# Tail logs from every service. Pass a service name to scope it:
#   just logs hive-service
logs *SERVICE:
    docker compose logs -f {{SERVICE}}

# Status check: hit /health on every application service.
health:
    @Write-Host "Checking IAM (8080)..."        ; try { Invoke-WebRequest -Uri http://127.0.0.1:8080/health -TimeoutSec 2 -UseBasicParsing | Select-Object -ExpandProperty StatusCode } catch { Write-Host "  down" -ForegroundColor Red }
    @Write-Host "Checking Telegraph (8081)..."  ; try { Invoke-WebRequest -Uri http://127.0.0.1:8081/health -TimeoutSec 2 -UseBasicParsing | Select-Object -ExpandProperty StatusCode } catch { Write-Host "  down" -ForegroundColor Red }
    @Write-Host "Checking Hive (8082)..."       ; try { Invoke-WebRequest -Uri http://127.0.0.1:8082/health -TimeoutSec 2 -UseBasicParsing | Select-Object -ExpandProperty StatusCode } catch { Write-Host "  down" -ForegroundColor Red }
    @Write-Host "Checking Manifesto (8083)..."  ; try { Invoke-WebRequest -Uri http://127.0.0.1:8083/health -TimeoutSec 2 -UseBasicParsing | Select-Object -ExpandProperty StatusCode } catch { Write-Host "  down" -ForegroundColor Red }
    @Write-Host "Checking OpenFGA (8090)..."    ; try { Invoke-WebRequest -Uri http://127.0.0.1:8090/healthz -TimeoutSec 2 -UseBasicParsing | Select-Object -ExpandProperty StatusCode } catch { Write-Host "  down" -ForegroundColor Red }

# === Database tooling ======================================================
# These wrap the `tools` profile so they don't run with `docker compose up`.

# List every database in the shared postgres instance.
db-list:
    docker compose --profile tools run --rm list-databases

# Truncate the configured TARGET_DB (default: iam_dev).
# Override with: just db-truncate hive_dev
db-truncate TARGET_DB="iam_dev":
    $env:TARGET_DB="{{TARGET_DB}}"; docker compose --profile tools run --rm truncate-db

# Mark every email as verified in TARGET_DB (default: iam_dev).
db-verify-emails TARGET_DB="iam_dev":
    $env:TARGET_DB="{{TARGET_DB}}"; docker compose --profile tools run --rm verify-emails
