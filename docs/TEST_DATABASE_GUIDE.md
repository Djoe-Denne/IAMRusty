# Test Database Guide

Comprehensive guide for using the test database system with testcontainers, single container management, and table truncation for unparallelizable tests.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Usage Patterns](#usage-patterns)
- [Configuration](#configuration)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The test database system provides:

- **Single Container**: One PostgreSQL container shared across all tests
- **Table Truncation**: Automatic cleanup between tests for isolation
- **Unparallelizable Tests**: Uses `serial_test` to prevent race conditions
- **Configuration Integration**: Seamless integration with the app's config system
- **Migration Management**: Automatic database migration execution
- **Automatic Cleanup**: Container cleanup on test completion with multiple safety mechanisms

### Key Features

1. **Performance**: Single container startup reduces test execution time
2. **Isolation**: Table truncation ensures clean state between tests
3. **Reliability**: Proper foreign key handling during cleanup
4. **Integration**: Works with existing configuration and migration systems
5. **Safety**: Multiple cleanup mechanisms prevent container leaks

## Architecture

### Components

```
tests/
├── common/
│   ├── mod.rs              # Module exports
│   └── database.rs         # Test database implementation
└── integration_*.rs        # Integration test files
```

### Class Hierarchy

```
TestFixture
├── TestDatabase
│   ├── DbConnectionPool    # Connection management
│   ├── DatabaseConnection  # Direct database access
│   └── AppConfig          # Test configuration
└── Container Management
    ├── TestDatabaseContainer
    └── Global Container State
```

## Quick Start

### Basic Test Setup

```rust
mod common;

use common::TestFixture;
use serial_test::serial;

#[tokio::test]
#[serial]  // Required for unparallelizable tests
async fn test_database_operation() {
    // Create test fixture - starts container, runs migrations, cleans tables
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    
    // Get database connection
    let db = fixture.db();
    
    // Your test logic here...
    
    // Cleanup happens automatically when fixture is dropped
}
```

### Using Configuration

```rust
#[tokio::test]
#[serial]
async fn test_with_configuration() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let config = fixture.config();
    
    // Use configuration in your application code
    assert!(config.database.url.contains("localhost"));
    assert_eq!(config.oauth.github.client_id, "test_github_client_id");
}
```

### Using Connection Pool

```rust
#[tokio::test]
#[serial]
async fn test_with_connection_pool() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let pool = fixture.pool();
    
    // Get write connection
    let write_conn = pool.get_write_connection();
    
    // Get read connection (same as write in test environment)
    let read_conn = pool.get_read_connection();
}
```

## Usage Patterns

### Raw SQL Operations

```rust
#[tokio::test]
#[serial]
async fn test_raw_sql() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = fixture.db();
    
    // Insert data
    let user_id = Uuid::new_v4();
    let insert_sql = format!(
        "INSERT INTO users (id, provider_user_id, username, created_at, updated_at) 
         VALUES ('{}', 'test_id', 'test_user', NOW(), NOW())",
        user_id
    );
    
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        insert_sql,
    ))
    .await
    .expect("Failed to insert user");
    
    // Query data
    let count_result = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM users".to_string(),
        ))
        .await
        .expect("Failed to count users");
    
    let count: i64 = count_result
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    assert_eq!(count, 1);
}
```

### Using SeaORM Entities

```rust
#[tokio::test]
#[serial]
async fn test_with_entities() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = fixture.db();
    
    // Example with SeaORM entities (when available)
    // let user = users::ActiveModel {
    //     id: Set(Uuid::new_v4()),
    //     provider_user_id: Set("test_id".to_string()),
    //     username: Set("test_user".to_string()),
    //     email: Set(Some("test@example.com".to_string())),
    //     ..Default::default()
    // };
    // 
    // let user = user.insert(&*db).await.expect("Failed to insert user");
    // assert_eq!(user.username, "test_user");
}
```

### Manual Cleanup

```rust
#[tokio::test]
#[serial]
async fn test_manual_cleanup() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    
    // Insert test data...
    
    // Manual cleanup if needed
    fixture.cleanup().await.expect("Failed to cleanup");
    
    // Verify cleanup...
}
```

### Testing Foreign Key Constraints

```rust
#[tokio::test]
#[serial]
async fn test_foreign_keys() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = fixture.db();
    
    // Insert parent record
    let user_id = Uuid::new_v4();
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO users (id, provider_user_id, username, created_at, updated_at) 
                 VALUES ('{}', 'test_id', 'test_user', NOW(), NOW())", user_id),
    ))
    .await
    .expect("Failed to insert user");
    
    // Insert child record
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO provider_tokens (user_id, provider, access_token, created_at, updated_at) 
                 VALUES ('{}', 'github', 'token', NOW(), NOW())", user_id),
    ))
    .await
    .expect("Failed to insert token");
    
    // Cleanup will handle foreign keys properly
}
```

## Configuration

### Test Configuration Values

The test database system uses these default configuration values:

```toml
[server]
host = "127.0.0.1"
port = 8080
tls_enabled = false

[database]
url = "postgres://postgres:postgres@localhost:<dynamic_port>/iam_test"
read_replicas = []

[oauth.github]
client_id = "test_github_client_id"
client_secret = "test_github_client_secret"
redirect_uri = "http://localhost:8080/api/auth/github/callback"
auth_url = "http://localhost:3000/login/oauth/authorize"
token_url = "http://localhost:3000/login/oauth/access_token"
user_url = "http://localhost:3000/user"

[oauth.gitlab]
client_id = "test_gitlab_client_id"
client_secret = "test_gitlab_client_secret"
redirect_uri = "http://localhost:8080/api/auth/gitlab/callback"
auth_url = "http://localhost:3000/oauth/authorize"
token_url = "http://localhost:3000/oauth/token"
user_url = "http://localhost:3000/api/v4/user"

[jwt]
secret = "test_jwt_secret_for_testing_only_must_be_at_least_32_bytes_long"
expiration_seconds = 3600

[logging]
level = "debug"
```

### Environment Variables

You can override configuration using environment variables:

```bash
# Database timeout (default: 30 attempts)
TEST_DB_TIMEOUT=60

# Enable verbose logging
RUST_LOG=debug

# Test-specific logging
RUST_LOG=testcontainers=debug,sea_orm=debug
```

## Best Practices

### Test Organization

1. **Use `#[serial]`**: Always mark database tests with `#[serial]` attribute
2. **One Fixture Per Test**: Create a new `TestFixture` for each test
3. **Descriptive Names**: Use clear, descriptive test function names
4. **Group Related Tests**: Organize tests in modules by functionality

```rust
mod user_tests {
    use super::*;
    
    #[tokio::test]
    #[serial]
    async fn test_user_creation() { /* ... */ }
    
    #[tokio::test]
    #[serial]
    async fn test_user_update() { /* ... */ }
}

mod auth_tests {
    use super::*;
    
    #[tokio::test]
    #[serial]
    async fn test_oauth_flow() { /* ... */ }
}
```

### Data Management

1. **Minimal Test Data**: Only create data needed for the specific test
2. **Realistic Data**: Use realistic values that match production scenarios
3. **UUID Generation**: Use `Uuid::new_v4()` for unique identifiers
4. **Timestamp Handling**: Use `NOW()` for database timestamps

### Error Handling

1. **Expect with Messages**: Use `.expect()` with descriptive messages
2. **Test Error Scenarios**: Include tests for error conditions
3. **Cleanup on Failure**: Cleanup happens automatically even if tests fail

```rust
#[tokio::test]
#[serial]
async fn test_error_handling() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = fixture.db();
    
    // Test constraint violation
    let result = db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO users (id, provider_user_id, username) VALUES (NULL, 'test', 'test')".to_string(),
    )).await;
    
    assert!(result.is_err(), "Should fail due to NULL constraint");
}
```

### Performance Optimization

1. **Reuse Container**: The container is automatically reused across tests
2. **Efficient Cleanup**: Table truncation is faster than dropping/recreating
3. **Connection Pooling**: Use the provided connection pool for efficiency
4. **Batch Operations**: Group related database operations when possible

## Container Cleanup

The test database system includes multiple cleanup mechanisms to prevent container leaks:

### Automatic Cleanup Mechanisms

1. **Test Hooks**: Automatic cleanup on process exit using `ctor` crate
2. **Signal Handlers**: Ctrl+C cleanup using `ctrlc` crate  
3. **Just Tasks**: Manual cleanup commands via `justfile`
4. **Static Container Names**: Uses "iam-test-db" for precise cleanup targeting

### Running Tests with Cleanup

```bash
# Run all tests with automatic cleanup
just test

# Run specific test types
just test-integration          # Database integration tests
just test-single test_name     # Single test with cleanup

# Check container status
just check-containers

# Manual cleanup if needed
just cleanup-containers
```

### Container Management

The system uses a static container name (`iam-test-db`) instead of random names, allowing for:

- **Precise Cleanup**: Only test containers are affected, not development databases
- **Status Checking**: Easy identification of test containers
- **Safe Operations**: Prevents accidental deletion of other PostgreSQL instances

### Cleanup Safety Features

- **Targeted Cleanup**: Only removes containers with the specific test name
- **Error Handling**: Graceful handling of cleanup failures
- **Multiple Triggers**: Cleanup happens on normal exit, signals, and manual commands
- **Logging**: Comprehensive logging of cleanup operations

## Troubleshooting

### Common Issues

#### Container Startup Failures

```bash
# Check Docker status
docker ps
docker logs <container_id>

# Verify Docker is running
systemctl status docker  # Linux
open -a Docker           # macOS
```

#### Database Connection Issues

```bash
# Increase timeout for slower systems
TEST_DB_TIMEOUT=60 cargo test

# Enable debug logging
RUST_LOG=testcontainers=debug cargo test
```

#### Migration Failures

```bash
# Check migration status
cd migration
cargo run -- status

# Reset migrations if needed
cargo run -- down
cargo run -- up
```

#### Port Conflicts

```bash
# Check for port conflicts
netstat -an | grep LISTEN
lsof -i :5432

# Clean up containers
docker container prune -f
```

### Debugging Tools

#### Enable Verbose Logging

```bash
# Maximum verbosity
RUST_LOG=trace cargo test test_name -- --nocapture

# Database-specific logging
RUST_LOG=sqlx=debug,sea_orm=debug cargo test

# Container-specific logging
RUST_LOG=testcontainers=debug cargo test
```

#### Manual Container Inspection

```bash
# List running containers
docker ps

# Connect to test database
docker exec -it <container_id> psql -U postgres -d iam_test

# Check database tables
\dt

# Check table contents
SELECT * FROM users;
```

### Performance Issues

#### Slow Test Execution

1. **Container Reuse**: Ensure container is being reused (check logs)
2. **Docker Resources**: Increase Docker memory/CPU allocation
3. **Disk Space**: Ensure sufficient disk space for containers
4. **Network**: Check for network connectivity issues

#### Memory Usage

1. **Container Limits**: Set appropriate container resource limits
2. **Connection Pooling**: Use connection pools efficiently
3. **Cleanup**: Ensure proper cleanup between tests

### CI/CD Considerations

#### GitHub Actions

```yaml
- name: Run Database Tests
  run: |
    # Ensure Docker is available
    docker --version
    
    # Run tests with appropriate timeouts
    TEST_DB_TIMEOUT=120 cargo test --test integration_database_test
  env:
    RUST_LOG: debug
```

#### GitLab CI

```yaml
test:database:
  script:
    - docker --version
    - TEST_DB_TIMEOUT=120 cargo test --test integration_database_test
  variables:
    RUST_LOG: debug
```

## Advanced Usage

### Custom Container Configuration

If you need to customize the PostgreSQL container:

```rust
// In tests/common/database.rs, modify the postgres_image creation:
let postgres_image = Postgres::default()
    .with_db_name("custom_test_db")
    .with_user("custom_user")
    .with_password("custom_password")
    .with_env_var("POSTGRES_INITDB_ARGS", "--auth-host=scram-sha-256");
```

### Multiple Database Schemas

For testing with multiple schemas:

```rust
#[tokio::test]
#[serial]
async fn test_multiple_schemas() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = fixture.db();
    
    // Create additional schema
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "CREATE SCHEMA IF NOT EXISTS test_schema".to_string(),
    ))
    .await
    .expect("Failed to create schema");
    
    // Use schema in tests...
}
```

### Transaction Testing

For testing transaction behavior:

```rust
#[tokio::test]
#[serial]
async fn test_transactions() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = fixture.db();
    
    // Begin transaction
    let txn = db.begin().await.expect("Failed to begin transaction");
    
    // Perform operations within transaction...
    
    // Rollback or commit as needed
    txn.rollback().await.expect("Failed to rollback");
}
```

This test database system provides a robust, efficient, and maintainable foundation for database testing in your IAM service. The single container approach with table truncation ensures both performance and test isolation while maintaining simplicity. 