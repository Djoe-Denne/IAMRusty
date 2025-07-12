# AIForAll Microservices

This repository contains multiple microservices for the AIForAll platform:

- **IAMRusty**: Identity and Access Management service
- **Telegraph**: Communication service for emails, notifications, and SMS
- **rustycog**: Shared Rust crates for common functionality
- **iam-events**: Event definitions shared between services

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Git

### Running All Services

The global docker-compose setup runs both IAMRusty and Telegraph services with shared infrastructure:

```bash
# Clone the repository
git clone <repository-url>
cd AIForAll

# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop all services
docker-compose down
```

### Services Overview

| Service | Port | Description |
|---------|------|-------------|
| IAMRusty | 8080 (HTTP), 8443 (HTTPS) | Identity and Access Management |
| Telegraph | 8081 | Communication service |
| PostgreSQL | 5432 | Database for IAMRusty |
| LocalStack | 4566 | SQS message queue service |

### Service Communication

- IAMRusty publishes `user_signed_up` events to the `user-events` SQS queue
- Telegraph consumes these events and sends welcome emails
- Both services share the same LocalStack SQS instance

### Development Configuration

Both services use development profiles when running in Docker:
- `IAMRusty/config/development.toml`
- `Telegraph/config/development.toml`

### Database Management

```bash
# Truncate all database tables
docker-compose --profile tools run --rm truncate-db

# Verify all emails (for testing)
docker-compose --profile tools run --rm verify-emails
```

### Individual Service Development

Each service can also be run individually:

```bash
# IAMRusty only
cd IAMRusty
docker-compose up

# Telegraph (requires external SQS)
cd Telegraph
cargo run
```

### Testing

Integration tests can be run against the running services:

```bash
# Run IAMRusty tests
cd IAMRusty
cargo test

# Run Telegraph tests  
cd Telegraph
cargo test
```

### Configuration

- **IAMRusty**: See `IAMRusty/README.md` for detailed configuration
- **Telegraph**: See `Telegraph/config/` for communication service settings
- **SQS**: Both services connect to LocalStack SQS on `localstack:4566`

### Monitoring

- Health checks are configured for all services
- LocalStack dashboard: http://localhost:4566/_localstack/health
- IAMRusty API: http://localhost:8080/health (when implemented)
- Telegraph API: http://localhost:8081/health (when implemented) 