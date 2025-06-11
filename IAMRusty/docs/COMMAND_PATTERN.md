# Command Design Pattern Implementation

## Overview

This document describes the implementation of the Command Design Pattern in the IAM service, which provides centralized handling of cross-cutting concerns including retries, logging, metrics, and tracing for authentication operations. The system has evolved from a static command service to a fully extensible registry-based architecture that supports auto-registration, pluggable error mapping, and easy SDK extraction.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [New Extensible System](#new-extensible-system)
- [Key Components](#key-components)
- [Migration from Legacy System](#migration-from-legacy-system)
- [Adding New Commands](#adding-new-commands)
- [Command Registry](#command-registry)
- [Error Mapping](#error-mapping)
- [Configuration](#configuration)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Legacy Documentation](#legacy-documentation)

## Architecture Overview

The new extensible command system follows a registry-based architecture that provides automatic command registration, full type safety, and comprehensive cross-cutting concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Handlers                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ oauth_start │  │oauth_callback│  │ other endpoints     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│               GenericCommandService                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ execute<C>  │  │list_commands│  │ supports_command    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                  CommandRegistry                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Retries   │  │   Logging   │  │      Metrics        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Timeouts   │  │  Tracing    │  │    Validation       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Type Safety │  │Error Mapping│  │   Auto-Registration │  │
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

The extensible command system consists of four main components:

```
┌─────────────────────┐
│ GenericCommandService│
│                     │
│ - execute<C>()      │
│ - list_commands()   │
│ - supports_command()│
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  CommandRegistry    │
│                     │
│ - Type-erased       │
│   command handlers  │
│ - Error mappers     │
│ - Command metadata  │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ CommandRegistryBuilder│
│                     │
│ - Builder pattern   │
│ - Fluent API        │
│ - Type safety       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│CommandRegistryFactory│
│                     │
│ - Pre-configured    │
│   registries        │
│ - IAM commands      │
│ - Custom builders   │
└─────────────────────┘
```

## New Extensible System

### Key Benefits

✅ **Auto-registration**: Commands register themselves without manual updates  
✅ **Type Safety**: Full compile-time checking maintained  
✅ **Pluggable Error Mapping**: Each command can have custom error handling  
✅ **Modular**: Can build registries with only needed commands  
✅ **SDK-Friendly**: Easy to extract into separate crates  
✅ **All Cross-Cutting Concerns**: Retry logic, timeouts, metrics, logging preserved  
✅ **Backward Compatible**: Existing functionality maintained

### What's New vs Legacy System

| Feature | Legacy System | New Extensible System |
|---------|---------------|----------------------|
| Command Registration | Manual updates to multiple files | Auto-registration via builder |
| Error Mapping | Static centralized mapping | Pluggable per-command mappers |
| Type Safety | ✅ Full | ✅ Full (maintained) |
| Cross-Cutting Concerns | ✅ All supported | ✅ All supported (enhanced) |
| SDK Extraction | ❌ Difficult | ✅ Easy |
| Adding Commands | Manual updates required | Only command definition needed |
| Introspection | Limited | ✅ List/check available commands |

## Key Components

### 1. GenericCommandService

The main service interface that replaces `DynCommandService`. It provides a type-safe way to execute any registered command.

```rust
use application::command::{GenericCommandService, CommandContext};

// Execute any registered command
let result = service.execute(command, context).await?;

// List available commands
let commands = service.list_available_commands();

// Check if a command is supported
if service.supports_command("login") {
    // Command is registered and can be executed
}
```

**Key features:**
- **Type Safety**: Full compile-time type checking maintained
- **Generic Execution**: Works with any command implementing `Command` trait
- **Runtime Introspection**: Can list and check available commands
- **Thread Safe**: Cloneable and safe for concurrent use

### 2. CommandRegistry

The core registry that stores command handlers with type erasure while maintaining type safety. Handles all cross-cutting concerns that were previously in CommandBus.

```rust
// Registry stores handlers with type erasure but maintains type safety
let registry = CommandRegistry::new();
registry.execute_command(command, context).await?;
```

**Cross-cutting concerns provided:**
- **Retry Logic**: Exponential backoff with jitter
- **Timeout Handling**: Configurable execution timeouts
- **Metrics Collection**: Duration, success/failure tracking
- **Structured Logging**: Comprehensive execution logging
- **Error Classification**: Smart retry decisions based error types
- **Type Erasure**: Stores different command types in unified way
- **Handler Management**: Maps command types to their handlers
- **Error Mapping**: Associates error mappers with commands

### 3. CommandRegistryBuilder

A builder for constructing command registries with a fluent API.

```rust
use application::command::{CommandRegistryBuilder, CommandErrorMapper};

let registry = CommandRegistryBuilder::new()
    .register::<MyCommand, _>(
        "my_command".to_string(),
        Arc::new(MyCommandHandler::new()),
        Arc::new(MyErrorMapper::new())
    )
    .register::<AnotherCommand, _>(
        "another_command".to_string(),
        Arc::new(AnotherCommandHandler::new()),
        Arc::new(AnotherErrorMapper::new())
    )
    .build();
```

**Key features:**
- **Fluent API**: Chain multiple registrations
- **Type Safety**: Ensures handler matches command type
- **Error Mapping**: Associates error mappers with commands
- **Validation**: Prevents duplicate registrations

### 4. CommandRegistryFactory

Pre-configured factory methods for common command sets.

```rust
use application::command::CommandRegistryFactory;

// Create registry with all IAM commands
let registry = CommandRegistryFactory::create_iam_registry(
    login_usecase,
    link_provider_usecase,
    token_usecase,
    user_usecase,
    auth_usecase
);

// Create registry with only user commands
let user_registry = CommandRegistryFactory::create_user_registry(user_usecase);
```

**Available factory methods:**
- `create_iam_registry()` - All IAM commands
- `create_login_registry()` - OAuth login commands
- `create_link_provider_registry()` - Provider linking commands
- `create_token_registry()` - Token management commands
- `create_user_registry()` - User management commands
- `create_auth_registry()` - Authentication commands

## Migration from Legacy System

### Before (DynCommandService)

```rust
// Manual method calls on service
let response = service.login(provider, code, redirect_uri, context).await?;
let user = service.get_user(user_id, context).await?;
let token = service.refresh_token(refresh_token, context).await?;

// Adding new commands required updating:
// 1. DynCommandService trait
// 2. DynCommandService implementation
// 3. Error mapping enum
// 4. Handler registration
```

### After (GenericCommandService)

```rust
// Command pattern with type safety
let login_cmd = LoginCommand::new(provider, code, redirect_uri);
let response = service.execute(login_cmd, context).await?;

let user_cmd = GetUserCommand::new(user_id);
let user = service.execute(user_cmd, context).await?;

let token_cmd = RefreshTokenCommand::new(refresh_token);
let token = service.execute(token_cmd, context).await?;

// Adding new commands only requires:
// 1. Command struct implementing Command trait
// 2. Handler implementing CommandHandler
// 3. Error mapper implementing CommandErrorMapper
// 4. Registration in factory (optional)
```

### Migration Steps

1. **Update service creation**:
```rust
// Old
let service = Arc::new(DynCommandService::new(/*...*/));

// New
let registry = CommandRegistryFactory::create_iam_registry(/*...*/);
let service = Arc::new(GenericCommandService::new(Arc::new(registry)));
```

2. **Update command calls**:
```rust
// Old
service.login(provider, code, uri, ctx).await?

// New
let cmd = LoginCommand::new(provider, code, uri);
service.execute(cmd, ctx).await?
```

3. **Update error handling** (already compatible with `CommandError`)

The new system is designed to be backward compatible, allowing gradual migration from the old system. The old CommandBus/DynCommandService is still available and can be used alongside the new system.

## Adding New Commands

### Step 1: Create Command Struct

```rust
use uuid::Uuid;
use async_trait::async_trait;
use application::command::{Command, CommandError};

#[derive(Debug, Clone)]
pub struct MyCustomCommand {
    id: Uuid,
    data: String,
}

impl MyCustomCommand {
    pub fn new(data: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            data,
        }
    }
}

#[async_trait]
impl Command for MyCustomCommand {
    type Result = String;

    fn command_type(&self) -> &'static str {
        "my_custom_command"
    }

    fn command_id(&self) -> Uuid {
        self.id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.data.is_empty() {
            Err(CommandError::Validation("Data cannot be empty".to_string()))
        } else {
            Ok(())
        }
    }
}
```

### Step 2: Create Command Handler

```rust
use async_trait::async_trait;
use application::command::{CommandHandler, CommandError};

pub struct MyCustomCommandHandler {
    // Dependencies (repositories, services, etc.)
}

impl MyCustomCommandHandler {
    pub fn new(/* dependencies */) -> Self {
        Self { /* ... */ }
    }
}

#[async_trait]
impl CommandHandler<MyCustomCommand> for MyCustomCommandHandler {
    async fn handle(&self, command: MyCustomCommand) -> Result<String, CommandError> {
        // Command logic here
        Ok(format!("Processed: {}", command.data))
    }
}
```

### Step 3: Create Error Mapper

```rust
use application::command::{CommandErrorMapper, CommandError};

pub struct MyCustomErrorMapper;

impl CommandErrorMapper for MyCustomErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        // Map domain/infrastructure errors to CommandError
        if let Some(domain_error) = error.downcast_ref::<MyDomainError>() {
            match domain_error {
                MyDomainError::NotFound => CommandError::Business("Resource not found".to_string()),
                MyDomainError::InvalidInput => CommandError::Validation("Invalid input".to_string()),
                _ => CommandError::Infrastructure(error.to_string()),
            }
        } else {
            CommandError::Infrastructure(error.to_string())
        }
    }
}
```

### Step 4: Register Command

```rust
// Option A: Using builder directly
let registry = CommandRegistryBuilder::new()
    .register::<MyCustomCommand, _>(
        "my_custom_command".to_string(),
        Arc::new(MyCustomCommandHandler::new()),
        Arc::new(MyCustomErrorMapper)
    )
    .build();

// Option B: Extend factory (recommended for reusable commands)
impl CommandRegistryFactory {
    pub fn create_custom_registry() -> CommandRegistry {
        Self::new()
            .register::<MyCustomCommand, _>(
                "my_custom_command".to_string(),
                Arc::new(MyCustomCommandHandler::new()),
                Arc::new(MyCustomErrorMapper)
            )
            .build()
    }
}
```

### Step 5: Use Command

```rust
let command = MyCustomCommand::new("Hello World".to_string());
let result = service.execute(command, context).await?;
```

Adding new commands is as simple as defining the command and its handler. The new system handles the rest, including auto-registration, error mapping, and cross-cutting concerns.

## Command Registry

### Registry Capabilities

```rust
// Create empty registry
let registry = CommandRegistry::new();

// Execute commands (type-safe)
let result = registry.execute_command(command, context).await?;

// List available command types
let types = registry.list_command_types();

// Check if command type is supported
if registry.get_handler("my_command").is_some() {
    // Command is registered
}
```

### Thread Safety

All registry components are thread-safe and can be shared across threads:

```rust
let registry = Arc::new(registry);
let service = Arc::new(GenericCommandService::new(registry));

// Safe to clone and use in multiple threads
let service_clone = service.clone();
tokio::spawn(async move {
    let result = service_clone.execute(command, context).await;
});
```

The CommandRegistry stores command handlers with type erasure while maintaining type safety. It handles all cross-cutting concerns that were previously in CommandBus.

## Error Mapping

### CommandErrorMapper Trait

Each command can have its own error mapping strategy:

```rust
pub trait CommandErrorMapper: Send + Sync {
    fn map_error(&self, error: Box<dyn Error + Send + Sync>) -> CommandError;
}
```

### Built-in Error Mappers

The system provides error mappers for all existing domains:

- `AuthErrorMapper` - Authentication domain errors
- `LoginErrorMapper` - Login-specific errors  
- `LinkProviderErrorMapper` - Provider linking errors
- `TokenErrorMapper` - Token management errors
- `UserErrorMapper` - User management errors

### Custom Error Mapping

```rust
pub struct CustomErrorMapper;

impl CommandErrorMapper for CustomErrorMapper {
    fn map_error(&self, error: Box<dyn Error + Send + Sync>) -> CommandError {
        // Type-safe error downcasting
        if let Some(domain_err) = error.downcast_ref::<DomainError>() {
            match domain_err {
                DomainError::ValidationFailed(msg) => 
                    CommandError::Validation(msg.clone()),
                DomainError::EntityNotFound => 
                    CommandError::Business("Entity not found".to_string()),
                DomainError::BusinessRuleViolation(msg) => 
                    CommandError::Business(msg.clone()),
            }
        } else if let Some(infra_err) = error.downcast_ref::<InfraError>() {
            CommandError::Infrastructure(infra_err.to_string())
        } else {
            CommandError::Infrastructure(error.to_string())
        }
    }
}
```

Each command can have custom error handling. The new system provides a pluggable error mapping mechanism that allows each command to define its own error handling logic.

## Configuration

The IAM service now supports configurable retry policies at both the global and command-specific levels through the application configuration system. This allows fine-grained control over retry behavior for different types of commands.

## Best Practices

### 1. Command Design

**Do:**
- ✅ Make commands immutable (use Clone, not mutable references)
- ✅ Include all necessary data in the command struct
- ✅ Implement comprehensive validation in `validate()`
- ✅ Use descriptive command type names

**Don't:**
- ❌ Include business logic in command structs
- ❌ Make commands stateful or mutable
- ❌ Skip validation (always implement it)
- ❌ Use generic command type names

### 2. Handler Design

**Do:**
- ✅ Keep handlers focused on a single command type
- ✅ Inject dependencies through constructor
- ✅ Handle all possible error scenarios
- ✅ Use appropriate error types for different failures

**Don't:**
- ❌ Make handlers handle multiple command types
- ❌ Access external dependencies directly
- ❌ Ignore error handling
- ❌ Mix presentation logic with business logic

### 3. Error Mapping

**Do:**
- ✅ Map domain errors to appropriate CommandError variants
- ✅ Preserve original error information when possible
- ✅ Use specific error messages for validation errors
- ✅ Use downcast for type-safe error handling

**Don't:**
- ❌ Lose error context during mapping
- ❌ Use generic error messages
- ❌ Panic or unwrap in error mappers
- ❌ Ignore error types (always handle all cases)

### 4. Registry Management

**Do:**
- ✅ Use factory methods for common command sets
- ✅ Register commands at application startup
- ✅ Use descriptive command type identifiers
- ✅ Test command registration and execution

**Don't:**
- ❌ Register commands dynamically at runtime
- ❌ Use duplicate command type names
- ❌ Register commands without error mappers
- ❌ Skip testing command integration

### 5. Context Usage
- **Request Correlation**: Always include request/execution IDs
- **User Context**: Include user information when available
- **Operation Metadata**: Add relevant operation-specific data
- **Tracing Information**: Support distributed tracing

### 6. Performance Considerations
- **Timeout Configuration**: Set appropriate timeouts for different operations
- **Retry Limits**: Avoid excessive retries that could amplify problems
- **Metrics Collection**: Monitor performance and adjust configurations
- **Resource Management**: Ensure proper cleanup of resources

### 7. Testing
- **Unit Tests**: Test command validation and error mapping
- **Integration Tests**: Test the full command execution flow
- **Retry Testing**: Verify retry behavior with simulated failures
- **Performance Tests**: Validate timeout and retry configurations

### 8. Configuration Management
- **Environment-Specific Settings**: Use different retry configurations for dev/test/prod
- **Command-Specific Tuning**: Override retry behavior for critical or sensitive operations
- **Conservative Production Settings**: Use longer delays and fewer retries in production
- **Disable Jitter in Tests**: Set `use_jitter = false` for predictable test behavior
- **Monitor and Adjust**: Use metrics to fine-tune retry configurations
- **Document Overrides**: Clearly document why specific commands have custom retry settings
- **Environment Variables**: Use environment variables for runtime configuration adjustments

## Examples

### Complete Command Implementation

```rust
// 1. Command Definition
#[derive(Debug, Clone)]
pub struct SendEmailCommand {
    id: Uuid,
    to: String,
    subject: String,
    body: String,
}

impl SendEmailCommand {
    pub fn new(to: String, subject: String, body: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            to,
            subject,
            body,
        }
    }
}

#[async_trait]
impl Command for SendEmailCommand {
    type Result = EmailDeliveryReceipt;

    fn command_type(&self) -> &'static str {
        "send_email"
    }

    fn command_id(&self) -> Uuid {
        self.id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.to.is_empty() {
            return Err(CommandError::Validation("Recipient email is required".to_string()));
        }
        if !self.to.contains('@') {
            return Err(CommandError::Validation("Invalid email format".to_string()));
        }
        if self.subject.is_empty() {
            return Err(CommandError::Validation("Subject is required".to_string()));
        }
        Ok(())
    }
}

// 2. Handler Implementation
pub struct SendEmailHandler {
    email_service: Arc<dyn EmailService>,
}

impl SendEmailHandler {
    pub fn new(email_service: Arc<dyn EmailService>) -> Self {
        Self { email_service }
    }
}

#[async_trait]
impl CommandHandler<SendEmailCommand> for SendEmailHandler {
    async fn handle(&self, command: SendEmailCommand) -> Result<EmailDeliveryReceipt, CommandError> {
        let email = Email {
            to: command.to,
            subject: command.subject,
            body: command.body,
        };

        self.email_service
            .send(email)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
            .map_err(|e| EmailErrorMapper.map_error(e))
    }
}

// 3. Error Mapper
pub struct EmailErrorMapper;

impl CommandErrorMapper for EmailErrorMapper {
    fn map_error(&self, error: Box<dyn Error + Send + Sync>) -> CommandError {
        if let Some(email_error) = error.downcast_ref::<EmailServiceError>() {
            match email_error {
                EmailServiceError::InvalidRecipient => 
                    CommandError::Validation("Invalid recipient email".to_string()),
                EmailServiceError::RateLimited => 
                    CommandError::Business("Email rate limit exceeded".to_string()),
                EmailServiceError::ServiceUnavailable => 
                    CommandError::Infrastructure("Email service unavailable".to_string()),
                _ => CommandError::Infrastructure(email_error.to_string()),
            }
        } else {
            CommandError::Infrastructure(error.to_string())
        }
    }
}

// 4. Registration and Usage
let registry = CommandRegistryBuilder::new()
    .register::<SendEmailCommand, _>(
        "send_email".to_string(),
        Arc::new(SendEmailHandler::new(email_service)),
        Arc::new(EmailErrorMapper)
    )
    .build();

let service = GenericCommandService::new(Arc::new(registry));

// Execute command
let command = SendEmailCommand::new(
    "user@example.com".to_string(),
    "Welcome!".to_string(),
    "Welcome to our service!".to_string()
);

let receipt = service.execute(command, context).await?;
println!("Email sent with ID: {}", receipt.id);
```

### SDK Integration Example

The extensible command system makes it easy to extract functionality into external SDKs:

```rust
// External SDK crate
pub struct IAMClient {
    service: GenericCommandService,
}

impl IAMClient {
    pub fn new(registry: CommandRegistry) -> Self {
        Self {
            service: GenericCommandService::new(Arc::new(registry)),
        }
    }

    // High-level API methods
    pub async fn login_with_github(&self, code: String) -> Result<LoginResponse, ClientError> {
        let command = LoginCommand::new(Provider::GitHub, code, self.redirect_uri.clone());
        let context = CommandContext::new();
        
        self.service
            .execute(command, context)
            .await
            .map_err(ClientError::from)
    }

    pub async fn get_user_profile(&self, user_id: Uuid) -> Result<User, ClientError> {
        let command = GetUserCommand::new(user_id);
        let context = CommandContext::new().with_user_id(user_id);
        
        self.service
            .execute(command, context)
            .await
            .map_err(ClientError::from)
    }
}

// SDK users can create clients with custom command sets
let registry = CommandRegistryFactory::create_iam_registry(/* dependencies */);
let client = IAMClient::new(registry);
```

This system provides the flexibility you requested: adding new commands no longer requires updating multiple files, commands can be easily organized into different registries for different use cases, and the system can be extracted into SDKs while maintaining full type safety and proper error handling.

## Legacy Documentation

The old CommandBus/DynCommandService documentation is still available in the repository. 