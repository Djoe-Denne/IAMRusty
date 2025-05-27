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

The CommandBus supports configurable retry policies:

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,           // Default: 3
    pub base_delay: Duration,        // Default: 100ms
    pub max_delay: Duration,         // Default: 30s
    pub backoff_multiplier: f64,     // Default: 2.0
    pub use_jitter: bool,           // Default: true
}
```

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

### CommandBus Configuration

```rust
use application::command::bus::{CommandBus, CommandBusConfig, RetryPolicy};
use std::time::Duration;

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

### Application Setup

```rust
// In setup/src/app.rs
pub async fn build_app_state(config: AppConfig) -> Result<AppState> {
    // ... create use cases ...
    
    // Create command bus and service
    let command_bus = Arc::new(CommandBus::new());
    let command_service = Arc::new(DynCommandService::new(
        command_bus,
        Arc::new(login_usecase),
        Arc::new(link_provider_usecase),
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

### Error Flow

1. **Use Case Error**: Original error from business logic
2. **Error Mapping**: Converted to appropriate CommandError type
3. **Retry Decision**: CommandBus decides whether to retry
4. **HTTP Mapping**: CommandError mapped to HTTP status code
5. **Client Response**: Standardized error response to client

### Error Mapping Examples

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