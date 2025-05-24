# Makefile for IAM Service Integration Tests

.PHONY: help test test-integration test-oauth test-verbose test-ci clean docker-clean

# Default target
help: ## Show this help message
	@echo "IAM Service Integration Tests"
	@echo "=============================="
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

test: ## Run all tests
	cargo test

test-integration: ## Run integration tests only
	cargo test --test integration_auth_oauth_flow

test-oauth: ## Run OAuth specific tests
	cargo test --test integration_auth_oauth_flow oauth

test-verbose: ## Run integration tests with verbose output
	RUST_LOG=debug TEST_VERBOSE=true cargo test --test integration_auth_oauth_flow -- --nocapture

test-ci: ## Run tests with CI optimizations
	CI=true TEST_VERBOSE=true TEST_MAX_CONCURRENCY=2 cargo test --test integration_auth_oauth_flow

test-single: ## Run a single test (usage: make test-single TEST=test_name)
	@if [ -z "$(TEST)" ]; then \
		echo "Usage: make test-single TEST=test_name"; \
		echo "Example: make test-single TEST=test_oauth_start_github_redirects_properly"; \
		exit 1; \
	fi
	RUST_LOG=debug cargo test --test integration_auth_oauth_flow $(TEST) -- --nocapture --exact

test-quick: ## Run tests with minimal setup (local development)
	TEST_DB_TIMEOUT=10 TEST_DB_RETRIES=10 cargo test --test integration_auth_oauth_flow

test-stress: ## Run tests with high concurrency to check for race conditions
	TEST_MAX_CONCURRENCY=8 cargo test --test integration_auth_oauth_flow -- --test-threads=8

clean: ## Clean build artifacts
	cargo clean

docker-clean: ## Clean Docker containers and images
	docker container prune -f
	docker image prune -f

check-deps: ## Check if all required dependencies are available
	@echo "Checking dependencies..."
	@which docker > /dev/null || (echo "Error: Docker is required but not installed" && exit 1)
	@which cargo > /dev/null || (echo "Error: Cargo is required but not installed" && exit 1)
	@docker ps > /dev/null || (echo "Error: Docker is not running" && exit 1)
	@echo "All dependencies are available ✓"

setup: check-deps ## Setup test environment
	@echo "Setting up test environment..."
	cargo build --tests
	@echo "Test environment ready ✓"

# Development targets
dev-test: ## Run tests in development mode with file watching
	@echo "Running tests in watch mode (requires cargo-watch)"
	@which cargo-watch > /dev/null || cargo install cargo-watch
	cargo watch -x "test --test integration_auth_oauth_flow"

# Specific test groups
test-start: ## Test OAuth start endpoints
	cargo test --test integration_auth_oauth_flow test_oauth_start

test-callback: ## Test OAuth callback endpoints  
	cargo test --test integration_auth_oauth_flow test_oauth_callback

test-state: ## Test OAuth state management
	cargo test --test integration_auth_oauth_flow test_oauth_state

test-security: ## Test security features
	cargo test --test integration_auth_oauth_flow security

# Performance testing
test-performance: ## Run performance-related tests
	cargo test --test integration_auth_oauth_flow performance

# Documentation
doc-tests: ## Generate and view test documentation
	cargo doc --document-private-items --open

# Environment-specific targets
test-github-actions: ## Run tests optimized for GitHub Actions
	GITHUB_ACTIONS=true CI=true TEST_VERBOSE=true TEST_MAX_CONCURRENCY=2 cargo test --test integration_auth_oauth_flow

test-gitlab-ci: ## Run tests optimized for GitLab CI
	GITLAB_CI=true CI=true TEST_VERBOSE=true TEST_MAX_CONCURRENCY=2 cargo test --test integration_auth_oauth_flow

# Debugging targets
debug-test: ## Run tests with maximum debugging output
	RUST_LOG=trace TEST_VERBOSE=true cargo test --test integration_auth_oauth_flow -- --nocapture

# Utility targets
list-tests: ## List all available integration tests
	cargo test --test integration_auth_oauth_flow -- --list

count-tests: ## Count number of integration tests
	@cargo test --test integration_auth_oauth_flow -- --list | grep -c "test result:"

# Container management
start-postgres: ## Start a local PostgreSQL container for testing
	docker run -d --name iam-test-postgres \
		-e POSTGRES_PASSWORD=test \
		-e POSTGRES_USER=test \
		-e POSTGRES_DB=iam_test \
		-p 5432:5432 \
		postgres:15-alpine

stop-postgres: ## Stop the local PostgreSQL container
	docker stop iam-test-postgres || true
	docker rm iam-test-postgres || true

# Example usage commands
examples: ## Show example usage commands
	@echo "Example Usage Commands:"
	@echo "======================"
	@echo ""
	@echo "Basic testing:"
	@echo "  make test-integration          # Run all integration tests"
	@echo "  make test-verbose              # Run with debug output"
	@echo "  make test-single TEST=test_oauth_start_github_redirects_properly"
	@echo ""
	@echo "Development:"
	@echo "  make dev-test                  # Watch mode (requires cargo-watch)"
	@echo "  make test-quick                # Fast tests for local dev"
	@echo ""
	@echo "CI/CD:"
	@echo "  make test-ci                   # CI optimized"
	@echo "  make test-github-actions       # GitHub Actions specific"
	@echo "  make test-gitlab-ci            # GitLab CI specific"
	@echo ""
	@echo "Debugging:"
	@echo "  make debug-test                # Maximum verbosity"
	@echo "  make list-tests                # Show all test names"
	@echo ""
	@echo "Performance:"
	@echo "  make test-stress               # High concurrency test"
	@echo "  make test-performance          # Performance tests only" 