# IAM Service - Modern Task Runner
# Install: cargo install just
# Usage: just <task>

# Set Windows-compatible shell
set shell := ["powershell.exe", "-c"]

# Default task - show available commands
default:
    @just --list

# 🧪 Testing Tasks
test: test-unit test-integration cleanup-containers

# Run all tests including examples
test-all: test test-integration-examples

# Run unit tests only
test-unit:
    cargo test --lib

# Run integration tests with database
test-integration:
    @echo "🧪 Running integration tests..."
    cargo test --test integration_database_test -- --nocapture
    @echo "🧹 Cleaning up test containers..."
    @just cleanup-containers

# Run integration example tests
test-integration-examples:
    @echo "🧪 Running integration example tests..."
    @just test-example-http-fixtures
    @just test-example-db-fixtures
    @just test-example-combined-fixtures
    @echo "🧹 Cleaning up test containers..."
    @just cleanup-containers

# Run HTTP fixture examples
test-example-http-fixtures:
    @echo "🧪 Running HTTP fixture examples..."
    cargo test --test example_http_fixtures -- --nocapture

# Run database fixture examples
test-example-db-fixtures:
    @echo "🧪 Running database fixture examples..."
    cargo test --test example_db_fixtures -- --nocapture

# Run combined fixture examples
test-example-combined-fixtures:
    @echo "🧪 Running combined fixture examples..."
    cargo test --test example_combined_fixtures -- --nocapture

# Run a specific example test
test-example TEST:
    @echo "🧪 Running specific example test: {{TEST}}"
    cargo test --test example_http_fixtures {{TEST}} -- --nocapture
    cargo test --test example_db_fixtures {{TEST}} -- --nocapture
    cargo test --test example_combined_fixtures {{TEST}} -- --nocapture
    @just cleanup-containers

# Clean up specific test container only
cleanup-containers:
    @echo "🧹 Stopping and removing test container 'iam-test-db'..."
    @try { docker stop iam-test-db 2>$null } catch { echo "Container already stopped or not found" }
    @try { docker rm iam-test-db 2>$null } catch { echo "Container already removed or not found" }
    @echo "✅ Test container cleanup completed"

# Run tests with verbose output
test-verbose:
    $env:RUST_LOG="debug"; cargo test --test integration_database_test -- --nocapture

# Check for running test containers
check-containers:
    @echo "📋 Test container status:"
    @try { docker ps --filter "name=iam-test-db" } catch { echo "No test container found" }

# Run a single test with cleanup
test-single TEST:
    @echo "🧪 Running single test: {{TEST}}"
    cargo test --test integration_database_test {{TEST}} -- --nocapture
    @just cleanup-containers

# Run tests and show container status before/after
test-with-status:
    @echo "📋 Container status BEFORE tests:"
    @just check-containers
    @echo ""
    @just test
    @echo ""
    @echo "📋 Container status AFTER cleanup:"
    @just check-containers

# Force cleanup all postgres containers (use with caution)
cleanup-all-postgres:
    @echo "⚠️  Stopping ALL postgres containers..."
    @$containers = docker ps -q --filter "ancestor=postgres"; if ($containers) { $containers | ForEach-Object { docker stop $_ } }
    @$containers = docker ps -aq --filter "ancestor=postgres"; if ($containers) { $containers | ForEach-Object { docker rm $_ } }
    @echo "✅ All postgres containers cleaned up"

# 🎯 Test Groups
test-start:
    cargo test --test integration_auth_oauth_flow test_oauth_start

test-callback:
    cargo test --test integration_auth_oauth_flow test_oauth_callback

test-state:
    cargo test --test integration_auth_oauth_flow test_oauth_state

# 📚 Example Tests
# Run all example tests
examples: test-integration-examples

# Run specific example categories
examples-http: test-example-http-fixtures
examples-db: test-example-db-fixtures
examples-combined: test-example-combined-fixtures

# 🔧 Development
dev-setup:
    @echo "🔧 Setting up development environment..."
    cargo build
    @echo "✅ Development setup completed"

# Run migrations
migrate:
    cd migration && cargo run

# Reset database (for development)
reset-db:
    cd migration && cargo run -- down
    cd migration && cargo run -- up

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
dev: dev-setup db-up db-migrate
    Write-Host "🎉 Development environment ready!"
    Write-Host "Run: just test-integration" 