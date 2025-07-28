#!/bin/bash

# Script to rename the service blueprint to create a new service
# Usage: ./scripts/rename-service.sh <service-name> [service-description]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

# Check if service name is provided
if [ $# -lt 1 ]; then
    print_error "Usage: $0 <service-name> [service-description]"
    print_error "Example: $0 user-management 'User management microservice'"
    exit 1
fi

SERVICE_NAME="$1"
SERVICE_DESCRIPTION="${2:-$SERVICE_NAME microservice}"

# Validate service name format (should be kebab-case)
if [[ ! $SERVICE_NAME =~ ^[a-z][a-z0-9-]*[a-z0-9]$ ]]; then
    print_error "Service name must be in kebab-case format (lowercase, hyphens allowed)"
    print_error "Example: user-management, order-service, notification-system"
    exit 1
fi

# Convert service name to different formats
SERVICE_NAME_KEBAB="$SERVICE_NAME"
SERVICE_NAME_SNAKE=$(echo "$SERVICE_NAME" | tr '-' '_')
SERVICE_NAME_PASCAL=$(echo "$SERVICE_NAME" | sed 's/-/ /g' | sed 's/\b\w/\u&/g' | sed 's/ //g')
SERVICE_NAME_UPPER=$(echo "$SERVICE_NAME_SNAKE" | tr '[:lower:]' '[:upper:]')

print_header "Service Blueprint Rename Tool"
print_status "Service Name (kebab-case): $SERVICE_NAME_KEBAB"
print_status "Service Name (snake_case): $SERVICE_NAME_SNAKE"
print_status "Service Name (PascalCase): $SERVICE_NAME_PASCAL"
print_status "Service Description: $SERVICE_DESCRIPTION"
echo

# Check if we're in the right directory
if [ ! -f "README.md" ] || [ ! -d "domain" ]; then
    print_error "Please run this script from the service blueprint root directory"
    exit 1
fi

# Function to replace placeholders in a file
replace_placeholders() {
    local file="$1"
    if [ -f "$file" ]; then
        # Create a backup
        cp "$file" "$file.bak"
        
        # Replace placeholders
        sed -i.tmp \
            -e "s/{{SERVICE_NAME}}/$SERVICE_NAME_SNAKE/g" \
            -e "s/{{SERVICE_NAME_KEBAB}}/$SERVICE_NAME_KEBAB/g" \
            -e "s/{{SERVICE_NAME_PASCAL}}/$SERVICE_NAME_PASCAL/g" \
            -e "s/{{SERVICE_NAME_UPPER}}/$SERVICE_NAME_UPPER/g" \
            -e "s/{{SERVICE_DESCRIPTION}}/$SERVICE_DESCRIPTION/g" \
            "$file"
        
        # Remove temporary file
        rm -f "$file.tmp"
        
        print_status "Updated: $file"
    fi
}

# Function to rename files and directories
rename_files() {
    print_header "Renaming Files and Directories"
    
    # Find and rename files containing placeholder patterns
    find . -type f -name "*example*" | while read -r file; do
        new_file=$(echo "$file" | sed "s/example/${SERVICE_NAME_SNAKE}/g")
        if [ "$file" != "$new_file" ]; then
            mv "$file" "$new_file"
            print_status "Renamed: $file -> $new_file"
        fi
    done
}

# Function to update all source files
update_source_files() {
    print_header "Updating Source Files"
    
    # List of files to update
    files_to_update=(
        "Cargo.toml"
        "src/main.rs"
        "domain/Cargo.toml"
        "domain/src/lib.rs"
        "domain/src/entity/mod.rs"
        "domain/src/port/mod.rs"
        "domain/src/service/mod.rs"
        "application/Cargo.toml"
        "application/src/lib.rs"
        "application/src/dto/mod.rs"
        "application/src/command/mod.rs"
        "application/src/usecase/mod.rs"
    )
    
    # Update each file
    for file in "${files_to_update[@]}"; do
        if [ -f "$file" ]; then
            replace_placeholders "$file"
        fi
    done
    
    # Update all Rust source files
    find . -name "*.rs" -type f | while read -r file; do
        replace_placeholders "$file"
    done
    
    # Update TOML files
    find . -name "*.toml" -type f | while read -r file; do
        replace_placeholders "$file"
    done
}

# Function to create configuration files
create_config_files() {
    print_header "Creating Configuration Files"
    
    # Create configuration directory if it doesn't exist
    mkdir -p config
    
    # Create default configuration
    cat > config/default.toml << EOF
[server]
host = "localhost"
port = 8080
tls_enabled = false
tls_cert_path = ""
tls_key_path = ""
tls_port = 8443

[database]
host = "localhost"
port = 5432
name = "${SERVICE_NAME_SNAKE}_dev"
username = "postgres"
password = "postgres"

[logging]
level = "info"

[queue]
type = "disabled"  # Options: disabled, sqs, kafka

# SQS configuration (if using SQS)
[queue.sqs]
region = "us-east-1"
endpoint = "http://localhost:4566"  # LocalStack endpoint
access_key = "test"
secret_key = "test"

# Kafka configuration (if using Kafka)
[queue.kafka]
brokers = ["localhost:9092"]
group_id = "${SERVICE_NAME_KEBAB}-consumer"
EOF

    # Create development configuration
    cat > config/development.toml << EOF
[server]
port = 8080

[database]
name = "${SERVICE_NAME_SNAKE}_dev"

[logging]
level = "debug"
EOF

    # Create test configuration
    cat > config/test.toml << EOF
[server]
port = 0  # Random port for tests

[database]
name = "${SERVICE_NAME_SNAKE}_test"

[logging]
level = "warn"
EOF

    print_status "Created configuration files in config/"
}

# Function to create Docker files
create_docker_files() {
    print_header "Creating Docker Files"
    
    # Create Dockerfile
    cat > Dockerfile << EOF
# Get build image argument
ARG BUILD_IMAGE=local/build-artifacts:latest

# Copy artifacts from build image
FROM \${BUILD_IMAGE} as build-source

# Final stage - minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \\
    ca-certificates \\
    libssl-dev \\
    curl \\
    && rm -rf /var/lib/apt/lists/*

# Create app user and directory
RUN adduser --disabled-password --gecos "" appuser
WORKDIR /app

# Copy the binary from builder stage
COPY --from=build-source /app/target/release/${SERVICE_NAME_KEBAB}-service /app/${SERVICE_NAME_KEBAB}-service
COPY --from=build-source /app/target/release/${SERVICE_NAME_SNAKE}migration /app/migration

# Copy entrypoint script
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Copy configuration files
COPY config /app/config

# Create resources directory if needed
RUN mkdir -p /app/resources

# Change ownership
RUN chown -R appuser:appuser /app
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \\
  CMD curl -f http://localhost:8080/health || exit 1

# Set entrypoint
ENTRYPOINT ["/app/docker-entrypoint.sh"]
EOF

    # Create docker-entrypoint.sh
    cat > docker-entrypoint.sh << 'EOF'
#!/bin/bash
set -e

# Run database migrations if migration binary exists
if [ -f "/app/migration" ]; then
    echo "Running database migrations..."
    /app/migration up
fi

# Start the service
echo "Starting service..."
exec "$@"
EOF

    chmod +x docker-entrypoint.sh

    # Create docker-compose.yml
    cat > docker-compose.yml << EOF
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_USER: postgres
      POSTGRES_DB: ${SERVICE_NAME_SNAKE}_dev
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

  ${SERVICE_NAME_KEBAB}-service:
    build:
      context: ..
      dockerfile: ${SERVICE_NAME_KEBAB}/Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_ENVIRONMENT=development
    volumes:
      - ./config:/app/config
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  postgres-data:
EOF

    print_status "Created Docker files"
}

# Function to create additional necessary files
create_additional_files() {
    print_header "Creating Additional Files"
    
    # Create .env file template
    cat > .env.example << EOF
# Environment variables for ${SERVICE_NAME}
RUST_LOG=info
RUST_ENVIRONMENT=development
DATABASE_URL=postgres://postgres:postgres@localhost:5432/${SERVICE_NAME_SNAKE}_dev
EOF

    # Create .gitignore
    cat > .gitignore << EOF
/target
Cargo.lock
.env
*.bak
.DS_Store
EOF

    print_status "Created additional files"
}

# Function to clean up backup files
cleanup() {
    print_header "Cleaning Up"
    
    # Remove backup files
    find . -name "*.bak" -type f -delete
    
    print_status "Removed backup files"
}

# Main execution
main() {
    print_header "Starting Service Rename Process"
    
    # Confirm with user
    echo -n "Are you sure you want to rename this blueprint to '$SERVICE_NAME'? (y/N): "
    read -r confirmation
    if [[ ! $confirmation =~ ^[Yy]$ ]]; then
        print_warning "Operation cancelled"
        exit 0
    fi
    
    # Execute rename steps
    rename_files
    update_source_files
    create_config_files
    create_docker_files
    create_additional_files
    cleanup
    
    print_header "Rename Complete!"
    print_status "Your new service '$SERVICE_NAME' has been created successfully!"
    echo
    print_status "Next steps:"
    print_status "1. Update the domain entities in domain/src/entity/"
    print_status "2. Define your business logic in domain/src/service/"
    print_status "3. Create your use cases in application/src/usecase/"
    print_status "4. Add HTTP handlers in http/src/handlers/"
    print_status "5. Implement repositories in infra/src/repository/"
    print_status "6. Create database migrations in migration/src/"
    print_status "7. Write tests in tests/"
    echo
    print_status "To build and run your service:"
    print_status "  cargo build"
    print_status "  docker-compose up -d postgres"
    print_status "  cargo run"
    echo
    print_status "Happy coding! 🚀"
}

# Run the main function
main 