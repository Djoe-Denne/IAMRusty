# Database Configuration Guide

This guide explains the database configuration system in the IAM application, including the structured configuration format, random port feature, and caching mechanisms.

## Overview

The database configuration uses a structured format split into separate components that can be individually configured. The configuration is handled through two layers:

1. **Core Configuration**: Provided by `rustycog-config` crate for shared configuration structures
2. **IAM Configuration**: Application-specific configuration that extends core configuration

The system supports random port allocation, connection pooling, read replicas, and configuration caching for consistent behavior across environments.

## Configuration Structure

### Current Structured Format

The database configuration is structured as follows:

```toml
[database]
# Database connection details
host = "localhost"
port = 5432  # or 0 for random port
db = "iam_db"

# Database credentials
[database.creds]
username = "postgres"
password = "postgres"

# Optional: Read replica URLs (still uses full URLs for flexibility)
read_replicas = []
```

### Configuration Components

- **`host`**: Database server hostname or IP address
- **`port`**: Database port number (see Random Port Feature below)
- **`db`**: Database name
- **`creds`**: Database credentials section
  - `username`: Database username
  - `password`: Database password
- **`read_replicas`**: Array of read replica URLs (optional)

## Random Port Feature

### Why Random Ports?

Random ports are particularly useful in testing environments where:
- Multiple test suites might run concurrently
- You want to avoid port conflicts
- Each test run should be isolated
- CI/CD environments need dynamic port allocation

### How to Use Random Ports

Set the `port` field to `0` to enable random port allocation:

```toml
[database]
host = "localhost"
port = 0  # This enables random port allocation
db = "iam_test"

[database.creds]
username = "postgres"
password = "postgres"
```

When `port = 0`, the system will:
1. Find an available random port on the system
2. Cache this port for consistency
3. Use this port for all database connections in the current process

### Port Resolution Process

1. **Check Cache**: First, check if a port has already been resolved for this configuration
2. **Generate Random Port**: If not cached, bind to `127.0.0.1:0` to get an available port
3. **Cache Port**: Store the resolved port in memory for future use
4. **Return Port**: Use the cached port for all subsequent operations

## Configuration Caching

### Why Caching?

The configuration system implements caching to ensure consistency, especially important for:
- **Random Port Consistency**: Ensures the same random port is used throughout the application lifecycle
- **Performance**: Avoids re-parsing configuration files multiple times
- **Test Isolation**: Prevents configuration changes between test setup and execution

### Cache Types

#### 1. Configuration Cache
- **Location**: `configuration/src/lib.rs`
- **Purpose**: Caches the entire `AppConfig` object
- **Key Function**: `load_config()` returns the same configuration instance

#### 2. Port Cache
- **Location**: `rustycog-config/src/lib.rs`
- **Purpose**: Caches resolved random ports
- **Key**: Combination of `host:db:username`
- **Scope**: Per-process lifetime

### Cache Implementation

The configuration cache is implemented using `OnceLock` for thread-safe lazy initialization:

```rust
/// Global configuration cache
static CONFIG_CACHE: OnceLock<Arc<Mutex<Option<AppConfig>>>> = OnceLock::new();

/// Global cache for resolved random ports to ensure consistency
static PORT_CACHE: OnceLock<Arc<Mutex<HashMap<String, u16>>>> = OnceLock::new();
```

### Cache Management

#### Clearing Caches

For testing purposes, you can clear caches:

```rust
// Clear all caches (includes configuration and port caches)
rustycog_config::clear_all_caches();

// Clear only configuration cache
configuration::clear_config_cache();

// Clear only database port cache
DatabaseConfig::clear_port_cache();
```

#### Automatic Cache Clearing

The test infrastructure automatically clears caches when creating new test containers to ensure fresh port generation.

## Configuration Loading

### Environment-Based Loading

The configuration system supports multiple environments:

```bash
# Development (default)
RUN_ENV=development

# Testing
RUN_ENV=test

# Production
RUN_ENV=production
```

### Configuration File Hierarchy

1. **Default Configuration**: `config/default.toml`
2. **Environment-Specific**: `config/{environment}.toml`
3. **Environment Variables**: `IAM_*` prefixed variables
4. **`.env` File**: Local environment overrides

### Environment Variable Mapping

You can override any configuration value using environment variables:

```bash
# Database configuration
IAM_DATABASE__HOST=db.example.com
IAM_DATABASE__PORT=5432
IAM_DATABASE__DB=production_db
IAM_DATABASE__CREDS__USERNAME=myuser
IAM_DATABASE__CREDS__PASSWORD=mypass

# Use double underscore for nested structures
IAM_DATABASE__CREDS__USERNAME=admin
```

## API Usage

### Getting Database URL

The `DatabaseConfig` provides methods to construct connection strings:

```rust
use iam-configuration::load_config;

// Load configuration
let config = load_config()?;

// Get the complete database URL
let db_url = config.database.url();
// Returns: "postgres://user:pass@host:port/db"

// Get the resolved port (handles random ports)
let port = config.database.actual_port();
```

### Creating Database Configuration

The `DatabaseConfig` struct provides several constructor methods:

```rust
use rustycog_config::DatabaseConfig;

// Create from components
let db_config = DatabaseConfig::new(
    "postgres".to_string(),
    "password".to_string(),
    "localhost".to_string(),
    5432,
    "my_db".to_string(),
);

// Create from URL (for backward compatibility)
let db_config = DatabaseConfig::from_url("postgres://user:pass@host:5432/db")?;
```

### Working with Connection Pools

```rust
use iam-infra::db::DbConnectionPool;

// Create connection pool from configuration
let pool = DbConnectionPool::new(&config.database).await?;

// Create connection pool from URL (legacy method)
let pool = DbConnectionPool::new_from_url(&db_url, vec![]).await?;

// Get connections
let write_connection = pool.get_write_connection();
let read_connection = pool.get_read_connection();
```

## Testing Integration

### Test Configuration

The test configuration (`config/test.toml`) uses random ports by default:

```toml
[database]
host = "localhost"
port = 0  # Random port for test isolation
db = "iam_test"

[database.creds]
username = "postgres"
password = "postgres"
```

### Test Container Integration

The test infrastructure integrates with testcontainers:

1. **Cache Clearing**: Clears all caches before container creation
2. **Port Coordination**: Uses the same random port for both container and application
3. **Configuration Consistency**: Ensures test fixtures use the same configuration

### Test Fixture Usage

```rust
use tests::common::database::{TestFixture, TestDatabase};

#[tokio::test]
async fn my_test() {
    // Create complete test fixture
    let fixture = TestFixture::new().await?;
    
    // Get the test configuration
    let config = fixture.config();
    
    // Use the database connection
    let db = fixture.db();
    
    // Or create just a database instance
    let test_db = TestDatabase::new().await?;
    let connection = test_db.get_connection();
    
    // Test cleanup is automatic
}
```

### Database Container Management

The test system uses a single PostgreSQL container for all tests:

```rust
/// Global test database container instance
static TEST_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestDatabaseContainer>>>>> = OnceLock::new();

/// Test database container wrapper
pub struct TestDatabaseContainer {
    container: ContainerAsync<GenericImage>,
    pub database_url: String,
    pub port: u16,
}
```

## Migration from URL-Based Configuration

### Old Format (Still Supported)

```toml
[database]
url = "postgres://user:pass@localhost:5432/db"
read_replicas = ["postgres://user:pass@replica:5432/db"]
```

### Current Format

```toml
[database]
host = "localhost"
port = 5432
db = "db"

[database.creds]
username = "user"
password = "pass"

read_replicas = ["postgres://user:pass@replica:5432/db"]
```

### Migration Steps

1. **Update Configuration Files**: Convert from URL to structured format
2. **Update Code**: Use `config.database` instead of parsing URLs manually
3. **Test**: Ensure all tests pass with new configuration
4. **Deploy**: Update production configuration files

## Configuration Examples

### Development Configuration

```toml
[database]
host = "localhost"
port = 5432
db = "iam_dev"

[database.creds]
username = "postgres"
password = "postgres"

read_replicas = []
```

### Production Configuration

```toml
[database]
host = "db.production.com"
port = 5432
db = "iam_prod"

[database.creds]
username = "postgres"
password = "postgres"

# Add read replicas for production
read_replicas = [
    "postgres://postgres:postgres@db-read-1:5432/iam_prod",
    "postgres://postgres:postgres@db-read-2:5432/iam_prod"
]
```

### Test Configuration

```toml
[database]
host = "localhost"
port = 0  # Random port for test isolation
db = "iam_test"

[database.creds]
username = "postgres"
password = "postgres"

read_replicas = []
```

## Best Practices

### Development

- Use fixed ports (`port = 5432`) for predictable local development
- Use environment variables for sensitive credentials
- Keep default configuration in `config/default.toml`

### Testing

- Always use random ports (`port = 0`) to avoid conflicts
- Clear caches between test suites if needed
- Use `TestFixture` for automatic cleanup
- Leverage table truncation for test isolation

### Production

- Use fixed ports for predictable deployment
- Override sensitive values with environment variables
- Use read replicas for better performance
- Monitor connection pool usage

### Security

- Never commit real credentials to version control
- Use environment variables for production secrets
- Rotate database passwords regularly
- Use connection pooling to limit database connections

## Advanced Features

### Connection Pool Configuration

The system supports advanced connection pool configuration:

```rust
// Custom pool configuration
let pool = DbConnectionPool::new_with_options(
    &config.database,
    sea_orm::ConnectOptions::new(db_url)
        .max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Info)
).await?;
```

### Read Replica Support

The configuration supports read replicas for better performance:

```toml
[database]
host = "primary-db.example.com"
port = 5432
db = "myapp"

[database.creds]
username = "app_user"
password = "secure_password"

read_replicas = [
    "postgres://app_user:secure_password@read-replica-1.example.com:5432/myapp",
    "postgres://app_user:secure_password@read-replica-2.example.com:5432/myapp"
]
```

### Container Integration

For containerized deployments:

```yaml
# docker-compose.yml
services:
  app:
    environment:
      - RUN_ENV=production
      - IAM_DATABASE__HOST=postgres
      - IAM_DATABASE__PORT=5432
      - IAM_DATABASE__DB=iam_prod
      - IAM_DATABASE__CREDS__USERNAME=postgres
      - IAM_DATABASE__CREDS__PASSWORD=secure_password
    depends_on:
      - postgres
  
  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=iam_prod
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=secure_password
```

## Troubleshooting

### Common Issues

#### Port Conflicts
```
Error: Address already in use (os error 98)
```
**Solution**: Use random ports (`port = 0`) or check for conflicting services.

#### Configuration Not Found
```
Failed to load configuration: Config file not found
```
**Solution**: Ensure `RUN_ENV` is set correctly and config files exist.

#### Cache Inconsistency
```
Container port differs from application port
```
**Solution**: Clear configuration caches before test runs.

#### Database Connection Failed
```
Failed to connect to database
```
**Solution**: Check database URL format, credentials, and network connectivity.

### Debug Information

Enable debug logging to see configuration loading:

```bash
RUST_LOG=debug cargo test
```

This will show:
- Configuration file loading
- Cache hits/misses
- Port resolution
- Container creation
- Database connection attempts

### Performance Troubleshooting

#### Slow Database Operations
- Check connection pool settings
- Monitor database query performance
- Consider read replicas for read-heavy workloads

#### Memory Usage
- Monitor connection pool size
- Check for connection leaks
- Verify proper cleanup in tests

## Implementation Details

### Key Files

- **`rustycog-config/src/lib.rs`**: Core configuration structures and port caching
- **`configuration/src/lib.rs`**: IAM-specific configuration loading and caching
- **`tests/common/database.rs`**: Test infrastructure integration
- **`config/*.toml`**: Environment-specific configuration files

### Dependencies

- **`serde`**: Configuration serialization/deserialization
- **`config`**: Configuration file and environment variable handling
- **`sea-orm`**: Database ORM and connection management
- **`testcontainers`**: Docker container management for tests
- **`url`**: URL parsing for backward compatibility

### Thread Safety

All caching mechanisms are thread-safe using:
- `OnceLock`: For lazy static initialization
- `Arc<Mutex<T>>`: For shared mutable state
- Atomic operations where appropriate

This ensures the configuration system works correctly in multi-threaded environments and async contexts.

## Future Considerations

### Planned Enhancements

1. **Dynamic Configuration Reloading**: Hot-reload configuration without restart
2. **Health Checks**: Built-in database health monitoring
3. **Metrics Integration**: Database performance metrics
4. **Backup Configuration**: Automated backup settings

### Deprecated Features

- **URL-based configuration**: Still supported but structured format is preferred
- **Single connection approach**: Connection pooling is now the standard

This comprehensive database configuration guide provides all the information needed to properly configure, test, and deploy the IAM application's database layer. 