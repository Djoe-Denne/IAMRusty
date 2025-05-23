# IAM Service

A modular Identity and Access Management (IAM) microservice implemented in Rust using hexagonal architecture.

## Architecture

The IAM service is built using a hexagonal/ports and adapters architecture with four layers:

1. **Domain Layer**: Core business logic and entities
2. **Application Layer**: Use cases and business flows
3. **Infrastructure Layer**: Implementation of domain interfaces
4. **HTTP Layer**: REST API endpoints and request/response handling

## Features

- OAuth2 authentication with multiple providers:
  - GitHub
  - GitLab
- JWT token generation and validation
- User management
- Modular and extensible architecture
- Read/write repository pattern for database scalability
- Database connection pool with read replica support
- Flexible configuration system

## Configuration

The IAM service supports a flexible configuration system with multiple sources:

1. **Environment Variables**: Using the `APP_` prefix
   ```
   APP_SERVER_PORT=8080
   APP_DATABASE_URL=postgres://postgres:postgres@localhost:5432/iam
   ```

2. **Environment File**: Create a `.env` file in the project root
   ```
   APP_JWT_SECRET=your-secret-key
   ```

3. **TOML Configuration File**: Create a `config.toml` file or specify custom file with `CONFIG_FILE` environment variable
   ```toml
   [server]
   host = "127.0.0.1"
   port = 8080
   ```

### Configuration Precedence

Configuration is loaded with the following precedence (highest to lowest):
1. Environment variables
2. `.env` file
3. Config file specified by CONFIG_FILE environment variable
4. Default config file (`config.toml`)

## Database Read/Write Split

The service supports database read/write separation for scalability:

- Writes go to a primary database
- Reads go to replicas (with round-robin load balancing)
- Automatic fallback to primary if replicas are unavailable

To configure read replicas, provide them in the configuration:

```toml
[database]
url = "postgres://postgres:postgres@primary:5432/iam"
read_replicas = [
    "postgres://postgres:postgres@replica1:5432/iam",
    "postgres://postgres:postgres@replica2:5432/iam"
]
```

Or with environment variables:

```
APP_DATABASE_URL=postgres://postgres:postgres@primary:5432/iam
APP_DATABASE_READ_REPLICAS=['postgres://postgres:postgres@replica1:5432/iam', 'postgres://postgres:postgres@replica2:5432/iam']
```

## Getting Started

### Prerequisites

- Rust (latest stable)
- PostgreSQL

### Building

```bash
cargo build --release
```

### Running

```bash
cargo run --release
```

### Configuration

Create a `config.toml` file or use environment variables to configure the service.

## API Documentation

The API follows OpenAPI specification. Endpoints include:

- `POST /api/auth/login/{provider}` - Start OAuth2 login flow
- `GET /api/auth/{provider}/callback` - OAuth2 callback endpoint
- `GET /api/me` - Get current user information
- `POST /api/logout` - Logout user

## Development

### Running Tests

```bash
cargo test
```

### Database Migrations

The service uses SeaORM migrations:

```bash
cargo run --bin migration -- up
``` 