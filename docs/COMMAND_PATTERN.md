# Command Design Pattern Implementation

## Overview

This document describes the implementation of the Command Design Pattern in the IAM service, which provides centralized handling of cross-cutting concerns including retries, logging, metrics, and tracing for authentication operations.

## Table of Contents

- [Architecture](#architecture)
- [Components](#components)
- [Benefits](#benefits)
- [Implementation Details](#implementation-details)
- [Usage Examples](#usage-examples)
- [Configuration](#configuration)
- [Monitoring and Observability](#monitoring-and-observability)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)

## Architecture

The Command Pattern implementation follows a layered architecture that separates command definition, execution, and cross-cutting concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Handlers                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ oauth_start │  │oauth_callback│  │ other endpoints     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                DynCommandService                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │    login    │  │link_provider│  │ generate_start_url  │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                  CommandBus                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Retries   │  │   Logging   │  │      Metrics        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Timeouts   │  │  Tracing    │  │    Validation       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                   Use Cases                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │LoginUseCase │  │LinkProvider │  │    Other UseCases   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Command Infrastructure (`application/src/command/`)

#### Core Traits and Types (`mod.rs`)
- **`Command`**: Base trait for all commands with validation and metadata
- **`CommandHandler`**: Trait for command execution logic
- **`CommandError`**: Standardized error types (Validation, Business, Infrastructure, Timeout)
- **`CommandContext`**: Execution context with tracing and metadata
- **`CommandMetrics`**: Execution metrics collection

#### CommandBus (`bus.rs`)
The central orchestrator that provides:
- **Retry Logic**: Exponential backoff with jitter
- **Timeout Handling**: Configurable execution timeouts
- **Metrics Collection**: Duration, success/failure tracking
- **Structured Logging**: Comprehensive execution logging
- **Error Classification**: Smart retry decisions based on error types

#### ErrorMapping (`error_mapping.rs`)
A centralized utility module that provides:
- **Consistent Error Mapping**: Standardized mapping from use case errors to CommandError types
- **Authentication Error Detection**: Smart classification of authentication-related errors
- **Token Service Error Handling**: Special handling for JWT validation and token service errors
- **Business Logic Mapping**: Proper categorization of business vs infrastructure errors
- **Maintainable Error Messages**: Centralized location for all error message formatting

#### DynCommandService (`service.rs`)
A service wrapper that:
- **Properly uses CommandBus**: All commands are executed through the CommandBus with full retry, logging, and metrics support
- Bridges the command pattern with existing use cases through command handlers
- Provides command validation and execution via the CommandBus
- Handles trait object compatibility with `?Sized` bounds on handlers
- Implements a clean API that maintains existing functionality while adding command pattern benefits

**Note**: The handlers use `?Sized` bounds to work with trait objects (`dyn LoginUseCase`, `dyn LinkProviderUseCase`), allowing the service to work with dynamic dispatch while still leveraging the full CommandBus functionality.

### 2. Command Implementations

#### Login Commands (`login.rs`)
- **`LoginCommand`**: OAuth login with authorization code
- **`GenerateLoginStartUrlCommand`**: Generate OAuth authorization URL
- **`LoginCommandHandler`**: Executes login logic via LoginUseCase
- **`GenerateLoginStartUrlCommandHandler`**: Generates OAuth URLs

#### Link Provider Commands (`link_provider.rs`)
- **`LinkProviderCommand`**: Link OAuth provider to existing user
- **`GenerateLinkProviderStartUrlCommand`**: Generate OAuth URL for linking
- **`LinkProviderCommandHandler`**: Executes provider linking logic
- **`GenerateLinkProviderStartUrlCommandHandler`**: Generates linking URLs

### 3. HTTP Integration

The HTTP handlers have been updated to use the command pattern instead of calling use cases directly:

```rust
// Before: Direct use case call
state.login_usecase.login(provider, code, redirect_uri).await

// After: Command pattern with context
let context = CommandContext::new()
    .with_metadata("operation".to_string(), "login_callback".to_string())
    .with_metadata("provider".to_string(), provider.as_str().to_string());

state.command_service.login(provider, code, redirect_uri, context).await
```

## Benefits

### 1. Centralized Cross-Cutting Concerns
- **Single Point of Control**: All authentication operations go through the same pipeline
- **Consistent Behavior**: Retries, logging, and metrics applied uniformly
- **Easy Maintenance**: Changes to cross-cutting concerns affect all commands automatically

### 2. Enhanced Observability
- **Structured Logging**: Every command execution is logged with context
- **Execution Metrics**: Duration, success rates, retry counts
- **Request Tracing**: Correlation IDs for distributed tracing
- **Error Analytics**: Categorized error reporting

### 3. Improved Reliability
- **Automatic Retries**: Transient failures are handled automatically
- **Circuit Breaking**: Timeout protection prevents hanging requests
- **Graceful Degradation**: Smart retry logic for different error types

### 4. Better Error Handling
- **Consistent Error Types**: Standardized error classification
- **Error Mapping**: Use case errors mapped to appropriate HTTP responses
- **Detailed Error Context**: Rich error information for debugging

## Implementation Details

### Command Validation

All commands implement validation before execution:

```rust
impl Command for LoginCommand {
    fn validate(&self) -> Result<(), CommandError> {
        if self.code.trim().is_empty() {
            return Err(CommandError::Validation(
                "Authorization code cannot be empty".to_string()
            ));
        }
        
        if !self.redirect_uri.starts_with("http://") && 
           !self.redirect_uri.starts_with("https://") {
            return Err(CommandError::Validation(
                "Redirect URI must be a valid HTTP/HTTPS URL".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### Retry Configuration

The CommandBus supports both global and command-specific retry policies:

#### Global Retry Policy (Legacy)
```rust
pub struct RetryPolicy {
    pub max_attempts: u32,           // Default: 3
    pub base_delay: Duration,        // Default: 100ms
    pub max_delay: Duration,         // Default: 30s
    pub backoff_multiplier: f64,     // Default: 2.0
    pub use_jitter: bool,           // Default: true
}
```

#### Configuration-Driven Retry Policies (Recommended)

The system now supports configuration-driven retry policies with command-specific overrides:

```rust
use configuration::{CommandRetryConfig, CommandConfig};

// Configuration structure that maps to TOML
pub struct CommandRetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,       // Milliseconds in config
    pub max_delay_ms: u64,        // Milliseconds in config
    pub backoff_multiplier: f64,
    pub use_jitter: bool,
}

pub struct CommandConfig {
    pub retry: CommandRetryConfig,                           // Default policy
    pub overrides: HashMap<String, CommandRetryConfig>,     // Command-specific overrides
}
```

#### Retry Policy Conversion

The system automatically converts configuration values to internal retry policies:

```rust
impl From<&CommandRetryConfig> for RetryPolicy {
    fn from(config: &CommandRetryConfig) -> Self {
        Self {
            max_attempts: config.max_attempts,
            base_delay: Duration::from_millis(config.base_delay_ms),    // Convert to Duration
            max_delay: Duration::from_millis(config.max_delay_ms),      // Convert to Duration
            backoff_multiplier: config.backoff_multiplier,
            use_jitter: config.use_jitter,
        }
    }
}
```

#### Command-Specific Retry Resolution

The CommandBus resolves the appropriate retry policy for each command:

```rust
impl CommandBus {
    /// Get retry policy for a specific command
    fn get_retry_policy(&self, command_type: &str) -> RetryPolicy {
        if let Some(ref command_config) = self.command_config {
            // Look for command-specific override first
            if let Some(override_config) = command_config.overrides.get(command_type) {
                return RetryPolicy::from(override_config);
            }
            // Fall back to default command config
            RetryPolicy::from(&command_config.retry)
        } else {
            // Fall back to bus default (legacy behavior)
            self.config.retry_policy.clone()
        }
    }
    
    pub async fn execute<C, H>(&self, command: C, handler: Arc<H>, context: CommandContext) 
    where C: Command + Clone, H: CommandHandler<C>
    {
        // Get command-specific retry policy
        let retry_policy = self.get_retry_policy(command.command_type());
        
        // Use the resolved policy for retry logic
        // ... retry implementation uses the specific policy
    }
}
```

#### Example: Command-Specific Behavior

Given this configuration:

```toml
[command.retry]
max_attempts = 3
base_delay_ms = 100

[command.overrides.login_command]
max_attempts = 5
base_delay_ms = 50
```

- `login_command` will retry up to 5 times with 50ms base delay
- `link_provider_command` will retry up to 3 times with 100ms base delay (uses default)
- Any other command will use the default policy

### Error Classification

Errors are classified for smart retry logic:

```rust
pub enum CommandError {
    Validation(String),      // Never retried
    Business(String),        // Never retried
    Infrastructure(String),  // Retried with backoff
    Timeout,                // Retried with backoff
    RetryExhausted(String), // Final failure after retries
}
```

## Usage Examples

### 1. OAuth Login Flow

```rust
// HTTP Handler
pub async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<OAuthResponse>, (StatusCode, Json<OAuthErrorResponse>)> {
    let provider = parse_provider(&provider_name)?;
    let code = extract_code(&query)?;
    let redirect_uri = get_redirect_uri(&state, provider);
    
    // Create command context with tracing information
    let context = CommandContext::new()
        .with_metadata("operation".to_string(), "login_callback".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());
    
    // Execute command through the command service
    let response = state.command_service
        .login(provider, code, redirect_uri, context)
        .await?;
    
    Ok(Json(OAuthResponse::Login(response.into())))
}
```

### 2. Provider Linking Flow

```rust
// HTTP Handler for linking OAuth provider to existing user
pub async fn handle_link_callback(
    state: AppState,
    provider: Provider,
    code: String,
    redirect_uri: String,
    user_id: Uuid,
) -> Result<Json<OAuthResponse>, (StatusCode, Json<OAuthErrorResponse>)> {
    // Create context with user information
    let context = CommandContext::new()
        .with_user_id(user_id)
        .with_metadata("operation".to_string(), "link_callback".to_string())
        .with_metadata("provider".to_string(), provider.as_str().to_string());
    
    // Execute link provider command
    let response = state.command_service
        .link_provider(user_id, provider, code, redirect_uri, context)
        .await?;
    
    Ok(Json(OAuthResponse::Link(response.into())))
}
```

### 3. Generating OAuth Start URLs

```rust
// Generate OAuth authorization URL
let context = CommandContext::new()
    .with_metadata("operation".to_string(), "login_start".to_string())
    .with_metadata("provider".to_string(), provider.as_str().to_string());

let auth_url = state.command_service
    .generate_login_start_url(provider, context)
    .await?;
```

## Configuration

### Command-Level Retry Configuration

The IAM service now supports configurable retry policies at both the global and command-specific levels through the application configuration system. This allows fine-grained control over retry behavior for different types of commands.

#### Configuration Structure

The retry configuration is defined in the application's TOML configuration files using the following structure:

```toml
# Command configuration
[command.retry]
# Default retry configuration for all commands
max_attempts = 3
base_delay_ms = 100
max_delay_ms = 30000
backoff_multiplier = 2.0
use_jitter = true

# Command-specific retry overrides
[command.overrides.login_command]
max_attempts = 5
base_delay_ms = 25
max_delay_ms = 2000
backoff_multiplier = 1.5
use_jitter = false

[command.overrides.link_provider_command]
max_attempts = 2
base_delay_ms = 100
max_delay_ms = 1000
backoff_multiplier = 1.0
use_jitter = false
```

#### Configuration Parameters

- **`max_attempts`**: Maximum number of retry attempts (including the initial attempt)
- **`base_delay_ms`**: Base delay between retries in milliseconds
- **`max_delay_ms`**: Maximum delay between retries in milliseconds (caps exponential backoff)
- **`backoff_multiplier`**: Multiplier for exponential backoff (default: 2.0)
- **`use_jitter`**: Whether to add random jitter to retry delays (recommended for production)

#### Environment-Specific Configuration

Different environments can have different retry configurations:

**Development (`config/development.toml`)**:
```toml
[command.retry]
# More lenient retry settings for development
max_attempts = 5
base_delay_ms = 200
max_delay_ms = 10000
backoff_multiplier = 2.0
use_jitter = true

[command.overrides.test_command]
max_attempts = 2
base_delay_ms = 100
max_delay_ms = 5000
backoff_multiplier = 1.5
use_jitter = false
```

**Production (`config/production.toml`)**:
```toml
[command.retry]
# Conservative retry settings for production
max_attempts = 3
base_delay_ms = 500
max_delay_ms = 60000  # 1 minute max delay
backoff_multiplier = 2.0
use_jitter = true

[command.overrides.critical_command]
max_attempts = 5
base_delay_ms = 1000
max_delay_ms = 30000
backoff_multiplier = 1.8
use_jitter = true
```

**Testing (`config/test.toml`)**:
```toml
[command.retry]
# Faster retry settings for tests
max_attempts = 2
base_delay_ms = 50
max_delay_ms = 5000
backoff_multiplier = 2.0
use_jitter = false  # Disable jitter for predictable test behavior
```

### CommandBus Configuration

The CommandBus now integrates with the application configuration system:

```rust
use application::command::bus::{CommandBus, CommandBusConfig};
use configuration::{CommandConfig, AppConfig};
use std::time::Duration;

// Load application configuration
let app_config = infra::config::load_config()?;

// Create CommandBus with command-specific configuration
let bus_config = CommandBusConfig {
    default_timeout: Duration::from_secs(30),
    // The retry_policy here serves as a fallback if no command config is provided
    retry_policy: RetryPolicy::default(),
    enable_metrics: true,
    enable_tracing: true,
};

let command_bus = CommandBus::with_command_config(
    bus_config,
    app_config.command,  // This contains the TOML configuration
);
```

### Retry Policy Resolution

The CommandBus resolves retry policies using the following precedence:

1. **Command-specific override**: If a command type has a specific configuration in `[command.overrides.<command_type>]`
2. **Default command configuration**: The configuration in `[command.retry]`
3. **Bus default**: The hardcoded default in `CommandBusConfig` (fallback)

```rust
// Example of how the CommandBus resolves retry policies
impl CommandBus {
    fn get_retry_policy(&self, command_type: &str) -> RetryPolicy {
        if let Some(ref command_config) = self.command_config {
            // Convert from configuration to internal retry policy
            RetryPolicy::from(command_config.get_retry_config(command_type))
        } else {
            // Fallback to bus default
            self.config.retry_policy.clone()
        }
    }
}
```

### Environment Variable Overrides

You can override configuration values using environment variables with the `IAM_` prefix:

```bash
# Override default retry attempts
IAM_COMMAND__RETRY__MAX_ATTEMPTS=5

# Override specific command configuration
IAM_COMMAND__OVERRIDES__LOGIN_COMMAND__MAX_ATTEMPTS=3
IAM_COMMAND__OVERRIDES__LOGIN_COMMAND__BASE_DELAY_MS=100
```

### Application Setup

```rust
// In setup/src/app.rs
pub async fn build_app_state(config: AppConfig) -> Result<AppState> {
    // ... create use cases ...
    
    // Create command bus with configuration-driven retry policies
    let command_bus = Arc::new(CommandBus::with_command_config(
        application::command::bus::CommandBusConfig::default(),
        config.command.clone(),  // Pass the command configuration
    ));
    
    let command_service = Arc::new(DynCommandService::new(
        command_bus,
        Arc::new(login_usecase),
        Arc::new(link_provider_usecase),
        Arc::new(token_usecase_for_commands),
        Arc::new(user_usecase_for_commands),
    ));

    // Create app state with command service
    let app_state = AppState::new(
        command_service,
        Arc::new(user_usecase),
        Arc::new(token_usecase),
        config.oauth.clone(),
    );

    Ok(app_state)
}
```

### Legacy CommandBus Configuration

For backward compatibility, you can still configure the CommandBus directly:

```rust
let config = CommandBusConfig {
    default_timeout: Duration::from_secs(30),
    retry_policy: RetryPolicy {
        max_attempts: 3,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(30),
        backoff_multiplier: 2.0,
        use_jitter: true,
    },
    enable_metrics: true,
    enable_tracing: true,
};

let command_bus = CommandBus::with_config(config);
```

However, it's recommended to use the configuration-driven approach for better maintainability and environment-specific settings.

## Monitoring and Observability

### Structured Logging

All command executions produce structured logs:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "INFO",
  "message": "Command executed successfully",
  "fields": {
    "command_type": "login",
    "duration_ms": 245,
    "retry_attempts": 0,
    "execution_id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": "123e4567-e89b-12d3-a456-426614174000",
    "operation": "login_callback",
    "provider": "github"
  }
}
```

### Metrics Collection

The system collects the following metrics:

- **Command Execution Duration**: Time taken for each command
- **Success/Failure Rates**: Success percentage by command type
- **Retry Statistics**: Number of retries per command
- **Error Distribution**: Breakdown of error types
- **Timeout Frequency**: Commands that hit timeout limits

### Error Logging

Failed commands produce detailed error logs:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "ERROR",
  "message": "Command execution failed",
  "fields": {
    "command_type": "login",
    "duration_ms": 5000,
    "retry_attempts": 3,
    "error_type": "Infrastructure(Database connection timeout)",
    "execution_id": "550e8400-e29b-41d4-a716-446655440000",
    "provider": "github"
  }
}
```

## Error Handling

### Centralized Error Mapping

All error mapping logic is centralized in the `ErrorMapping` utility module (`application/src/command/error_mapping.rs`). This provides:

- **Consistency**: All command handlers use the same error mapping logic
- **Maintainability**: Error messages and categorization are centralized
- **Testability**: Error mapping logic can be tested independently
- **Standards**: Uniform approach to error classification across all commands

```rust
use super::{Command, CommandError, CommandHandler, error_mapping::ErrorMapping};

#[async_trait]
impl<L> CommandHandler<LoginCommand> for LoginCommandHandler<L> {
    async fn handle(&self, command: LoginCommand) -> Result<LoginResponse, CommandError> {
        self.login_use_case
            .login(command.provider, command.code, command.redirect_uri)
            .await
            .map_err(ErrorMapping::map_login_error)  // Centralized mapping
    }
}
```

### Error Mapping Examples

#### Before: Inline Error Mapping (Duplicated)
```rust
// Duplicated in every command handler
.map_err(|e| match e {
    LoginError::AuthError(msg) => CommandError::Business(format!("Authentication failed: {}", msg)),
    LoginError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e)),
    LoginError::TokenError(e) => CommandError::Infrastructure(format!("Token service error: {}", e)),
})
```

#### After: Centralized Error Mapping
```rust
// Single line using centralized mapping
.map_err(ErrorMapping::map_login_error)
```

#### Error Mapping Categories
```rust
impl ErrorMapping {
    /// Map LoginError to appropriate CommandError
    pub fn map_login_error(error: LoginError) -> CommandError {
        match error {
            LoginError::AuthError(msg) => CommandError::Business(format!("Authentication failed: {}", msg)),
            LoginError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e)),
            LoginError::TokenError(e) => CommandError::Infrastructure(format!("Token service error: {}", e)),
        }
    }

    /// Map LinkProviderError with conflict handling
    pub fn map_link_provider_error(error: LinkProviderError) -> CommandError {
        match error {
            LinkProviderError::AuthError(msg) => {
                CommandError::Business(format!("Authentication failed: {}", msg))
            }
            LinkProviderError::UserNotFound => {
                CommandError::Business("User not found".to_string())
            }
            LinkProviderError::ProviderAlreadyLinked => {
                CommandError::Business("Provider account is already linked to another user".to_string())
            }
            LinkProviderError::ProviderAlreadyLinkedToSameUser => {
                CommandError::Business("Provider is already linked to your account".to_string())
            }
            // ... other mappings
        }
    }

    /// Smart token service error mapping
    pub fn map_token_service_error_to_validation(error: &dyn std::error::Error) -> CommandError {
        let error_msg = error.to_string();
        if Self::is_authentication_related_error(&error_msg) {
            CommandError::Validation(format!("Authentication failed: {}", error_msg))
        } else {
            CommandError::Infrastructure(error.to_string())
        }
    }
}
```

### HTTP Error Mapping

Command errors are then mapped to HTTP responses:

```rust
// Use case error to command error
LoginError::AuthError(msg) => CommandError::Business(format!("Authentication failed: {}", msg))
LoginError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e))
LoginError::TokenError(e) => CommandError::Infrastructure(format!("Token service error: {}", e))

// Command error to HTTP response
CommandError::Business(msg) if msg.contains("Authentication failed") => {
    (StatusCode::UNAUTHORIZED, Json(OAuthErrorResponse {
        operation: "login".to_string(),
        error: "authentication_failed".to_string(),
        message: "Authentication failed".to_string(),
    }))
}
CommandError::Validation(msg) => {
    (StatusCode::BAD_REQUEST, Json(OAuthErrorResponse {
        operation: "login".to_string(),
        error: "validation_failed".to_string(),
        message: msg,
    }))
}
```

## Technical Implementation Notes

### Trait Object Compatibility

The `DynCommandService` uses trait objects (`dyn LoginUseCase`, `dyn LinkProviderUseCase`) to provide a unified interface while working with the CommandBus. This required adding `?Sized` bounds to all command handlers:

```rust
pub struct LoginCommandHandler<L> 
where
    L: LoginUseCase + ?Sized,  // ?Sized allows trait objects
{
    login_use_case: Arc<L>,
}
```

### CommandBus Integration

The service properly routes all commands through the CommandBus:

```rust
pub async fn login(&self, ...) -> Result<LoginResponse, CommandError> {
    let command = LoginCommand::new(provider, code, redirect_uri);
    self.command_bus
        .execute(command, self.login_handler.clone(), context)  // Goes through CommandBus
        .await
}
```

This ensures that all cross-cutting concerns (retries, logging, metrics, timeouts) are applied consistently to every command execution.

## Best Practices

### 1. Command Design
- **Single Responsibility**: Each command should have one clear purpose
- **Immutable Data**: Commands should be immutable after creation
- **Rich Validation**: Validate all inputs before execution
- **Clear Naming**: Use descriptive names that indicate the action

### 2. Context Usage
- **Request Correlation**: Always include request/execution IDs
- **User Context**: Include user information when available
- **Operation Metadata**: Add relevant operation-specific data
- **Tracing Information**: Support distributed tracing

### 3. Error Handling
- **Appropriate Classification**: Choose the right error type for retry behavior
- **Rich Error Messages**: Include context and actionable information
- **Error Logging**: Log errors with sufficient detail for debugging
- **User-Friendly Messages**: Map technical errors to user-friendly responses
- **Centralized Mapping**: Use the ErrorMapping utility for consistent error handling
- **Authentication Detection**: Leverage smart classification for JWT/token errors

### 4. Performance Considerations
- **Timeout Configuration**: Set appropriate timeouts for different operations
- **Retry Limits**: Avoid excessive retries that could amplify problems
- **Metrics Collection**: Monitor performance and adjust configurations
- **Resource Management**: Ensure proper cleanup of resources

### 5. Testing
- **Unit Tests**: Test command validation and error mapping
- **Integration Tests**: Test the full command execution flow
- **Retry Testing**: Verify retry behavior with simulated failures
- **Performance Tests**: Validate timeout and retry configurations

### 6. Configuration Management
- **Environment-Specific Settings**: Use different retry configurations for dev/test/prod
- **Command-Specific Tuning**: Override retry behavior for critical or sensitive operations
- **Conservative Production Settings**: Use longer delays and fewer retries in production
- **Disable Jitter in Tests**: Set `use_jitter = false` for predictable test behavior
- **Monitor and Adjust**: Use metrics to fine-tune retry configurations
- **Document Overrides**: Clearly document why specific commands have custom retry settings
- **Environment Variables**: Use environment variables for runtime configuration adjustments

## Future Enhancements

### Potential Improvements

1. **Circuit Breaker Pattern**: Add circuit breakers for external service calls
2. **Rate Limiting**: Implement rate limiting at the command level
3. **Async Processing**: Support for asynchronous command execution
4. **Command Queuing**: Queue commands for batch processing
5. **Saga Pattern**: Support for distributed transactions
6. **Command Versioning**: Handle command schema evolution
7. **Audit Trail**: Complete audit logging for compliance
8. **Performance Optimization**: Command execution optimization

### Integration Opportunities

1. **Metrics Systems**: Integration with Prometheus/Grafana
2. **Distributed Tracing**: Integration with Jaeger/Zipkin
3. **Log Aggregation**: Integration with ELK stack
4. **Alerting**: Integration with alerting systems
5. **Dashboard**: Real-time command execution monitoring

## Conclusion

The Command Design Pattern implementation provides a robust foundation for handling authentication operations with comprehensive cross-cutting concerns. It improves system reliability, observability, and maintainability while providing a consistent interface for all authentication-related operations.

The pattern is extensible and can be applied to other areas of the system as needed, providing a standardized approach to command execution throughout the application. 