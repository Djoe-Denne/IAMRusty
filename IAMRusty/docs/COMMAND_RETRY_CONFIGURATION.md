# Command Retry Configuration Guide

## Overview

The IAM service implements a sophisticated command retry system that allows for fine-grained control over retry behavior at both global and command-specific levels. This guide explains how to configure, customize, and optimize retry policies for different environments and use cases.

## Table of Contents

- [Configuration Structure](#configuration-structure)
- [Environment-Specific Settings](#environment-specific-settings)
- [Command-Specific Overrides](#command-specific-overrides)
- [Configuration Parameters](#configuration-parameters)
- [Environment Variables](#environment-variables)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)
- [Examples](#examples)

## Configuration Structure

### Basic Structure

The retry configuration is defined in TOML files under the `[command]` section:

```toml
[command.retry]
# Default retry configuration for all commands
max_attempts = 3
base_delay_ms = 100
max_delay_ms = 30000
backoff_multiplier = 2.0
use_jitter = true

# Command-specific overrides
[command.overrides.<command_type>]
max_attempts = 5
base_delay_ms = 50
max_delay_ms = 10000
backoff_multiplier = 1.5
use_jitter = false
```

### Configuration Hierarchy

The system resolves retry policies in the following order of precedence:

1. **Command-specific override**: `[command.overrides.<command_type>]`
2. **Default command configuration**: `[command.retry]`
3. **System default**: Hardcoded fallback values

## Environment-Specific Settings

### Development Environment

**File**: `config/development.toml`

```toml
[command.retry]
# More lenient retry settings for development
max_attempts = 5
base_delay_ms = 200
max_delay_ms = 10000
backoff_multiplier = 2.0
use_jitter = true

# Fast retries for testing specific commands
[command.overrides.test_command]
max_attempts = 2
base_delay_ms = 100
max_delay_ms = 5000
backoff_multiplier = 1.5
use_jitter = false
```

**Characteristics**:
- Higher retry counts for development convenience
- Longer delays to avoid overwhelming development services
- Jitter enabled to simulate production conditions

### Testing Environment

**File**: `config/test.toml`

```toml
[command.retry]
# Faster retry settings for tests
max_attempts = 2
base_delay_ms = 50
max_delay_ms = 5000
backoff_multiplier = 2.0
use_jitter = false  # Disable jitter for predictable test behavior

# No command-specific overrides for consistent test behavior
```

**Characteristics**:
- Lower retry counts for faster test execution
- Short delays to minimize test duration
- **Jitter disabled** for predictable and reproducible test results
- Minimal or no command-specific overrides for consistency

### Production Environment

**File**: `config/production.toml`

```toml
[command.retry]
# Conservative retry settings for production
max_attempts = 3
base_delay_ms = 500
max_delay_ms = 60000  # 1 minute max delay
backoff_multiplier = 2.0
use_jitter = true

# Critical commands get more retries
[command.overrides.critical_command]
max_attempts = 5
base_delay_ms = 1000
max_delay_ms = 30000
backoff_multiplier = 1.8
use_jitter = true

# Sensitive commands get fewer retries
[command.overrides.sensitive_operation]
max_attempts = 2
base_delay_ms = 2000
max_delay_ms = 10000
backoff_multiplier = 1.5
use_jitter = true
```

**Characteristics**:
- Conservative retry counts to avoid amplifying problems
- Longer delays to be respectful of external services
- **Jitter enabled** to avoid thundering herd problems
- Command-specific tuning based on business criticality

## Command-Specific Overrides

### When to Use Overrides

Use command-specific overrides when:

1. **Different Criticality Levels**: Critical operations need more retries
2. **External Service Constraints**: Some services have rate limits or longer response times
3. **Business Requirements**: Certain operations have specific SLA requirements
4. **Risk Management**: Sensitive operations should have fewer retries

### Common Override Patterns

#### Authentication Commands
```toml
[command.overrides.login_command]
max_attempts = 5          # Authentication is critical
base_delay_ms = 100       # Fast retries for user experience
max_delay_ms = 5000       # Don't make users wait too long
backoff_multiplier = 1.5  # Moderate backoff
use_jitter = true
```

#### External API Calls
```toml
[command.overrides.oauth_provider_call]
max_attempts = 3          # External service limits
base_delay_ms = 1000      # Respect external rate limits
max_delay_ms = 30000      # Allow for external service recovery
backoff_multiplier = 2.0  # Standard exponential backoff
use_jitter = true
```

#### Database Operations
```toml
[command.overrides.database_operation]
max_attempts = 5          # Database should be reliable
base_delay_ms = 50        # Quick retries for transient issues
max_delay_ms = 10000      # Don't hold connections too long
backoff_multiplier = 2.0
use_jitter = false        # Consistent timing for db connections
```

## Configuration Parameters

### max_attempts

**Type**: `u32`  
**Default**: `3`  
**Range**: `1` to `10` (recommended)

Number of total attempts (including the initial attempt).

```toml
max_attempts = 3  # Initial attempt + 2 retries
```

**Guidelines**:
- **1**: For operations that should never retry (idempotency concerns)
- **2-3**: For most operations (good balance)
- **4-5**: For critical operations
- **>5**: Rarely recommended (can amplify problems)

### base_delay_ms

**Type**: `u64`  
**Default**: `100`  
**Range**: `10` to `5000` milliseconds (recommended)

The initial delay before the first retry.

```toml
base_delay_ms = 100  # 100ms initial delay
```

**Guidelines**:
- **10-50ms**: For internal operations (database, cache)
- **100-500ms**: For most operations
- **500-2000ms**: For external API calls
- **>2000ms**: For rate-limited or slow external services

### max_delay_ms

**Type**: `u64`  
**Default**: `30000` (30 seconds)  
**Range**: `1000` to `300000` milliseconds (5 minutes max recommended)

Maximum delay between retries (caps exponential backoff).

```toml
max_delay_ms = 30000  # 30 second maximum delay
```

**Guidelines**:
- **1-5 seconds**: For user-facing operations
- **10-30 seconds**: For background operations
- **30-60 seconds**: For batch operations
- **>60 seconds**: For non-time-sensitive operations

### backoff_multiplier

**Type**: `f64`  
**Default**: `2.0`  
**Range**: `1.0` to `3.0` (recommended)

Multiplier for exponential backoff.

```toml
backoff_multiplier = 2.0  # Double delay each retry
```

**Guidelines**:
- **1.0**: Linear backoff (constant delay)
- **1.2-1.5**: Gentle exponential backoff
- **2.0**: Standard exponential backoff
- **>2.5**: Aggressive exponential backoff

### use_jitter

**Type**: `bool`  
**Default**: `true`

Whether to add random jitter to retry delays.

```toml
use_jitter = true  # Add up to 25% random jitter
```

**Guidelines**:
- **true**: For production (prevents thundering herd)
- **false**: For testing (predictable timing)
- **false**: For single-instance deployments

## Environment Variables

Override any configuration value using environment variables with the `IAM_` prefix:

### Basic Examples

```bash
# Override default retry attempts
export IAM_COMMAND__RETRY__MAX_ATTEMPTS=5

# Override default base delay
export IAM_COMMAND__RETRY__BASE_DELAY_MS=200

# Disable jitter
export IAM_COMMAND__RETRY__USE_JITTER=false
```

### Command-Specific Overrides

```bash
# Override login command retry attempts
export IAM_COMMAND__OVERRIDES__LOGIN_COMMAND__MAX_ATTEMPTS=3

# Override OAuth provider call delays
export IAM_COMMAND__OVERRIDES__OAUTH_PROVIDER_CALL__BASE_DELAY_MS=1000
export IAM_COMMAND__OVERRIDES__OAUTH_PROVIDER_CALL__MAX_DELAY_MS=30000
```

### Docker/Kubernetes Examples

```yaml
# docker-compose.yml
environment:
  - IAM_COMMAND__RETRY__MAX_ATTEMPTS=3
  - IAM_COMMAND__RETRY__BASE_DELAY_MS=500
  - IAM_COMMAND__OVERRIDES__LOGIN_COMMAND__MAX_ATTEMPTS=5

# kubernetes deployment
env:
  - name: IAM_COMMAND__RETRY__MAX_ATTEMPTS
    value: "3"
  - name: IAM_COMMAND__RETRY__BASE_DELAY_MS
    value: "500"
```

## Best Practices

### 1. Environment-Specific Tuning

- **Development**: Higher retry counts, longer delays
- **Testing**: Lower retry counts, no jitter, minimal overrides
- **Production**: Conservative settings, jitter enabled

### 2. Command Classification

Classify commands by their characteristics:

- **Critical**: Authentication, payment processing
- **External**: OAuth providers, third-party APIs
- **Internal**: Database operations, cache operations
- **Sensitive**: Account modifications, security operations

### 3. Monitoring and Adjustment

Monitor these metrics to tune configuration:

- **Success rate after retries**
- **Average retry count per command**
- **Total execution time including retries**
- **Error distribution by command type**

### 4. Configuration Management

- **Version control all configuration files**
- **Document reasons for command-specific overrides**
- **Use environment variables for runtime adjustments**
- **Test configuration changes in non-production first**

### 5. Safety Guidelines

- **Avoid very high retry counts** (>10)
- **Set reasonable maximum delays** (<5 minutes)
- **Use jitter in production** to prevent thundering herd
- **Consider external service limits** when setting delays

## Troubleshooting

### Common Issues

#### 1. Commands Never Succeed

**Symptoms**: Commands always fail even with retries
**Causes**: 
- Business logic errors (should not be retried)
- Validation errors (should not be retried)
- Persistent infrastructure issues

**Solutions**:
- Check error classification in logs
- Verify error mapping in command handlers
- Ensure retryable vs non-retryable errors are properly classified

#### 2. Too Many Retries

**Symptoms**: High retry counts, increased latency
**Causes**:
- Infrastructure issues
- External service problems
- Too aggressive retry configuration

**Solutions**:
- Review `max_attempts` settings
- Increase `base_delay_ms` and `max_delay_ms`
- Check external service health

#### 3. Retry Exhaustion

**Symptoms**: Many `RetryExhausted` errors
**Causes**:
- Persistent failures
- Insufficient retry attempts
- External service degradation

**Solutions**:
- Investigate root cause of failures
- Temporarily increase `max_attempts` if needed
- Implement circuit breaker patterns

#### 4. Unpredictable Test Behavior

**Symptoms**: Flaky tests, inconsistent timing
**Causes**:
- Jitter enabled in test configuration
- Non-deterministic retry behavior

**Solutions**:
- Set `use_jitter = false` in test configuration
- Use shorter, predictable delays in tests
- Mock external dependencies

### Debugging Configuration

#### View Current Configuration

The system logs the effective configuration at startup:

```json
{
  "level": "INFO",
  "message": "Command retry configuration loaded",
  "default_policy": {
    "max_attempts": 3,
    "base_delay_ms": 100,
    "max_delay_ms": 30000,
    "backoff_multiplier": 2.0,
    "use_jitter": true
  },
  "overrides": {
    "login_command": {
      "max_attempts": 5,
      "base_delay_ms": 50
    }
  }
}
```

#### Test Configuration Changes

Use the configuration test utility:

```bash
# Test configuration loading
cargo run --bin config-test

# Test specific command retry resolution
cargo run --bin config-test -- --command login_command
```

## Examples

### Complete Production Configuration

```toml
# config/production.toml

[command.retry]
# Conservative production defaults
max_attempts = 3
base_delay_ms = 500
max_delay_ms = 60000  # 1 minute
backoff_multiplier = 2.0
use_jitter = true

# Authentication - critical, user-facing
[command.overrides.login_command]
max_attempts = 5
base_delay_ms = 200
max_delay_ms = 10000  # 10 seconds max
backoff_multiplier = 1.8
use_jitter = true

[command.overrides.link_provider_command]
max_attempts = 4
base_delay_ms = 300
max_delay_ms = 15000
backoff_multiplier = 2.0
use_jitter = true

# External OAuth providers - respect rate limits
[command.overrides.oauth_github_call]
max_attempts = 3
base_delay_ms = 1000  # 1 second base
max_delay_ms = 30000  # 30 seconds max
backoff_multiplier = 2.2
use_jitter = true

[command.overrides.oauth_gitlab_call]
max_attempts = 3
base_delay_ms = 1200
max_delay_ms = 45000
backoff_multiplier = 2.5
use_jitter = true

# Database operations - should be reliable
[command.overrides.user_create]
max_attempts = 4
base_delay_ms = 100
max_delay_ms = 5000
backoff_multiplier = 2.0
use_jitter = false  # Consistent DB timing

# Sensitive operations - fewer retries
[command.overrides.account_deletion]
max_attempts = 2
base_delay_ms = 2000
max_delay_ms = 10000
backoff_multiplier = 1.5
use_jitter = true
```

### Development Configuration

```toml
# config/development.toml

[command.retry]
# Generous development defaults
max_attempts = 5
base_delay_ms = 200
max_delay_ms = 15000
backoff_multiplier = 2.0
use_jitter = true

# Quick testing of specific functionality
[command.overrides.test_command]
max_attempts = 2
base_delay_ms = 50
max_delay_ms = 2000
backoff_multiplier = 1.5
use_jitter = false
```

### Testing Configuration

```toml
# config/test.toml

[command.retry]
# Fast, predictable test settings
max_attempts = 2
base_delay_ms = 50
max_delay_ms = 2000
backoff_multiplier = 2.0
use_jitter = false  # Critical for test determinism

# No command-specific overrides for consistent test behavior
```

This comprehensive retry configuration system provides the flexibility to tune retry behavior for different environments, command types, and operational requirements while maintaining consistency and reliability across the IAM service. 