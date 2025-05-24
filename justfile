# IAM Service - Modern Task Runner
# Install: cargo install just
# Usage: just <task>

# Set Windows-compatible shell
set shell := ["powershell.exe", "-c"]

# Default task - show available commands
default:
    just --list

# 🧪 Testing Tasks
test:
    cargo test

# Run OAuth integration tests
test-integration:
    cargo test --test integration_auth_oauth_flow

# Run OAuth specific tests  
test-oauth:
    cargo test --test integration_auth_oauth_flow oauth

# Run a single test with debugging
test-single TEST:
    $env:RUST_LOG="debug"; cargo test --test integration_auth_oauth_flow {{TEST}} -- --nocapture --exact

# Run tests with verbose debugging
test-debug:
    $env:RUST_LOG="debug"; $env:TEST_VERBOSE="true"; cargo test --test integration_auth_oauth_flow -- --nocapture

# Run tests optimized for CI
test-ci:
    $env:CI="true"; $env:TEST_VERBOSE="true"; $env:TEST_MAX_CONCURRENCY="2"; cargo test --test integration_auth_oauth_flow

# Quick tests for local development
test-quick:
    $env:TEST_DB_TIMEOUT="10"; $env:TEST_DB_RETRIES="10"; cargo test --test integration_auth_oauth_flow

# 🎯 Test Groups
test-start:
    cargo test --test integration_auth_oauth_flow test_oauth_start

test-callback:
    cargo test --test integration_auth_oauth_flow test_oauth_callback

test-state:
    cargo test --test integration_auth_oauth_flow test_oauth_state

# 🔧 Development
setup:
    Write-Host "🚀 Setting up development environment..."
    cargo install cargo-watch
    cargo build --tests
    Write-Host "✅ Ready!"

# Watch tests and re-run on changes
watch:
    cargo watch -x "test --test integration_auth_oauth_flow"

# 🧹 Cleanup
clean:
    cargo clean

clean-docker:
    docker container prune -f
    docker image prune -f

# 📊 Information
list-tests:
    cargo test --test integration_auth_oauth_flow -- --list

check-deps:
    Write-Host "🔍 Checking dependencies..."
    if (!(Get-Command docker -ErrorAction SilentlyContinue)) { Write-Host "❌ Docker required"; exit 1 }
    if (!(Get-Command cargo -ErrorAction SilentlyContinue)) { Write-Host "❌ Cargo required"; exit 1 }
    docker ps *>$null; if ($LASTEXITCODE -ne 0) { Write-Host "❌ Docker not running"; exit 1 }
    Write-Host "✅ All dependencies available"

# 🚀 CI/CD
test-github:
    $env:GITHUB_ACTIONS="true"; $env:CI="true"; $env:TEST_VERBOSE="true"; $env:TEST_MAX_CONCURRENCY="2"; cargo test --test integration_auth_oauth_flow

test-gitlab:
    $env:GITLAB_CI="true"; $env:CI="true"; $env:TEST_VERBOSE="true"; $env:TEST_MAX_CONCURRENCY="2"; cargo test --test integration_auth_oauth_flow

# 🔄 Database Tasks
db-up:
    Write-Host "🐘 Starting PostgreSQL..."
    docker-compose up postgres -d

db-migrate:
    cd migration; cargo run -- up

db-reset:
    cd migration; cargo run -- down; cargo run -- up

# 🏃 Quick Start Combo
dev: setup db-up db-migrate
    Write-Host "🎉 Development environment ready!"
    Write-Host "Run: just test-integration" 