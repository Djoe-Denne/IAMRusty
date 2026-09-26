# AIForAll - Root Task Runner
# Install: cargo install just
# Usage: just <task>
#
# This justfile orchestrates the full local docker-compose stack:
#   - postgres, localstack, openfga, openbao (infrastructure)
#   - create-databases, openfga-migrate, build-artifacts (one-shot init)
#   - iam-service, telegraph-service, hive-service, manifesto-service (apps)
# Host monolith (J1): `just up-infra` then `just monolith` — never builds apps.
# Host FGA worker: `just sentinel-sync` after queues-on + monolith restart (not nested).
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

# Infra only for the host monolith (J1). Explicit service list — never
# `build-artifacts` and never iam/telegraph/hive/manifesto/lazaret-service.
# Port 8080 stays free so `just monolith` can bind. Does not change `just up`.
up-infra:
    @Write-Host "Starting compose infrastructure only (no Rust apps, no build-artifacts)..." -ForegroundColor Cyan
    docker compose up -d postgres create-databases openfga-migrate openfga localstack openbao openbao-seed platform-mesh-certs
    @Write-Host "Infra is up. Host chain: just monolith (restart if queues were off) then just sentinel-sync." -ForegroundColor Green
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

# Host oodhive-monolith (J1). Needs `just up-infra`. Binds 0.0.0.0:8080 —
# stop compose iam-service if that port is taken.
# If this process booted with [queue] enabled=false, RESTART it after enabling queues.
monolith:
    & ./monolith/run-host.ps1

# Preuves — index :
#   just prove-gold                  = nominal Kind 0605
#   just prove-j1 / monolith-prove   = host J1
#   monolith/prove-e2e-curl.ps1      = curl hôte ≠ preuve
monolith-prove:
    & ./monolith/prove-j1.ps1

alias prove-j1 := monolith-prove

# Host sentinel-sync worker (ADR-0303 / 0404). Separate process — not nested in oodhive-monolith.
# CWD repo root. Loader reads `config/sentinel-sync.toml` + SENTINEL_SYNC_* (RUN_ENV is unused here).
# Order: just up-infra → queues on → just monolith (restart) → just sentinel-sync.
sentinel-sync:
    $env:RUN_ENV = "development"
    . ./openfga/ensure-host-store.ps1
    cargo run -p sentinel-sync

# Host Manifesto component catalog stub (GET http://127.0.0.1:9000/api/components).
# Needed for prove-e2e-curl POST .../components after the OpenFGA Admin grant.
component-catalog:
    & ./monolith/run-component-catalog.ps1

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

# === OpenFGA model ========================================================

# Regenerate openfga/model.json from openfga/model.fga.
#
# The integration-test fixture (rustycog-testing TestOpenFga) uploads
# openfga/model.json into the test container at startup. Whenever
# openfga/model.fga changes, run this recipe and commit the regenerated
# JSON alongside the DSL update.
#
# Uses the `fga` CLI when present on PATH; otherwise falls back to a
# one-shot `docker run openfga/cli` so contributors do not need a local
# install.
regenerate-openfga-model-json:
    @if (Get-Command fga -ErrorAction SilentlyContinue) { \
        Write-Host "Using local fga CLI" -ForegroundColor Cyan; \
        fga model transform --file openfga/model.fga | Set-Content -Path openfga/model.json; \
    } else { \
        Write-Host "Local fga CLI not found; running openfga/cli via docker" -ForegroundColor Cyan; \
        docker run --rm -v "${PWD}/openfga:/work" -w /work openfga/cli model transform --file model.fga | Set-Content -Path openfga/model.json; \
    }
    @Write-Host "Wrote openfga/model.json" -ForegroundColor Green

# === Local Kubernetes (ADR-0603) ===========================================
# Kind cluster aiforall-local — never apparatus-p4-it. Does not replace Compose `up`.

deploy-m1:
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; & ./deploy/verify-m1.ps1

deploy-m2:
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; & ./deploy/verify-m2.ps1

deploy-m3:
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; & ./deploy/verify-m3.ps1

# Real apparatus-controller on kind aiforall-local (J2). Never apparatus-p4-it.
# Does not bind host :8080. Does not replace Compose `up` / `up-infra` / `monolith`.
deploy-j2:
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; & ./deploy/deploy-j2.ps1

# Demo overlay: oodhive-monolith on kind aiforall-local (J3 / ADR-0604).
# Distinct from deploy/apps/overlays/kind (M2 nginx stub). Not 0601 canon.
# Needs `just up-infra` (Postgres/OpenFGA on the Windows host via host.docker.internal).
# Never apparatus-p4-it. Does not bind host :8080.
deploy-j3:
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; & ./deploy/deploy-j3.ps1

# Dette hors gold 0605 / 0008 — schedule manuel + Job enroll (0604). Pas une étape nominale.
debt-schedule-reference-kv:
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; & ./deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1

# Preuves — index :
#   just prove-gold                  = nominal Kind 0605
#   just prove-j1 / monolith-prove   = host J1
#   monolith/prove-e2e-curl.ps1      = curl hôte ≠ preuve
prove-gold:
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; & ./deploy/apps/overlays/kind-demo-monolith/prove-gold-path.ps1

