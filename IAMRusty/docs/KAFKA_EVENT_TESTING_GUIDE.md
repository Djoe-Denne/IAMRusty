# Kafka Event Testing Guide

## Overview

This guide explains the Kafka event testing approach in the IAM service, focusing on real container-based integration testing for event publishing verification.

## Testing Philosophy

### Design Principles

1. **Real Integration Testing**: Use actual Kafka containers with testcontainers
2. **Fail-Safe Operation**: Business operations continue when Kafka is unavailable  
3. **Environment Isolation**: Tests don't interfere with each other
4. **Configuration Integration**: Leverage existing configuration system for test setup

## Architecture

### Test Infrastructure

**Key Files**:
- `tests/common/kafka_testcontainer.rs` - Kafka container management
- `tests/signup_kafka.rs` - Integration test example
- `tests/common/mock_event_publisher.rs` - No-op publisher for tests

### Components

1. **TestKafkaFixture** - High-level Kafka test setup and teardown
2. **TestKafka** - Core Kafka testing functionality  
3. **Container Management** - Apache Kafka 3.7.0 in KRaft mode (no Zookeeper)
4. **Configuration Integration** - Uses existing configuration system for port management

### Container Configuration

**Implementation**: Apache Kafka 3.7.0 in KRaft mode for:
- **Simplified Setup**: No Zookeeper dependency
- **Faster Startup**: KRaft mode reduces container initialization time
- **Production Parity**: Matches modern Kafka deployments

## Implementation Approach

### Container Lifecycle

**Global Singleton Pattern** (see `tests/common/kafka_testcontainer.rs`):
- Single container for all tests in a file
- Configuration-integrated port management
- Automatic cleanup on test completion
- Environment variable setup for test server

### Port Management

**Configuration-Integrated Approach**:
- Uses existing configuration cache system
- Coordinates with database port allocation
- Ensures consistent port usage across test lifecycle
- Random port allocation via `port = 0` in test config

### Environment Variable Strategy

**Structured Configuration Override**:
- Uses `IAM_KAFKA__*` environment variables
- Follows same pattern as database configuration
- Runtime configuration pickup without file modifications
- Test isolation through environment scope

## Test Implementation

### Integration Test Structure

**Example**: `tests/signup_kafka.rs`

**Test Flow**:
1. **Setup Phase**: Start Kafka container and configure environment
2. **Server Phase**: Start test server that picks up Kafka configuration  
3. **Action Phase**: Execute business operation (e.g., user signup)
4. **Verification Phase**: Consume and verify events from Kafka topic
5. **Cleanup Phase**: Automatic container and environment cleanup

### Event Verification

**Approach**: Real message consumption with timeout handling

**Key Features**:
- **TestKafkaConsumer**: Dedicated consumer for test verification
- **Timeout Management**: Configurable wait times for event arrival
- **Message Parsing**: JSON event structure validation
- **Content Verification**: Event data matches operation inputs

### Error Scenarios

**Graceful Degradation Testing**:
- Verify service continues when Kafka unavailable
- Test with invalid Kafka configuration
- Validate no-op publisher behavior

## Current Implementation Details

### Test Execution

**Prerequisites**:
- Docker available for testcontainers
- Test environment configuration (`RUN_ENV=test`)
- Sufficient memory for Kafka container (~512MB)

**Execution**:
```bash
# Run Kafka integration test (ignored by default)
cargo test test_signup_kafka_integration -- --nocapture --ignored

# With debug logging
RUST_LOG=debug cargo test test_signup_kafka_integration -- --nocapture --ignored
```

### Configuration Requirements

**Test Configuration** (`config/test.toml`):
```toml
[kafka]
enabled = true
host = "127.0.0.1"
port = 0  # Triggers random port allocation
user_events_topic = "test-user-events"
```

### Message Consumption

**Consumer Strategy** (see `tests/common/kafka_testcontainer.rs`):
- Unique consumer group per test run
- Earliest offset consumption for test messages
- Configurable timeout with retry logic
- Manual commit for reliable test verification

## Event Structure Validation

### Expected Event Format

Events follow standardized structure:
- `event_type`: String identifier (e.g., "user_signed_up")  
- `event_id`: UUID for event tracking
- `user_id`: UUID of affected user
- `occurred_at`: Timestamp of event
- Event-specific data fields

### Validation Approach

**Multi-Level Verification**:
1. **Structure Validation**: Required fields present and correctly typed
2. **Content Validation**: Field values match operation inputs  
3. **Business Logic Validation**: Event data reflects actual operation results

## Integration with Test System

### Database Coordination

**Unified Test Setup** (see `tests/signup_kafka.rs`):
- Combines with `TestFixture` for database setup
- Coordinates with HTTP server testing
- Maintains test isolation through table truncation
- Shares configuration system for consistent setup

### Service Mocking Integration

**External Service Coordination**:
- Works alongside HTTP service fixtures
- Maintains mock server isolation
- Supports complete end-to-end testing flows

## Best Practices

### Test Design

1. **Use `#[ignore]` Attribute**: Kafka tests require Docker and are slower
2. **Serial Execution**: Always use `#[serial]` for integration tests
3. **Timeout Configuration**: Set appropriate waits for event publication
4. **Resource Cleanup**: Rely on automatic container cleanup

### Performance Considerations

1. **Container Reuse**: Single container per test file reduces overhead
2. **Memory Management**: Ensure sufficient Docker memory allocation
3. **Port Coordination**: Use configuration system for port management
4. **Parallel Testing**: Kafka tests run separately from other integration tests

### Debugging

1. **Comprehensive Logging**: Enable debug logging for troubleshooting
2. **Message Inspection**: Print received messages for verification
3. **Environment Validation**: Check environment variables in test output
4. **Container Status**: Monitor Docker container health during tests

## Error Handling

### Common Issues

1. **Container Startup Failures**: Verify Docker availability and memory
2. **Port Conflicts**: Ensure configuration cache coordination
3. **Consumer Timeouts**: Increase wait times or check event publishing
4. **Environment Setup**: Verify test configuration loading

### Troubleshooting Commands

```bash
# Check running containers
docker ps | grep kafka

# View container logs  
docker logs <container-id>

# Manual topic inspection
docker exec <container-id> kafka-topics.sh --list --bootstrap-server localhost:9092
```

## Current Limitations

### Test Scope

- **Single Event Type**: Currently focuses on user signup events
- **Basic Validation**: Structure and content verification only
- **Development Stage**: Tests marked as `#[ignore]` for optional execution

### Performance

- **Container Overhead**: Kafka containers add test execution time
- **Docker Dependency**: Requires Docker environment for test execution
- **Memory Requirements**: Kafka containers need substantial memory allocation

## Future Enhancements

### Potential Improvements

1. **Multiple Event Types**: Expand beyond user signup events
2. **Schema Validation**: Add formal event schema verification
3. **Performance Testing**: Load testing with high message volumes
4. **Error Scenarios**: More comprehensive failure case testing

### Implementation Roadmap

1. **Current**: Basic integration testing with user signup events
2. **Next**: Additional event types and schema validation
3. **Future**: Performance testing and production monitoring integration

## Example References

For complete implementations and usage patterns:

- **Integration Test**: `tests/signup_kafka.rs`
- **Container Management**: `tests/common/kafka_testcontainer.rs`
- **Mock Publisher**: `tests/common/mock_event_publisher.rs`
- **Configuration**: `config/test.toml`

This Kafka testing approach ensures reliable event publishing verification while maintaining test performance and isolation. 