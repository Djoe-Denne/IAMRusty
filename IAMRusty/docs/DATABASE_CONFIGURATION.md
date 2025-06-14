# Database Configuration Guide

This guide explains the database configuration system in the IAM application, including the structured configuration format, random port feature, and caching mechanisms.

## Overview

The database configuration has been redesigned to provide more flexibility and better support for testing environments. Instead of using a single database URL, the configuration is now split into separate components that can be individually configured.

## Configuration Structure

### New Structured Format

The database configuration is now structured as follows:

```toml
[database]
# Database credentials
[database.creds]
username = "postgres"
password = "postgres"

# Database connection details
host = "localhost"
port = 5432  # or 0 for random port
db = "iam_db"

# Optional: Read replica URLs (still uses full URLs for flexibility)
read_replicas = []
```

### Configuration Components

- **`creds`**: Database credentials section
  - `username`: Database username
  - `password`: Database password
- **`host`**: Database server hostname or IP address
- **`port`**: Database port number (see Random Port Feature below)
- **`db`**: Database name
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
[database.creds]
username = "postgres"
password = "postgres"
host = "localhost"
port = 0  # This enables random port allocation
db = "iam_test"
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
- **Location**: `infra/src/config/mod.rs`
- **Purpose**: Caches the entire `AppConfig` object
- **Key Function**: `load_config()` returns the same configuration instance

#### 2. Port Cache
- **Location**: `application/src/config.rs`
- **Purpose**: Caches resolved random ports
- **Key**: Combination of `host:db:username`
- **Scope**: Per-process lifetime

### Cache Management

#### Clearing Caches

For testing purposes, you can clear caches:

```rust
// Clear all caches
infra::config::clear_all_caches();

// Clear only configuration cache
infra::config::clear_config_cache();

// Clear only port cache
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
IAM_DATABASE__CREDS__USERNAME=myuser
IAM_DATABASE__CREDS__PASSWORD=mypass
IAM_DATABASE__HOST=db.example.com
IAM_DATABASE__PORT=5432
IAM_DATABASE__DB=production_db

# Use double underscore for nested structures
IAM_DATABASE__CREDS__USERNAME=admin
```

## API Usage

### Getting Database URL

The `DatabaseConfig` provides methods to construct connection strings:

```rust
use infra::config::load_config;

// Load configuration
let config = load_config()?;

// Get the complete database URL
let db_url = config.database.url();
// Returns: "postgres://user:pass@host:port/db"

// Get the resolved port (handles random ports)
let port = config.database.actual_port();
```

### Backward Compatibility

For legacy code that expects URL strings:

```rust
// Create from URL
let db_config = DatabaseConfig::from_url("postgres://user:pass@host:5432/db")?;

// Use with connection pool
let pool = DbConnectionPool::new_from_url(&db_url, vec![]).await?;
```

## Testing Integration

### Test Configuration

The test configuration (`config/test.toml`) uses random ports by default:

```toml
[database]
[database.creds]
username = "postgres"
password = "postgres"
host = "localhost"
port = 0  # Random port for test isolation
db = "iam_test"
```

### Test Container Integration

The test infrastructure integrates with testcontainers:

1. **Cache Clearing**: Clears all caches before container creation
2. **Port Coordination**: Uses the same random port for both container and application
3. **Configuration Consistency**: Ensures test fixtures use the same configuration

### Test Fixture Usage

```rust
use tests::common::database::TestFixture;

#[tokio::test]
async fn my_test() {
    let fixture = TestFixture::new().await?;
    
    // Get the test configuration
    let config = fixture.config();
    
    // Use the database connection
    let db = _fixture.db();
    
    // Test cleanup is automatic
}
```

## Migration from URL-Based Configuration

### Old Format (Deprecated)

```toml
[database]
url = "postgres://user:pass@localhost:5432/db"
read_replicas = ["postgres://user:pass@replica:5432/db"]
```

### New Format

```toml
[database]
[database.creds]
username = "user"
password = "pass"
host = "localhost"
port = 5432
db = "db"
read_replicas = ["postgres://user:pass@replica:5432/db"]
```

### Migration Steps

1. **Update Configuration Files**: Convert from URL to structured format
2. **Update Code**: Use `config.database` instead of `config.database.url`
3. **Test**: Ensure all tests pass with new configuration
4. **Deploy**: Update production configuration files

## Best Practices

### Development

- Use fixed ports (`port = 5432`) for predictable local development
- Use environment variables for sensitive credentials
- Keep default configuration in `config/default.toml`

### Testing

- Always use random ports (`port = 0`) to avoid conflicts
- Clear caches between test suites if needed
- Use `TestFixture` for automatic cleanup

### Production

- Use fixed ports for predictable deployment
- Override sensitive values with environment variables
- Use read replicas for better performance

### Security

- Never commit real credentials to version control
- Use environment variables for production secrets
- Rotate database passwords regularly

## Troubleshooting

### Common Issues

#### Port Conflicts
```
Error: Address already in use (os error 98)
```
**Solution**: Use random ports (`port = 0`) or check for conflicting services.

#### Configuration Not Found
```
Failed to load test configuration
```
**Solution**: Ensure `RUN_ENV=test` is set and `config/test.toml` exists.

#### Inconsistent Ports
```
Container port differs from application port
```
**Solution**: Clear configuration caches before test runs.

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

## Implementation Details

### Key Files

- **`application/src/config.rs`**: Core configuration structures and port caching
- **`infra/src/config/mod.rs`**: Configuration loading and caching
- **`tests/common/database.rs`**: Test infrastructure integration
- **`config/*.toml`**: Environment-specific configuration files

### Dependencies

- **`url`**: URL parsing for backward compatibility
- **`serde`**: Configuration serialization/deserialization
- **`config`**: Configuration file and environment variable handling
- **`testcontainers`**: Docker container management for tests

### Thread Safety

All caching mechanisms are thread-safe using:
- `OnceLock`: For lazy static initialization
- `Arc<Mutex<T>>`: For shared mutable state
- Atomic operations where appropriate

This ensures the configuration system works correctly in multi-threaded environments and async contexts. 