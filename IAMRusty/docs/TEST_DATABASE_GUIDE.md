# Test Database Guide

Comprehensive guide for using the test database system with testcontainers, single container management, and table truncation for unparallelizable tests.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Usage Patterns](#usage-patterns)
- [Database Fixtures Integration](#database-fixtures-integration)
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
- **Database Fixtures**: Integrated with comprehensive DB fixture system for entity creation and validation

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

## Database Fixtures Integration

The test database system integrates seamlessly with the comprehensive database fixture system, providing a fluent API for entity creation and validation. This combination offers the best of both worlds: reliable database infrastructure and convenient entity management.

### Overview

The database fixtures provide:

- **Fluent API**: Chainable methods for entity creation
- **Type Safety**: Strongly typed entity builders
- **Factory Methods**: Pre-built entities for common scenarios
- **Validation**: Check methods to verify entity state against database
- **Automatic Cleanup**: Works with table truncation for test isolation

### Basic Integration

```rust
mod common;
mod fixtures;

use common::TestFixture;
use fixtures::DbFixtures;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_user_with_fixtures() {
    // Setup test database
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create user using database fixtures
    let user = DbFixtures::user()
        .username("test_user")
        .avatar_url(Some("https://example.com/avatar.png".to_string()))
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Verify the user exists in the database
    assert!(user.check(db.clone()).await.expect("Failed to check user"));
    
    // Access user properties
    assert_eq!(user.username(), "test_user");
    assert_eq!(user.avatar_url(), Some(&"https://example.com/avatar.png".to_string()));
    
    debug!("✅ User created with ID: {}", user.id());
    
    // Table truncation will clean up automatically
}
```

### Factory Methods with Test Database

```rust
#[tokio::test]
#[serial]
async fn test_factory_methods_integration() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create users using factory methods
    let arthur = DbFixtures::user().arthur().commit(db.clone()).await?;
    let bob = DbFixtures::user().bob().commit(db.clone()).await?;
    let alice = DbFixtures::user().alice().commit(db.clone()).await?;
    
    // Verify factory method data
    assert_eq!(arthur.username(), "arthur");
    assert_eq!(bob.username(), "bob");
    assert_eq!(alice.username(), "alice");
    
    // All users should exist in database
    assert!(arthur.check(db.clone()).await?);
    assert!(bob.check(db.clone()).await?);
    assert!(alice.check(db.clone()).await?);
    
    // Verify in database using raw SQL
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
    
    assert_eq!(count, 3);
}
```

### Complete Entity Relationships

```rust
#[tokio::test]
#[serial]
async fn test_complete_user_setup_with_fixtures() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a complete user setup with all related entities
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await?;
    
    // Add primary email
    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await?;
    
    // Add GitHub provider token
    let github_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await?;
    
    // Add refresh token
    let refresh_token = DbFixtures::refresh_token()
        .arthur_valid(user.id())
        .commit(db.clone())
        .await?;
    
    // Verify all entities exist using fixtures
    assert!(user.check(db.clone()).await?);
    assert!(primary_email.check(db.clone()).await?);
    assert!(github_token.check(db.clone()).await?);
    assert!(refresh_token.check(db.clone()).await?);
    
    // Verify relationships
    assert_eq!(primary_email.user_id(), user.id());
    assert_eq!(github_token.user_id(), user.id());
    assert_eq!(refresh_token.user_id(), user.id());
    
    // Verify using raw SQL queries
    let email_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT COUNT(*) as count FROM user_emails WHERE user_id = '{}'", user.id()),
        ))
        .await?
        .unwrap()
        .try_get("", "count")?;
    
    assert_eq!(email_count, 1);
    
    debug!("✅ Complete user setup created:");
    debug!("   User: {} ({})", user.username(), user.id());
    debug!("   Primary Email: {} ({})", primary_email.email(), primary_email.id());
    debug!("   GitHub Token: {} ({})", github_token.provider_user_id(), github_token.id());
    debug!("   Refresh Token: {} ({})", refresh_token.is_usable(), refresh_token.id());
}
```

### Fixture Validation and Database State

```rust
#[tokio::test]
#[serial]
async fn test_fixture_validation_with_database() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create and commit a user fixture
    let original_user = DbFixtures::user()
        .username("original_user")
        .avatar_url(Some("https://example.com/original.png".to_string()))
        .commit(db.clone())
        .await?;
    
    // Initially, the fixture should match the database
    assert!(original_user.check(db.clone()).await?);
    
    // Manually modify the database using raw SQL
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!(
            "UPDATE users SET username = 'modified_user', avatar_url = 'https://example.com/modified.png' WHERE id = '{}'",
            original_user.id()
        ),
    ))
    .await?;
    
    // The original fixture should now fail the check
    let check_result = original_user.check(db.clone()).await?;
    assert!(!check_result, "Original fixture should fail check after database was modified");
    
    // Verify the original fixture still has the old data (unchanged)
    assert_eq!(original_user.username(), "original_user");
    assert_eq!(original_user.avatar_url(), Some(&"https://example.com/original.png".to_string()));
    
    // Verify the database has the new data
    let updated_user = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT username, avatar_url FROM users WHERE id = '{}'", original_user.id()),
        ))
        .await?
        .unwrap();
    
    let db_username: String = updated_user.try_get("", "username")?;
    let db_avatar_url: Option<String> = updated_user.try_get("", "avatar_url")?;
    
    assert_eq!(db_username, "modified_user");
    assert_eq!(db_avatar_url, Some("https://example.com/modified.png".to_string()));
    
    debug!("✅ Fixture validation correctly detected database changes");
}
```

### Combining with Configuration

```rust
#[tokio::test]
#[serial]
async fn test_fixtures_with_configuration() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let config = test_fixture.config();
    
    // Create user with configuration-aware data
    let user = DbFixtures::user()
        .username("config_test_user")
        .commit(db.clone())
        .await?;
    
    // Create provider token using configuration values
    let github_token = DbFixtures::provider_token()
        .user_id(user.id())
        .provider("github")
        .access_token("test_access_token")
        .provider_user_id("github_user_123")
        .commit(db.clone())
        .await?;
    
    // Verify entities exist
    assert!(user.check(db.clone()).await?);
    assert!(github_token.check(db.clone()).await?);
    
    // Use configuration in test logic
    assert_eq!(config.oauth.github.client_id, "test_github_client_id");
    assert_eq!(github_token.provider(), "github");
    
    debug!("✅ Fixtures work seamlessly with test configuration");
    debug!("   GitHub Client ID: {}", config.oauth.github.client_id);
    debug!("   User ID: {}", user.id());
    debug!("   Token Provider: {}", github_token.provider());
}
```

### Performance Benefits

The combination of test database and fixtures provides excellent performance:

1. **Single Container**: Database container is reused across all tests
2. **Table Truncation**: Fast cleanup between tests (faster than container recreation)
3. **Efficient Fixtures**: Fixtures use prepared statements and optimized queries
4. **Connection Pooling**: Shared connection pool reduces overhead
5. **Batch Operations**: Fixtures can be created in batches for complex scenarios

### Best Practices for Integration

1. **Use Both Systems**: Combine raw SQL for complex queries with fixtures for entity creation
2. **Validate with Fixtures**: Use fixture check methods for entity validation
3. **Factory Methods**: Prefer factory methods for common test scenarios
4. **Custom Data**: Use fluent API for test-specific entity customization
5. **Relationships**: Create parent entities before child entities
6. **Serial Tests**: Always use `#[serial]` for database tests
7. **Error Handling**: Use descriptive error messages with `.expect()`

### Available Database Fixtures

The following entity fixtures are available for use with the test database:

- **User Fixtures**: `DbFixtures::user()` - Create users with factory methods (arthur, bob, alice, charlie)
- **User Email Fixtures**: `DbFixtures::user_email()` - Create user emails with primary/secondary options
- **Provider Token Fixtures**: `DbFixtures::provider_token()` - Create OAuth provider tokens (GitHub, GitLab)
- **Refresh Token Fixtures**: `DbFixtures::refresh_token()` - Create JWT refresh tokens with expiration

For detailed documentation on each fixture type, see the [Fixtures Guide](FIXTURES_GUIDE.md#database-fixtures).

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