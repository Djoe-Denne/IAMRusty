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
    cargo test --lib -q 2>$null

# Run integration tests with database
test-integration:
    @echo "🧪 Running integration tests..."
    @try { $env:RUN_ENV="test"; cargo test --test integration_auth_test -q -- --nocapture 2>$null; $env:RUN_ENV="test"; cargo test --test integration_user_test -q -- --nocapture 2>$null; $env:RUN_ENV="test"; cargo test --test integration_database_test -q -- --nocapture 2>$null } finally { echo "🧹 Cleaning up test containers..."; just cleanup-containers }

# Run integration example tests
test-integration-examples:
    @echo "🧪 Running integration example tests..."
    @try { just test-example-http-fixtures; just test-example-db-fixtures; just test-example-combined-fixtures } finally { echo "🧹 Cleaning up test containers..."; just cleanup-containers }

# Run HTTP fixture examples
test-example-http-fixtures:
    @echo "🧪 Running HTTP fixture examples..."
    $env:RUN_ENV="test"; cargo test --test example_http_fixtures -q -- --nocapture 2>$null

# Run database fixture examples
test-example-db-fixtures:
    @echo "🧪 Running database fixture examples..."
    $env:RUN_ENV="test"; cargo test --test example_db_fixtures -q -- --nocapture 2>$null

# Run combined fixture examples
test-example-combined-fixtures:
    @echo "🧪 Running combined fixture examples..."
    $env:RUN_ENV="test"; cargo test --test example_combined_fixtures -q -- --nocapture 2>$null

# Run a specific example test
test-example TEST:
    @echo "🧪 Running specific example test: {{TEST}}"
    @try { $env:RUN_ENV="test"; cargo test --test example_http_fixtures {{TEST}} -q -- --nocapture 2>$null; $env:RUN_ENV="test"; cargo test --test example_db_fixtures {{TEST}} -q -- --nocapture 2>$null; $env:RUN_ENV="test"; cargo test --test example_combined_fixtures {{TEST}} -q -- --nocapture 2>$null } finally { just cleanup-containers }

# Clean up specific test container only
cleanup-containers:
    @echo "🧹 Stopping and removing test container 'iam-test-db'..."
    @try { docker stop iam-test-db 2>$null } catch { echo "Container already stopped or not found" }
    @try { docker rm iam-test-db 2>$null } catch { echo "Container already removed or not found" }
    @echo "✅ Test container cleanup completed"

# Run tests with verbose output
test-verbose:
    $env:RUST_LOG="debug"; cargo test --test integration_database_test -q -- --nocapture 2>$null

# Check for running test containers
check-containers:
    @echo "📋 Test container status:"
    @try { docker ps --filter "name=iam-test-db" } catch { echo "No test container found" }

# Run a single test with cleanup
test-single TEST:
    @echo "🧪 Running single test: {{TEST}}"
    @try { $env:RUN_ENV="test"; cargo test --test integration_auth_test {{TEST}} -q -- --nocapture 2>$null; $env:RUN_ENV="test"; cargo test --test integration_user_test {{TEST}} -q -- --nocapture 2>$null; $env:RUN_ENV="test"; cargo test --test integration_database_test {{TEST}} -q -- --nocapture 2>$null } finally { just cleanup-containers }

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
# Run authentication tests only
test-auth:
    @echo "🔐 Running authentication tests..."
    @try { $env:RUN_ENV="test"; cargo test --test integration_auth_test -q -- --nocapture 2>$null } finally { just cleanup-containers }

# Run user tests only
test-user:
    @echo "👤 Running user tests..."
    @try { $env:RUN_ENV="test"; cargo test --test integration_user_test -q -- --nocapture 2>$null } finally { just cleanup-containers }

# Run database tests only
test-db:
    @echo "🗄️ Running database tests..."
    @try { $env:RUN_ENV="test"; cargo test --test integration_database_test -q -- --nocapture 2>$null } finally { just cleanup-containers }

# Run OAuth start endpoint tests
test-oauth-start:
    $env:RUN_ENV="test"; cargo test --test integration_auth_test test_oauth_start -q -- --nocapture 2>$null

# Run OAuth callback endpoint tests
test-oauth-callback:
    $env:RUN_ENV="test"; cargo test --test integration_auth_test test_oauth_callback -q -- --nocapture 2>$null

# Run user profile endpoint tests
test-me:
    $env:RUN_ENV="test"; cargo test --test integration_user_test test_get_me -q -- --nocapture 2>$null

# Run provider token endpoint tests
test-provider-tokens:
    $env:RUN_ENV="test"; cargo test --test integration_user_test test_get_provider_token -q -- --nocapture 2>$null

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
    cargo watch -x "test --test integration_auth_oauth_flow -q 2>$null"

# 🧹 Cleanup
clean:
    cargo clean

clean-docker:
    docker container prune -f
    docker image prune -f

# 📊 Information
list-tests:
    cargo test --test integration_auth_oauth_flow -q -- --list 2>$null

check-deps:
    Write-Host "🔍 Checking dependencies..."
    if (!(Get-Command docker -ErrorAction SilentlyContinue)) { Write-Host "❌ Docker required"; exit 1 }
    if (!(Get-Command cargo -ErrorAction SilentlyContinue)) { Write-Host "❌ Cargo required"; exit 1 }
    docker ps *>$null; if ($LASTEXITCODE -ne 0) { Write-Host "❌ Docker not running"; exit 1 }
    Write-Host "✅ All dependencies available"

# 🚀 CI/CD
test-github:
    $env:GITHUB_ACTIONS="true"; $env:CI="true"; $env:TEST_VERBOSE="true"; $env:TEST_MAX_CONCURRENCY="2"; cargo test --test integration_auth_oauth_flow -q 2>$null

test-gitlab:
    $env:GITLAB_CI="true"; $env:CI="true"; $env:TEST_VERBOSE="true"; $env:TEST_MAX_CONCURRENCY="2"; cargo test --test integration_auth_oauth_flow -q 2>$null

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

# 🚀 Test Server Management
# Start the IAM server in background for integration testing
start-test-server:
    @echo "🚀 Starting IAM test server..."
    @$env:RUN_ENV="test"; $process = Start-Process -FilePath "cargo" -ArgumentList "run" -WindowStyle Hidden -PassThru; $process.Id | Out-File -FilePath ".test_server_pid" -Encoding ASCII
    @echo "⏳ Waiting for server to start..."
    @$attempts = 0; do { Start-Sleep -Seconds 1; $attempts++; try { $response = Invoke-WebRequest -Uri "http://127.0.0.1:8081/health" -TimeoutSec 2 -ErrorAction Stop; $ready = $true } catch { $ready = $false } } while (-not $ready -and $attempts -lt 30)
    @if ($ready) { echo "✅ Test server is running on http://127.0.0.1:8081" } else { echo "❌ Test server failed to start"; exit 1 }

# Stop the test server
stop-test-server:
    @echo "🛑 Stopping IAM test server..."
    @if (Test-Path ".test_server_pid") { $pid = Get-Content ".test_server_pid" -ErrorAction SilentlyContinue; if ($pid) { try { Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue } catch { } }; Remove-Item ".test_server_pid" -ErrorAction SilentlyContinue }
    @Get-Process -Name "cargo" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    @echo "✅ Test server stopped"

# Check if test server is running
check-test-server:
    @echo "🔍 Checking test server status..."
    @try { $response = Invoke-WebRequest -Uri "http://127.0.0.1:8081/health" -TimeoutSec 2; echo "✅ Server is running" } catch { echo "❌ Server is not responding" } 