# IAM Service

A modular Identity and Access Management (IAM) microservice implemented in Rust using hexagonal architecture.

## Architecture

The IAM service is built using a hexagonal/ports and adapters architecture with four layers:

1. **Domain Layer**: Core business logic and entities
2. **Application Layer**: Use cases and business flows
3. **Infrastructure Layer**: Implementation of domain interfaces
4. **HTTP Layer**: REST API endpoints and request/response handling

## Features

- OAuth2 authentication with multiple providers (multi-provider, provider-agnostic user model):
  - GitHub
  - GitLab
- **Provider Linking**: Authenticated users can link additional OAuth2 providers to their existing account
- JWT token generation and validation
- User management with support for multiple linked OAuth providers
- **Email addresses are managed in a separate table**; users can have multiple emails, but only one is primary
- **Automatic email discovery**: When linking providers, new emails are automatically added as secondary emails
- User profile endpoints always return the primary email (if available)
- Modular and extensible architecture
- Read/write repository pattern for database scalability
- Database connection pool with read replica support
- Flexible configuration system
- SeaORM-based database migrations and entity generation

## Configuration

The IAM service supports a flexible configuration system with multiple sources and automatic loading:

### Configuration Sources (in order of precedence)

1. **Environment Variables**: Using the `APP_` prefix
   ```bash
   APP_SERVER_HOST=127.0.0.1
   APP_SERVER_PORT=8080
   APP_DATABASE_URL=postgres://postgres:postgres@localhost:5432/iam
   APP_JWT_SECRET=your-super-secret-jwt-key-at-least-32-characters-long
   APP_OAUTH_GITHUB_CLIENT_ID=your-github-client-id
   APP_OAUTH_GITHUB_CLIENT_SECRET=your-github-client-secret
   ```

2. **Environment File**: Create a `.env` file in the project root (automatically loaded)
   ```env
   # Database Configuration
   DATABASE_URL=postgres://postgres:postgres@localhost:5432/iam
   
   # Application Configuration
   APP_SERVER_HOST=127.0.0.1
   APP_SERVER_PORT=8080
   
   # JWT Configuration
   APP_JWT_SECRET=your-super-secret-jwt-key-at-least-32-characters-long
   APP_JWT_EXPIRATION_SECONDS=3600
   
   # OAuth Configuration
   APP_OAUTH_GITHUB_CLIENT_ID=your-github-client-id
   APP_OAUTH_GITHUB_CLIENT_SECRET=your-github-client-secret
   APP_OAUTH_GITHUB_REDIRECT_URI=http://localhost:8080/auth/github/callback
   
   APP_OAUTH_GITLAB_CLIENT_ID=your-gitlab-client-id
   APP_OAUTH_GITLAB_CLIENT_SECRET=your-gitlab-client-secret
   APP_OAUTH_GITLAB_REDIRECT_URI=http://localhost:8080/auth/gitlab/callback
   
   # Logging
   RUST_LOG=info,iam_service=debug
   ```

3. **TOML Configuration File**: Create a `config.toml` file or specify custom file with `CONFIG_FILE` environment variable
   ```toml
   [server]
   host = "127.0.0.1"
   port = 8080
   
   [database]
   url = "postgres://postgres:postgres@localhost:5432/iam"
   
   [oauth.github]
   client_id = "your-github-client-id"
   client_secret = "your-github-client-secret"
   redirect_uri = "http://localhost:8080/auth/github/callback"
   
   [oauth.gitlab]
   client_id = "your-gitlab-client-id"
   client_secret = "your-gitlab-client-secret"
   redirect_uri = "http://localhost:8080/auth/gitlab/callback"
   
   [jwt]
   secret = "your-super-secret-jwt-key-at-least-32-characters-long"
   expiration_seconds = 3600
   ```

### Configuration Setup

1. Copy `.env.example` to `.env` (if available) or create a new `.env` file
2. Update the values in `.env` with your specific configuration
3. The `.env` file is automatically loaded at application startup
4. Override specific values using environment variables if needed

## Database Management

The service uses SeaORM for database operations with automatic migrations and entity generation.

### Database Setup

1. **Start PostgreSQL** (using Docker Compose):
   ```bash
   docker-compose up postgres -d
   ```

2. **Create Database** (if not using Docker Compose):
   ```bash
   createdb iam
   ```

### Database Migrations

The service includes a comprehensive migration system for schema management:

#### Running Migrations

```bash
# Run all pending migrations
cd migration
cargo run -- up

# Or run with specific database URL
cargo run -- up -u postgres://postgres:postgres@localhost:5432/iam

# Check migration status
cargo run -- status

# Rollback last migration
cargo run -- down
```

#### Creating New Migrations

```bash
# Install SeaORM CLI (if not already installed)
cargo install sea-orm-cli

# Create a new migration
sea-orm-cli migrate generate create_new_table

# The migration file will be created in migration/src/
# Edit the generated file to define your schema changes
```

#### Database Schema

The current schema includes:

- **users**: User profiles (provider-agnostic)
  - `id` (UUID, primary key)
  - `username`
  - `avatar_url`
  - `created_at`, `updated_at`

- **user_emails**: Email addresses for users
  - `id` (UUID, primary key)
  - `user_id` (UUID, foreign key to users)
  - `email` (String, unique)
  - `is_primary` (bool)
  - `is_verified` (bool)
  - `created_at`, `updated_at`

- **provider_tokens**: OAuth tokens for external API access
  - `id` (auto-increment primary key)
  - `user_id` (foreign key to users)
  - `provider` (github, gitlab, etc.)
  - `provider_user_id` (String, unique per provider)
  - `access_token`, `refresh_token`, `expires_in`
  - `created_at`, `updated_at`

- **refresh_tokens**: JWT refresh tokens
  - `id` (UUID, primary key)
  - `user_id` (foreign key to users)
  - `token`, `is_valid`, `expires_at`
  - `created_at`

### Entity Generation

Entities are automatically generated from the database schema:

```bash
# Generate entities from current database schema
sea-orm-cli generate entity \
  --database-url postgres://postgres:postgres@localhost:5432/iam \
  --output-dir infra/src/repository/entity

# This will create/update:
# - infra/src/repository/entity/users.rs
# - infra/src/repository/entity/provider_tokens.rs
# - infra/src/repository/entity/refresh_tokens.rs
# - infra/src/repository/entity/mod.rs
# - infra/src/repository/entity/prelude.rs
```

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

```bash
APP_DATABASE_URL=postgres://postgres:postgres@primary:5432/iam
APP_DATABASE_READ_REPLICAS=['postgres://postgres:postgres@replica1:5432/iam', 'postgres://postgres:postgres@replica2:5432/iam']
```

## User Model and OAuth Flow

- Users are provider-agnostic and can link multiple OAuth2 providers to a single account.
- Email addresses are managed in a separate entity (`user_emails`).
- The `email` field in user responses always returns the user's primary email (if available).
- Users can have multiple emails, but only one is primary.
- The system supports multi-provider login and account linking.

### OAuth Flow Behavior

The OAuth endpoints support two different operations based on the request context:

1. **Login Operation** (unauthenticated users):
   - Access `/auth/{provider}/start` without Authorization header
   - Creates new user or authenticates existing user
   - Returns JWT tokens and user profile

2. **Provider Linking Operation** (authenticated users):
   - Access `/auth/{provider}/start` with Authorization header (`Bearer <jwt-token>`)
   - Links the OAuth provider to the existing authenticated user
   - Prevents linking providers already linked to other users
   - Automatically adds new emails as secondary (unverified) emails
   - Returns updated user profile with all emails

The operation type is encoded in the OAuth state parameter, ensuring secure and stateless operation context preservation during provider redirects.

## Getting Started

### Prerequisites

- Rust 1.70+
- PostgreSQL (for production) or Docker (for development/testing)
- OAuth2 provider credentials (GitHub, GitLab, etc.)

### Quick Start

1. **Configuration**: Set up your environment variables or configuration file (see [Configuration](#configuration) section)

2. **Database Setup**: 
   ```bash
   # Using Docker (recommended for development)
   docker-compose up postgres -d
   
   # Run migrations
   cd migration && cargo run -- up
   ```

3. **Start the service**:
   ```bash
   cargo run
   ```

4. **Test OAuth integration**:
   ```bash
   # Modern way (recommended)
   just test-integration          # Using just
   cargo make test-integration    # Using cargo-make
   
   # Traditional way  
   cargo test --test integration_auth_oauth_flow
   ```

### 🧪 OAuth Integration Tests

The service includes comprehensive integration tests for OAuth authentication flows with testcontainers for PostgreSQL. These tests validate:

- **OAuth Start Endpoints**: GitHub/GitLab redirects, provider validation, state management
- **OAuth Callbacks**: Login flow, provider linking, error handling  
- **Security Features**: State parameter integrity, tamper resistance, nonce validation
- **Performance**: Concurrent flows, database cleanup efficiency

#### Running Tests

**Modern Task Runner** (recommended):

```bash
# Install just (modern, clean syntax)
cargo install just

# Run OAuth integration tests
just test-integration
just test-single test_oauth_start_github_redirects_properly
just test-debug                # With full debugging output
just watch                     # Auto-run on file changes

# Test specific areas
just test-start                 # OAuth start endpoints
just test-callback              # OAuth callback handling
just test-state                 # State management

# Development workflow
just dev                        # Complete setup: dependencies + database + migrations
```

**Alternative: Cargo-Make** (Rust-native):
```bash
cargo install cargo-make
cargo make test-integration
```

**Direct Commands**:
```bash
# Run all OAuth integration tests
cargo test --test integration_auth_oauth_flow

# Run specific test with debugging
RUST_LOG=debug cargo test test_oauth_start_github_redirects_properly -- --nocapture --exact

# Run with CI optimizations
CI=true TEST_MAX_CONCURRENCY=2 cargo test --test integration_auth_oauth_flow
```

For detailed test documentation, see [`tests/README.md`](tests/README.md) and the comprehensive [**Testing Guide**](docs/TESTING_GUIDE.md).

## API Documentation

The API follows OpenAPI specification. See `openspecs.yaml` (v1.3.0) for complete API documentation.

**Latest Updates (v1.3.0):**
- Added provider linking functionality for authenticated users
- Enhanced OAuth endpoints to support dual-purpose operation (login vs. linking)
- Added comprehensive error handling for provider conflicts
- Added new response schemas for link operations with email management

### Main Endpoints

- `GET /auth/{provider}/start` - Start OAuth2 authentication flow (supports both login and provider linking)
- `GET /auth/{provider}/callback` - OAuth2 callback endpoint (returns different response based on operation)
- `POST /token/refresh` - Refresh JWT token
- `GET /me` - Get current authenticated user profile (primary email)
- `GET /.well-known/jwks.json` - Public keys for JWT validation
- `POST /internal/{provider}/token` - Get provider access token (internal)

#### OAuth Authentication Endpoints

**Login Flow** (for new/existing users):
```http
GET /auth/{provider}/start
# No Authorization header required
# Redirects to provider OAuth page
# Returns JWT tokens and user profile on callback
```

**Provider Linking Flow** (for authenticated users):
```http
GET /auth/{provider}/start
Authorization: Bearer <jwt-token>
# Requires valid JWT token
# Links provider to existing user account
# Returns updated user profile with all emails on callback
```

### Example Usage

```bash
# Start GitHub OAuth login flow (for new users)
curl "http://localhost:8080/auth/github/start"

# Link GitHub to existing account (for authenticated users)
curl -H "Authorization: Bearer <jwt-token>" \
     "http://localhost:8080/auth/github/start"

# Get user profile (requires valid JWT)
curl -H "Authorization: Bearer <jwt-token>" "http://localhost:8080/me"

# Refresh token
curl -X POST "http://localhost:8080/token/refresh" \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "<refresh-token>"}'
```

#### Response Examples

**Login Response** (after successful OAuth callback):
```json
{
  "operation": "login",
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar_url": "https://avatars.githubusercontent.com/u/123456"
  },
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 3600,
  "refresh_token": "refresh_token_here"
}
```

**Link Provider Response** (after successful provider linking):
```json
{
  "operation": "link",
  "message": "GitHub successfully linked",
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar_url": "https://avatars.githubusercontent.com/u/123456"
  },
  "emails": [
    {
      "id": "email-uuid-1",
      "email": "john@example.com",
      "is_primary": true,
      "is_verified": true
    },
    {
      "id": "email-uuid-2", 
      "email": "john.github@example.com",
      "is_primary": false,
      "is_verified": false
    }
  ],
  "new_email_added": true,
  "new_email": "john.github@example.com"
}
```

#### Error Responses

The provider linking operation includes specific error handling:

```json
{
  "operation": "link",
  "error": "provider_already_linked",
  "message": "This GitHub account is already linked to another user"
}
```

Common error scenarios:
- `provider_already_linked_to_same_user`: Provider is already linked to the same user
- `provider_already_linked`: Provider account is linked to a different user
- `user_not_found`: Authenticated user no longer exists
- `auth_error`: Failed to authenticate with the OAuth provider

## Development

### Project Structure

```
├── domain/           # Core business logic and entities
├── application/      # Use cases and business flows  
├── infra/           # Infrastructure implementations
├── http/            # HTTP layer (Axum web server)
├── migration/       # Database migrations
├── config/          # Configuration files
└── target/          # Build output
```

### Adding New Features

1. **Add domain entities** in `domain/src/entity/`
2. **Define use cases** in `application/src/usecase/`
3. **Implement repositories** in `infra/src/repository/`
4. **Add HTTP handlers** in `http/src/handlers/`
5. **Create migrations** for schema changes
6. **Regenerate entities** after schema changes

### Key Dependencies

- **base64**: For OAuth state parameter encoding/decoding
- **serde**: JSON serialization for OAuth state management
- **thiserror**: Structured error handling for use cases
- **async-trait**: Async trait support for use case interfaces
- **uuid**: User ID handling and state nonce generation

### Database Workflow

1. **Schema changes**: Create migration files
2. **Run migrations**: `cd migration && cargo run -- up`
3. **Regenerate entities**: `sea-orm-cli generate entity`
4. **Update repository code** if needed
5. **Test changes**: `cargo test` 