#!/bin/bash

# Test runner script with automatic container cleanup
# Usage: ./scripts/test-with-cleanup.sh [test-name]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to cleanup containers
cleanup_containers() {
    print_status "🧹 Cleaning up test containers..."
    
    # Get all postgres:15-alpine container IDs
    CONTAINER_IDS=$(docker ps -q --filter "ancestor=postgres:15-alpine" 2>/dev/null || true)
    
    if [ -n "$CONTAINER_IDS" ]; then
        print_status "Stopping containers: $CONTAINER_IDS"
        echo "$CONTAINER_IDS" | xargs docker stop 2>/dev/null || true
        echo "$CONTAINER_IDS" | xargs docker rm 2>/dev/null || true
        print_success "✅ Containers cleaned up"
    else
        print_status "No test containers found to clean up"
    fi
    
    # Also clean up any stopped containers
    STOPPED_CONTAINERS=$(docker ps -aq --filter "ancestor=postgres:15-alpine" 2>/dev/null || true)
    if [ -n "$STOPPED_CONTAINERS" ]; then
        print_status "Removing stopped containers..."
        echo "$STOPPED_CONTAINERS" | xargs docker rm 2>/dev/null || true
    fi
}

# Function to run tests
run_tests() {
    local test_name="$1"
    
    if [ -n "$test_name" ]; then
        print_status "🧪 Running specific test: $test_name"
        cargo test --test "$test_name" -- --nocapture
    else
        print_status "🧪 Running all integration tests..."
        
        # Run database tests
        print_status "Running database integration tests..."
        cargo test --test integration_database_test -- --nocapture
        
        # Run fixture tests
        print_status "Running fixture tests..."
        cargo test --test example_fixture_usage -- --nocapture
        cargo test --test integration_with_fixtures_example -- --nocapture
    fi
}

# Trap to ensure cleanup runs even if tests fail
trap cleanup_containers EXIT

# Main execution
main() {
    print_status "🚀 Starting test run with automatic cleanup"
    
    # Check if Docker is running
    if ! docker info >/dev/null 2>&1; then
        print_error "Docker is not running. Please start Docker and try again."
        exit 1
    fi
    
    # Clean up any existing containers first
    cleanup_containers
    
    # Run the tests
    if run_tests "$1"; then
        print_success "🎉 All tests passed!"
    else
        print_error "❌ Some tests failed"
        exit 1
    fi
}

# Run main function with all arguments
main "$@" 