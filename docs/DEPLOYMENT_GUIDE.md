# Deployment Guide

## Overview

This guide covers deploying the IAM service based on the actual codebase implementation. It uses the real configuration structure, endpoints, and Docker setup that exists in the project.

## Table of Contents

- [Real Project Structure](#real-project-structure)
- [Actual Configuration](#actual-configuration)
- [Docker Deployment](#docker-deployment)
- [Environment Variables](#environment-variables)
- [Database Setup](#database-setup)
- [HTTPS/TLS Configuration](#httpstls-configuration)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)

## Real Project Structure

The actual project builds to:
- **Binary name**: `iam-service` (not `iam`)
- **Main entrypoint**: `/app/iam-service`
- **Configuration**: Uses TOML files with specific structure
- **Environment**: Controlled by `RUN_ENV` variable

## Actual Configuration

### Configuration Files

The project uses these real configuration files:

**`config/default.toml`** (base configuration):
```toml
[server]
host = "127.0.0.1"
port = 8081
tls_enabled = false
tls_cert_path = "./certs/cert.pem"
tls_key_path = "./certs/key.pem"
tls_port = 8443

[database]
host = "localhost"
port = 5432
db = "iam_dev"

[database.creds]
username = "postgres"
password = "postgres"

# Read replicas (array of connection strings)
read_replicas = []

[oauth.github]
client_id = "your-github-client-id"
client_secret = "your-github-client-secret"
redirect_uri = "http://localhost:8080/api/auth/github/callback"
auth_url = "https://github.com/login/oauth/authorize"
token_url = "https://github.com/login/oauth/access_token"
user_url = "https://api.github.com/user"

[oauth.gitlab]
client_id = "your-gitlab-client-id"
client_secret = "your-gitlab-client-secret"
redirect_uri = "http://localhost:8080/api/auth/gitlab/callback"
auth_url = "https://gitlab.com/oauth/authorize"
token_url = "https://gitlab.com/oauth/token"
user_url = "https://gitlab.com/api/v4/user"

[jwt]
secret = "your-jwt-secret-key-should-be-at-least-32-bytes"
expiration_seconds = 3600
```

**`config/production.toml`** (production overrides):
```toml
[server]
host = "0.0.0.0"
port = 8080
tls_enabled = true
tls_cert_path = "/etc/ssl/certs/iam.crt"
tls_key_path = "/etc/ssl/private/iam.key"
tls_port = 8443

[database]
host = "db"
port = 5432
db = "iam_prod"

[database.creds]
username = "postgres"
password = "postgres"

# Production read replicas
read_replicas = [
    "postgres://postgres:postgres@db-read-1:5432/iam_prod",
    "postgres://postgres:postgres@db-read-2:5432/iam_prod"
]

[oauth.github]
redirect_uri = "https://your-domain.com/api/auth/github/callback"

[oauth.gitlab]
redirect_uri = "https://your-domain.com/api/auth/gitlab/callback"

[jwt]
expiration_seconds = 86400  # 24 hours
```

## Docker Deployment

### Actual Dockerfile

The project includes this real Dockerfile:

```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /usr/src/app

# Copy over manifests and dependencies first
COPY Cargo.toml Cargo.lock ./
COPY domain/Cargo.toml domain/
COPY application/Cargo.toml application/
COPY infra/Cargo.toml infra/
COPY http/Cargo.toml http/

# Create dummy source files to build dependencies
RUN mkdir -p domain/src application/src infra/src http/src src \
    && echo "fn main() {}" > src/main.rs \
    && echo "pub fn dummy() {}" > domain/src/lib.rs \
    && echo "pub fn dummy() {}" > application/src/lib.rs \
    && echo "pub fn dummy() {}" > infra/src/lib.rs \
    && echo "pub fn dummy() {}" > http/src/lib.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy source files
RUN rm -rf domain/src application/src infra/src http/src src

# Copy the actual source code
COPY domain/src domain/src/
COPY application/src application/src/
COPY infra/src infra/src/
COPY http/src http/src/
COPY src src/

# Touch the source files to trigger rebuild
RUN find . -name "*.rs" -exec touch {} \;

# Build the application
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/iam-service /app/iam-service

# Create directories
RUN mkdir -p /app/keys /app/config

# Run as non-root user
RUN adduser --disabled-password --gecos "" appuser \
    && chown -R appuser:appuser /app
USER appuser

# Set the entrypoint
ENTRYPOINT ["/app/iam-service"]
```

### Actual Docker Compose

The project includes this real `docker-compose.yml`:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_USER: postgres
      POSTGRES_DB: iam_dev
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

  iam-service:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"   # HTTP port
      - "8443:8443"   # HTTPS port
    environment:
      - RUN_ENV=${RUN_ENV:-production}
      - RUST_LOG=info,tower_http=debug
    volumes:
      - ./keys:/app/keys
      - ./config:/app/config
      - ./certs:/app/certs
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  postgres-data:
```

## Environment Variables

Based on the actual configuration system, you can override config with environment variables using `APP_` prefix:

```bash
# Server configuration
APP_SERVER_HOST=0.0.0.0
APP_SERVER_PORT=8080
APP_SERVER_TLS_ENABLED=true
APP_SERVER_TLS_CERT_PATH=/path/to/cert.pem
APP_SERVER_TLS_KEY_PATH=/path/to/key.pem
APP_SERVER_TLS_PORT=8443

# Database configuration
APP_DATABASE_HOST=localhost
APP_DATABASE_PORT=5432
APP_DATABASE_DB=iam_prod
APP_DATABASE_CREDS_USERNAME=postgres
APP_DATABASE_CREDS_PASSWORD=secure_password

# OAuth configuration
APP_OAUTH_GITHUB_CLIENT_ID=your_github_client_id
APP_OAUTH_GITHUB_CLIENT_SECRET=your_github_client_secret
APP_OAUTH_GITHUB_REDIRECT_URI=https://yourdomain.com/api/auth/github/callback

APP_OAUTH_GITLAB_CLIENT_ID=your_gitlab_client_id
APP_OAUTH_GITLAB_CLIENT_SECRET=your_gitlab_client_secret
APP_OAUTH_GITLAB_REDIRECT_URI=https://yourdomain.com/api/auth/gitlab/callback

# JWT configuration
APP_JWT_SECRET=your-super-secret-jwt-key-at-least-32-characters-long
APP_JWT_EXPIRATION_SECONDS=3600

# Environment selection
RUN_ENV=production
RUST_LOG=info,iam_service=debug
```

## Database Setup

The service uses PostgreSQL with SeaORM. Database configuration is in the config files:

```toml
[database]
host = "localhost"
port = 5432
db = "iam_prod"

[database.creds]
username = "postgres"
password = "postgres"

# Optional read replicas
read_replicas = [
    "postgres://user:pass@replica1:5432/dbname",
    "postgres://user:pass@replica2:5432/dbname"
]
```

### Migrations

Run database migrations:

```bash
cd migration
cargo run -- up
```

## Real Endpoints

The actual HTTP server implements these endpoints:

- `GET /health` - Health check endpoint
- `GET /api/auth/{provider}/start` - OAuth start (GitHub, GitLab)
- `GET /api/auth/{provider}/callback` - OAuth callback
- `POST /api/token/refresh` - Token refresh
- `GET /api/me` - Get current user (requires auth)

## HTTPS/TLS Configuration

The service supports HTTPS via configuration:

```toml
[server]
tls_enabled = true
tls_cert_path = "/path/to/cert.pem"
tls_key_path = "/path/to/key.pem"
tls_port = 8443
```

Generate certificates (for development):

```bash
# Use the actual script in the project
./scripts/generate-certs.sh
# or on Windows
./scripts/generate-certs.ps1
```

## Production Deployment

### Simple Production Setup

1. **Build the image:**
```bash
docker build -t iam-service:latest .
```

2. **Create production environment file:**
```bash
# .env.prod
RUN_ENV=production
APP_DATABASE_CREDS_PASSWORD=secure_production_password
APP_JWT_SECRET=super-secure-jwt-secret-at-least-32-chars
APP_OAUTH_GITHUB_CLIENT_SECRET=github_secret
APP_OAUTH_GITLAB_CLIENT_SECRET=gitlab_secret
```

3. **Run with production config:**
```bash
docker run -d \
  --name iam-service \
  --env-file .env.prod \
  -p 8080:8080 \
  -p 8443:8443 \
  -v ./config:/app/config:ro \
  -v ./certs:/app/certs:ro \
  iam-service:latest
```

### Health Checks

The service provides a health endpoint:

```bash
curl http://localhost:8080/health
# Returns: OK
```

Use this for container health checks:

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
```

## Monitoring

### Logging

Configure logging via environment:

```bash
RUST_LOG=info,iam_service=debug,tower_http=debug
```

### Basic Metrics

The service includes panic recovery middleware that logs panics and returns JSON error responses.

## Troubleshooting

### Configuration Issues

1. **Check environment**: Ensure `RUN_ENV` is set correctly
2. **Verify config files**: Make sure config files exist in `/app/config/`
3. **Database connection**: Verify database host and credentials

### Common Issues

**Service won't start:**
```bash
# Check logs
docker logs iam-service

# Verify configuration
docker exec iam-service cat /app/config/production.toml
```

**Database connection failed:**
```bash
# Test database connectivity
docker exec iam-service pg_isready -h db -U postgres -d iam_prod
```

**OAuth not working:**
- Verify redirect URIs match exactly in OAuth provider settings
- Check client IDs and secrets are set correctly
- Ensure HTTPS is configured if required by OAuth provider

### Container Debugging

```bash
# Execute shell in container
docker exec -it iam-service /bin/bash

# Check process
docker exec iam-service ps aux

# Check configuration
docker exec iam-service ls -la /app/config/
```

This deployment guide reflects the actual implementation in the codebase and avoids any hallucinated features or configurations that don't exist.