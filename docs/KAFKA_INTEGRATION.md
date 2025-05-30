# Kafka Integration Documentation

## Overview

The IAM service includes real Kafka event publishing capabilities for domain events. This integration ensures that important events (like user signups, logins, etc.) are published to Kafka topics for consumption by other services.

## Implementation

### Real Kafka Event Publisher

**Location:** `infra/src/event/kafka.rs`

The `KafkaEventPublisher` implements the `EventPublisher` trait from the domain layer and provides:

- **Asynchronous event publishing** using `rdkafka` with Tokio support
- **SSL/TLS support** for secure connections
- **SASL authentication** (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512, GSSAPI, OAUTHBEARER)
- **Retry logic** with configurable max retries
- **Batch publishing** for improved performance
- **Health checks** for monitoring connectivity
- **Graceful error handling** that doesn't block business operations

### Configuration

**Location:** `configuration/src/lib.rs`

Kafka configuration uses a structured format similar to database configuration:

```toml
[kafka]
enabled = true
host = "localhost"
port = 9092  # Use 0 for random port (useful for testing)
user_events_topic = "user-events"
client_id = "iam-service"
timeout_ms = 5000
max_retries = 3
compression = "gzip"
security_protocol = "plaintext"

# Multi-broker setup (optional)
additional_brokers = ["broker2:9093", "broker3:9094"]

# SSL Configuration (optional)
# ssl_ca_location = "/path/to/ca-cert"
# ssl_certificate_location = "/path/to/client-cert"
# ssl_key_location = "/path/to/client-key"
# ssl_key_password = "key-password"

# SASL Configuration (optional)
# sasl_mechanism = "PLAIN"
# sasl_username = "username"
# sasl_password = "password"
```

### Random Port Feature

Similar to the database configuration, Kafka now supports random port allocation for testing:

```toml
[kafka]
enabled = true
host = "localhost"
port = 0  # Random port - useful for testing
user_events_topic = "test-user-events"
client_id = "iam-service-test"
```

When `port = 0`, the system will:
1. Find an available random port on the system
2. Cache this port for consistency
3. Use this port for all Kafka connections in the current process

This feature is particularly useful for:
- **Concurrent test execution** without port conflicts
- **CI/CD environments** with dynamic port allocation
- **Development environments** with multiple service instances

### Environment Variable Support

Configuration can be overridden using environment variables with the `IAM_` prefix:

```bash
export IAM_KAFKA__ENABLED=true
export IAM_KAFKA__HOST=localhost
export IAM_KAFKA__PORT=9092
export IAM_KAFKA__USER_EVENTS_TOPIC=user-events
```

### Event Factory Integration

**Location:** `infra/src/event/factory.rs`

The event factory creates the appropriate event publisher based on configuration:

- **Real Kafka publishing** when `kafka.enabled = true`
- **Mock publishing** when disabled (useful for development/testing)

### Event Structure

Events published to Kafka include:

```json
{
  "event_type": "user_signed_up",
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "550e8400-e29b-41d4-a716-446655440001", 
  "email": "user@example.com",
  "username": "username",
  "email_verified": false,
  "occurred_at": "2024-01-01T00:00:00Z"
}
```

### Headers

Kafka messages include headers for easier consumption:

- `event_id`: UUID of the event
- `event_type`: Type of domain event
- `user_id`: UUID of the user (used as partition key)

## Configuration API

### KafkaConfig Methods

```rust
use configuration::KafkaConfig;

// Create new configuration
let config = KafkaConfig::new(
    "localhost".to_string(),
    9092,
    "user-events".to_string(),
    "iam-service".to_string(),
);

// Get brokers string for Kafka client
let brokers = config.brokers(); // Returns "localhost:9092"

// Get resolved port (handles random ports)
let port = config.actual_port(); // Returns 9092 or random port if port = 0

// Create from brokers string (backward compatibility)
let config = KafkaConfig::from_brokers("broker1:9092,broker2:9093")?;

// Clear port cache (useful for testing)
KafkaConfig::clear_port_cache();
```

### Multi-Broker Support

For Kafka clusters with multiple brokers:

```toml
[kafka]
host = "primary-broker"
port = 9092
additional_brokers = [
    "broker2.cluster.com:9093", 
    "broker3.cluster.com:9094"
]
```

The `brokers()` method will return: `"primary-broker:9092,broker2.cluster.com:9093,broker3.cluster.com:9094"`

## Testing

### Unit Tests

**Location:** `infra/src/event/kafka.rs`

Basic unit tests verify:
- Configuration creation
- Event serialization 
- Error handling

### Integration Tests

**Location:** `tests/auth_email_password.rs`

The `test_signup_kafka_integration` test provides:

- **Real Kafka container** using testcontainers
- **End-to-end testing** of event publishing
- **Environment variable configuration**
- **Verification of successful integration**

### Test Infrastructure

**Location:** `tests/common/kafka_testcontainer.rs`

Provides:
- **Kafka testcontainer setup** using Apache Kafka 3.7.0 in KRaft mode
- **Configuration integration** with random port support
- **Container lifecycle management** with cleanup
- **Environment variable injection** for seamless config override

The test infrastructure now uses the configuration system's random port mechanism:

```rust
// Load test configuration
let config = infra::config::load_config()?;
let kafka_config = &config.kafka;

// Use configuration's port resolution
let kafka_port = kafka_config.actual_port();
```

## Usage

### Development

For development, you can disable Kafka publishing:

```toml
[kafka]
enabled = false
```

Events will be handled by the mock publisher and logged.

### Testing

For testing with random ports:

```toml
[kafka]
enabled = true
host = "localhost"
port = 0  # Random port to avoid conflicts
user_events_topic = "test-user-events"
client_id = "iam-service-test"
```

### Production

For production, configure real Kafka brokers:

```toml
[kafka]
enabled = true
host = "kafka1.prod.com"
port = 9092
additional_brokers = ["kafka2.prod.com:9092", "kafka3.prod.com:9092"]
user_events_topic = "user-events"
client_id = "iam-service-prod"
security_protocol = "ssl"
ssl_ca_location = "/etc/ssl/ca-cert"
ssl_certificate_location = "/etc/ssl/client-cert"
ssl_key_location = "/etc/ssl/client-key"
```

### SSL Configuration

For secure environments, configure SSL:

```toml
[kafka]
security_protocol = "ssl"
ssl_ca_location = "probe"  # Use system CA certificates
ssl_certificate_location = "/path/to/client.crt"
ssl_key_location = "/path/to/client.key"
ssl_key_password = "optional-key-password"
```

### SASL Authentication

For SASL authentication:

```toml
[kafka]
security_protocol = "sasl_ssl"
sasl_mechanism = "SCRAM-SHA-256"
sasl_username = "iam-service"
sasl_password = "secure-password"
```

## Port Management

### Random Port Allocation

The Kafka configuration includes a sophisticated port management system:

1. **Cache Key Generation**: Each configuration gets a unique cache key based on `host:client_id`
2. **Port Resolution**: Random ports are resolved once and cached for consistency
3. **Cache Management**: Port cache can be cleared for testing isolation

### Configuration Consistency

The port caching ensures that:
- **Same configuration** always gets the same port
- **Different configurations** get different ports
- **Test isolation** is maintained through cache clearing

## Error Handling

The Kafka integration follows a "fail-open" approach:

- **Non-blocking**: Business operations continue even if Kafka is unavailable
- **Comprehensive logging**: All publish attempts are logged for monitoring
- **Graceful degradation**: Errors are captured but don't propagate to business logic
- **Health checks**: Connectivity can be monitored via health check endpoints

## Monitoring

Events are logged at appropriate levels:

- **DEBUG**: Individual event publishing success/failure
- **INFO**: Batch publishing results, health check status
- **WARN**: Retry attempts, configuration issues
- **ERROR**: Critical failures, connection issues

## Dependencies

- `rdkafka = "0.36.2"` with features `["cmake-build", "tokio"]`
- Integration with existing configuration system
- Async/await support via Tokio

## Running Tests

To run the Kafka integration test:

```bash
# Set test environment
export RUN_ENV=test

# Run specific Kafka test
cargo test test_signup_kafka_integration

# Run with output
cargo test test_signup_kafka_integration -- --nocapture
```

The test will:
1. Start a real Kafka container using random port
2. Configure the IAM service to use it
3. Perform a user signup
4. Verify the integration works end-to-end

## Configuration Migration

### From Old Format

If you have existing configurations using the old `brokers` field:

```toml
# Old format (deprecated)
[kafka]
brokers = "localhost:9092"
```

### To New Format

Update to the new structured format:

```toml
# New format
[kafka]
host = "localhost"
port = 9092
```

### Backward Compatibility

The system provides backward compatibility through the `from_brokers()` method for programmatic configuration.

## Notes

- **No SSL during build**: SSL features are handled at runtime to avoid slow compilation on Windows
- **Graceful runtime SSL**: SSL configuration is applied only when needed
- **Environment variable overrides**: Easy integration testing without config file changes
- **Container coordination**: Test containers coordinate with configuration system for port management
- **Real Kafka testing**: Uses actual Kafka instead of mocks for integration validation 