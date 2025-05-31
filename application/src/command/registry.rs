use super::{Command, CommandError, CommandHandler, CommandContext, CommandMetrics};
use async_trait::async_trait;
use inventory;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use tokio::time::timeout;

/// Trait for dependency container that provides command handler dependencies
pub trait DependencyContainer: Send + Sync + std::fmt::Debug {
    /// Get a dependency by type name and convert it to the expected type
    fn get_dependency(&self, type_name: &str) -> Option<Box<dyn Any + Send>>;
}

/// Simple dependency container implementation for use cases
#[derive(Debug)]
pub struct SimpleDependencyContainer {
    dependencies: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl SimpleDependencyContainer {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }
    
    pub fn with_dependency<T: Send + Sync + 'static>(
        mut self,
        name: &str,
        dependency: Arc<T>,
    ) -> Self {
        self.dependencies.insert(name.to_string(), dependency);
        self
    }
}

impl DependencyContainer for SimpleDependencyContainer {
    fn get_dependency(&self, type_name: &str) -> Option<Box<dyn Any + Send>> {
        self.dependencies.get(type_name).map(|dep| {
            // Clone the Arc and return it as a boxed Any
            let cloned_arc = dep.clone();
            Box::new(cloned_arc) as Box<dyn Any + Send>
        })
    }
}

/// One entry per command: its type string, a factory for handler & its error-mapper
pub struct CommandRegistration {
    pub command_name: &'static str,
    pub handler_factory: fn(&dyn DependencyContainer) -> Arc<dyn DynCommandHandler>,
    pub error_mapper_factory: fn() -> Arc<dyn CommandErrorMapper>,
}

// Tell `inventory` to gather all `CommandRegistration` instances
inventory::collect!(CommandRegistration);

/// Build the registry by iterating all registered commands
pub fn build_registry_from_inventory(container: &dyn DependencyContainer) -> CommandRegistry {
    let mut builder = CommandRegistryBuilder::new();
    for reg in inventory::iter::<CommandRegistration> {
        // We need to call the handler factory and pass it to register_raw
        builder = builder.register_raw(
            reg.command_name.to_string(),
            (reg.handler_factory)(container),
        );
    }
    builder.build()
}

/// Build the registry with custom config by iterating all registered commands
pub fn build_registry_from_inventory_with_config(
    config: RegistryConfig,
    container: &dyn DependencyContainer,
) -> CommandRegistry {
    let mut builder = CommandRegistryBuilder::with_config(config);
    for reg in inventory::iter::<CommandRegistration> {
        builder = builder.register_raw(
            reg.command_name.to_string(),
            (reg.handler_factory)(container),
        );
    }
    builder.build()
}

/// Build the registry with custom config and metrics by iterating all registered commands
pub fn build_registry_from_inventory_with_config_and_metrics(
    config: RegistryConfig,
    metrics_collector: Arc<dyn MetricsCollector>,
    container: &dyn DependencyContainer,
) -> CommandRegistry {
    let mut builder = CommandRegistryBuilder::with_config_and_metrics(config, metrics_collector);
    for reg in inventory::iter::<CommandRegistration> {
        builder = builder.register_raw(
            reg.command_name.to_string(),
            (reg.handler_factory)(container),
        );
    }
    builder.build()
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay_ms = self.base_delay.as_millis() as f64;
        let exponential_delay = base_delay_ms * self.backoff_multiplier.powi(attempt as i32);
        
        let mut delay = Duration::from_millis(exponential_delay as u64);
        if delay > self.max_delay {
            delay = self.max_delay;
        }
        
        if self.use_jitter {
            // Simple jitter using system time - not cryptographically secure but sufficient for retry jitter
            let time_nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as f64;
            let jitter = (time_nanos / 1_000_000_000.0 - 0.5) * 0.1; // ±5% jitter
            let jitter_factor = 1.0 + jitter;
            delay = Duration::from_millis((delay.as_millis() as f64 * jitter_factor) as u64);
        }
        
        delay
    }
    
    pub fn is_retryable(&self, error: &CommandError) -> bool {
        matches!(error, CommandError::Infrastructure(_) | CommandError::Timeout)
    }
}

/// Registry configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub default_timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub enable_metrics: bool,
    pub enable_tracing: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            enable_metrics: true,
            enable_tracing: true,
        }
    }
}

/// Trait for collecting metrics
#[async_trait]
pub trait MetricsCollector: Send + Sync {
    async fn record_metrics(&self, metrics: CommandMetrics);
}

/// Simple logging metrics collector
pub struct LoggingMetricsCollector;

#[async_trait]
impl MetricsCollector for LoggingMetricsCollector {
    async fn record_metrics(&self, metrics: CommandMetrics) {
        info!(
            command_type = %metrics.command_type,
            duration_ms = metrics.duration_ms,
            success = metrics.success,
            retry_attempts = metrics.retry_attempts,
            error_type = ?metrics.error_type,
            "Command metrics recorded"
        );
    }
}

/// Trait for error mapping that command handlers can implement
pub trait CommandErrorMapper: Send + Sync {
    /// Map a domain error to CommandError
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError;
}

/// Type-erased command handler that can be stored in the registry
#[async_trait]
pub trait DynCommandHandler: Send + Sync {
    /// Execute the command with type-erased parameters
    async fn execute_dyn(
        &self,
        command: Box<dyn Any + Send>,
        _context: CommandContext,
    ) -> Result<Box<dyn Any + Send>, CommandError>;
    
    /// Get the command type this handler supports
    fn command_type(&self) -> &'static str;
    
    /// Get the error mapper for this handler
    fn error_mapper(&self) -> Arc<dyn CommandErrorMapper>;
}

/// Wrapper that implements DynCommandHandler for concrete command handlers
pub struct CommandHandlerWrapper<C, H> 
where
    C: Command + 'static,
    H: CommandHandler<C>,
{
    handler: Arc<H>,
    error_mapper: Arc<dyn CommandErrorMapper>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C, H> CommandHandlerWrapper<C, H>
where
    C: Command + 'static,
    H: CommandHandler<C>,
{
    pub fn new(handler: Arc<H>, error_mapper: Arc<dyn CommandErrorMapper>) -> Self {
        Self {
            handler,
            error_mapper,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<C, H> DynCommandHandler for CommandHandlerWrapper<C, H>
where
    C: Command + 'static,
    H: CommandHandler<C>,
{
    async fn execute_dyn(
        &self,
        command: Box<dyn Any + Send>,
        _context: CommandContext,
    ) -> Result<Box<dyn Any + Send>, CommandError> {
        // Downcast the command to the expected type
        let command = command
            .downcast::<C>()
            .map_err(|_| CommandError::Infrastructure("Invalid command type".to_string()))?;
        
        // Execute the command and handle errors
        match self.handler.handle(*command).await {
            Ok(result) => Ok(Box::new(result)),
            Err(command_error) => {
                // If it's already a CommandError, just pass it through
                // Otherwise, this shouldn't happen since handlers should return CommandError
                Err(command_error)
            }
        }
    }
    
    fn command_type(&self) -> &'static str {
        // We need to get this from a sample command, but we can't create one here
        // This will be set when registering
        std::any::type_name::<C>()
    }
    
    fn error_mapper(&self) -> Arc<dyn CommandErrorMapper> {
        self.error_mapper.clone()
    }
}

/// Command registry that stores all command handlers
pub struct CommandRegistry {
    handlers: HashMap<String, Arc<dyn DynCommandHandler>>,
    config: RegistryConfig,
    metrics_collector: Arc<dyn MetricsCollector>,
}

impl CommandRegistry {
    /// Create a new command registry with default configuration
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            config: RegistryConfig::default(),
            metrics_collector: Arc::new(LoggingMetricsCollector),
        }
    }
    
    /// Create a new command registry with custom configuration
    pub fn with_config(config: RegistryConfig) -> Self {
        Self {
            handlers: HashMap::new(),
            config,
            metrics_collector: Arc::new(LoggingMetricsCollector),
        }
    }
    
    /// Create a new command registry with custom configuration and metrics collector
    pub fn with_config_and_metrics(
        config: RegistryConfig,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            handlers: HashMap::new(),
            config,
            metrics_collector,
        }
    }
    
    /// Register a command handler with its error mapper
    pub fn register<C, H>(
        &mut self,
        command_type: String,
        handler: Arc<H>,
        error_mapper: Arc<dyn CommandErrorMapper>,
    ) where
        C: Command + 'static,
        H: CommandHandler<C> + 'static,
    {
        let wrapper = Arc::new(CommandHandlerWrapper::new(handler, error_mapper));
        self.handlers.insert(command_type, wrapper);
    }
    
    /// Register a command handler directly without wrapping (for inventory system)
    pub fn register_raw(
        &mut self,
        command_type: String,
        handler: Arc<dyn DynCommandHandler>,
    ) {
        self.handlers.insert(command_type, handler);
    }
    
    /// Get a handler for a command type
    pub fn get_handler(&self, command_type: &str) -> Option<Arc<dyn DynCommandHandler>> {
        self.handlers.get(command_type).cloned()
    }
    
    /// Execute a command through the registry with full cross-cutting concerns
    pub async fn execute_command<C: Command + Clone + 'static>(
        &self,
        command: C,
        context: CommandContext,
    ) -> Result<C::Result, CommandError> {
        let command_type = command.command_type();
        let start_time = Instant::now();
        let mut retry_attempts = 0;
        
        if self.config.enable_tracing {
            info!(
                command_type = %command_type,
                command_id = %command.command_id(),
                execution_id = %context.execution_id,
                "Starting command execution"
            );
        }
        
        // Validate command first
        if let Err(validation_error) = command.validate() {
            if self.config.enable_tracing {
                error!(
                    command_type = %command_type,
                    error = %validation_error,
                    "Command validation failed"
                );
            }
            return Err(validation_error);
        }
        
        // Get handler
        let handler = self.get_handler(command_type)
            .ok_or_else(|| CommandError::Infrastructure(
                format!("No handler registered for command type: {}", command_type)
            ))?;
        
        let retry_policy = &self.config.retry_policy;
        
        // Retry loop
        loop {
            let execution_future = self.execute_once(handler.clone(), command.clone(), context.clone());
            let execution_result = timeout(self.config.default_timeout, execution_future).await;
            
            match execution_result {
                Ok(Ok(result)) => {
                    // Success
                    let duration = start_time.elapsed();
                    if self.config.enable_tracing {
                        info!(
                            command_type = %command_type,
                            duration_ms = duration.as_millis() as u64,
                            retry_attempts = retry_attempts,
                            "Command executed successfully"
                        );
                    }
                    
                    if self.config.enable_metrics {
                        self.record_success_metrics(&command, duration, retry_attempts).await;
                    }
                    
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    // Command failed
                    retry_attempts += 1;
                    
                    // Check if error is retryable
                    if !retry_policy.is_retryable(&e) {
                        let duration = start_time.elapsed();
                        if self.config.enable_tracing {
                            error!(
                                command_type = %command_type,
                                error = %e,
                                duration_ms = duration.as_millis() as u64,
                                retry_attempts = retry_attempts,
                                "Command execution failed - error not retryable"
                            );
                        }
                        
                        if self.config.enable_metrics {
                            self.record_failure_metrics(&command, start_time, retry_attempts, &e).await;
                        }
                        
                        return Err(e);
                    }
                    
                    // Check if we've reached max attempts
                    if retry_attempts >= retry_policy.max_attempts {
                        let duration = start_time.elapsed();
                        if self.config.enable_tracing {
                            error!(
                                command_type = %command_type,
                                error = %e,
                                duration_ms = duration.as_millis() as u64,
                                retry_attempts = retry_attempts,
                                "Command execution failed after maximum retries"
                            );
                        }
                        
                        if self.config.enable_metrics {
                            self.record_failure_metrics(&command, start_time, retry_attempts, &e).await;
                        }
                        
                        return Err(CommandError::RetryExhausted(e.to_string()));
                    }
                    
                    // Calculate delay and retry
                    let delay = retry_policy.calculate_delay(retry_attempts - 1);
                    if self.config.enable_tracing {
                        warn!(
                            command_type = %command_type,
                            error = %e,
                            retry_attempt = retry_attempts,
                            delay_ms = delay.as_millis() as u64,
                            "Command failed, retrying"
                        );
                    }
                    
                    tokio::time::sleep(delay).await;
                }
                Err(_) => {
                    // Timeout
                    retry_attempts += 1;
                    let timeout_error = CommandError::Timeout;
                    
                    if retry_attempts >= retry_policy.max_attempts {
                        let duration = start_time.elapsed();
                        if self.config.enable_tracing {
                            error!(
                                command_type = %command_type,
                                duration_ms = duration.as_millis() as u64,
                                retry_attempts = retry_attempts,
                                "Command execution timed out after retries"
                            );
                        }
                        
                        if self.config.enable_metrics {
                            self.record_failure_metrics(&command, start_time, retry_attempts, &timeout_error).await;
                        }
                        
                        return Err(CommandError::RetryExhausted("Timeout".to_string()));
                    }
                    
                    let delay = retry_policy.calculate_delay(retry_attempts - 1);
                    if self.config.enable_tracing {
                        warn!(
                            command_type = %command_type,
                            retry_attempt = retry_attempts,
                            delay_ms = delay.as_millis() as u64,
                            "Command timed out, retrying"
                        );
                    }
                    
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    /// Execute command once without retry logic
    async fn execute_once<C: Command + 'static>(
        &self,
        handler: Arc<dyn DynCommandHandler>,
        command: C,
        context: CommandContext,
    ) -> Result<C::Result, CommandError> {
        let result = handler.execute_dyn(Box::new(command), context).await?;
        
        // Downcast result back to expected type
        result
            .downcast::<C::Result>()
            .map(|boxed| *boxed)
            .map_err(|_| CommandError::Infrastructure("Invalid result type".to_string()))
    }
    
    async fn record_success_metrics<C: Command>(
        &self,
        command: &C,
        duration: Duration,
        retry_attempts: u32,
    ) {
        let metrics = CommandMetrics {
            command_type: command.command_type().to_string(),
            duration_ms: duration.as_millis() as u64,
            success: true,
            retry_attempts,
            error_type: None,
        };
        
        self.metrics_collector.record_metrics(metrics).await;
    }
    
    async fn record_failure_metrics<C: Command>(
        &self,
        command: &C,
        start_time: Instant,
        retry_attempts: u32,
        error: &CommandError,
    ) {
        let duration = start_time.elapsed();
        let metrics = CommandMetrics {
            command_type: command.command_type().to_string(),
            duration_ms: duration.as_millis() as u64,
            success: false,
            retry_attempts,
            error_type: Some(format!("{:?}", error)),
        };
        
        self.metrics_collector.record_metrics(metrics).await;
    }
    
    /// List all registered command types
    pub fn list_command_types(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a command registry
pub struct CommandRegistryBuilder {
    registry: CommandRegistry,
}

impl CommandRegistryBuilder {
    /// Create a new registry builder with default configuration
    pub fn new() -> Self {
        Self {
            registry: CommandRegistry::new(),
        }
    }
    
    /// Create a new registry builder with custom configuration
    pub fn with_config(config: RegistryConfig) -> Self {
        Self {
            registry: CommandRegistry::with_config(config),
        }
    }
    
    /// Create a new registry builder with custom configuration and metrics collector
    pub fn with_config_and_metrics(
        config: RegistryConfig,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            registry: CommandRegistry::with_config_and_metrics(config, metrics_collector),
        }
    }
    
    /// Register a command handler with error mapper
    pub fn register<C, H>(
        mut self,
        command_type: String,
        handler: Arc<H>,
        error_mapper: Arc<dyn CommandErrorMapper>,
    ) -> Self
    where
        C: Command + 'static,
        H: CommandHandler<C> + 'static,
    {
        self.registry.register::<C, H>(command_type, handler, error_mapper);
        self
    }
    
    /// Register a command handler with error mapper without type checking
    pub fn register_raw(
        mut self,
        command_type: String,
        handler: Arc<dyn DynCommandHandler>,
    ) -> Self {
        self.registry.register_raw(command_type, handler);
        self
    }
    
    /// Build the registry
    pub fn build(self) -> CommandRegistry {
        self.registry
    }
}

impl Default for CommandRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to simplify command registration
#[macro_export]
macro_rules! register_command {
    ($builder:expr, $command_type:ty, $handler:expr, $error_mapper:expr) => {
        {
            // Create a sample command to get the command type string
            // This is a bit hacky but works for the type system
            let sample_type_name = std::any::type_name::<$command_type>();
            $builder.register::<$command_type, _>(
                sample_type_name.to_string(),
                $handler,
                $error_mapper,
            )
        }
    };
    ($builder:expr, $command_type:ty, $command_name:expr, $handler:expr, $error_mapper:expr) => {
        {
            $builder.register::<$command_type, _>(
                $command_name.to_string(),
                $handler,
                $error_mapper,
            )
        }
    };
}

/// Macro to simplify inventory-based command registration
/// 
/// Usage in command modules:
/// ```rust
/// // At the end of login.rs
/// submit_command_registration! {
///     command_name: "login",
///     handler_factory: |container| {
///         let login_use_case = container
///             .get_dependency("LoginUseCase")
///             .and_then(|dep| dep.downcast::<Arc<dyn LoginUseCase>>().ok())
///             .map(|boxed| *boxed)
///             .expect("LoginUseCase dependency not found");
///         
///         Arc::new(CommandHandlerWrapper::new(
///             Arc::new(LoginCommandHandler::new(login_use_case)),
///             Arc::new(LoginErrorMapper),
///         ))
///     },
///     error_mapper: Arc::new(LoginErrorMapper),
/// }
/// ```
#[macro_export]
macro_rules! submit_command_registration {
    (
        command_name: $name:expr,
        handler_factory: $factory:expr,
        error_mapper: $mapper:expr $(,)?
    ) => {
        inventory::submit! {
            $crate::command::registry::CommandRegistration {
                command_name: $name,
                handler_factory: $factory,
                error_mapper_factory: $mapper,
            }
        }
    };
}

/*
ZERO-BOILERPLATE COMMAND REGISTRATION SYSTEM

BEFORE (manual factory.rs - 177 lines):
================================

```rust
pub fn create_iam_registry(
    login_usecase: Arc<dyn LoginUseCase>,
    link_provider_usecase: Arc<dyn LinkProviderUseCase>,
    token_usecase: Arc<dyn TokenUseCase>,
    user_usecase: Arc<dyn UserUseCase>,
    auth_usecase: Arc<dyn AuthUseCase>,
) -> CommandRegistry {
    let mut builder = CommandRegistryBuilder::new();

    // Register login commands
    let login_handler = Arc::new(LoginCommandHandler::new(login_usecase.clone()));
    let login_start_url_handler = Arc::new(GenerateLoginStartUrlCommandHandler::new(login_usecase));
    let login_error_mapper = Arc::new(LoginErrorMapper);

    builder = builder
        .register::<LoginCommand, _>("login".to_string(), login_handler, login_error_mapper.clone())
        .register::<GenerateLoginStartUrlCommand, _>("generate_login_start_url".to_string(), login_start_url_handler, login_error_mapper);

    // ... 160+ more lines of similar registration code ...
}
```

AFTER (inventory-based - 3 lines):
==================================

```rust
// In main application:
let container = SimpleDependencyContainer::new()
    .with_dependency("LoginUseCase", login_usecase)
    .with_dependency("LinkProviderUseCase", link_provider_usecase)
    .with_dependency("TokenUseCase", token_usecase)
    .with_dependency("UserUseCase", user_usecase)
    .with_dependency("AuthUseCase", auth_usecase);

let registry = build_registry_from_inventory(&container);
let service = GenericCommandService::new(Arc::new(registry));
```

COMMAND SELF-REGISTRATION:
=========================

Each command module (login.rs, signup.rs, etc.) adds at the bottom:

```rust
// At the end of login.rs:
inventory::submit! {
    CommandRegistration {
        command_name: "login",
        handler_factory: |container| {
            let login_use_case = container
                .get_dependency("LoginUseCase")
                .and_then(|dep| dep.downcast::<Arc<dyn LoginUseCase>>().ok())
                .map(|boxed| *boxed)
                .expect("LoginUseCase dependency not found");
            
            Arc::new(CommandHandlerWrapper::new(
                Arc::new(LoginCommandHandler::new(login_use_case)),
                Arc::new(LoginErrorMapper),
            ))
        },
        error_mapper_factory: Arc::new(LoginErrorMapper),
    }
}
```

BENEFITS:
=========
✅ Zero boilerplate in factory.rs (can delete entire 177-line file)
✅ Compile-time discovery - no runtime reflection needed
✅ Type-safe dependency injection
✅ Each command module is self-contained
✅ Adding new commands requires NO changes to registry/factory code
✅ Automatic plugin discovery - just add inventory::submit! to any command module

USAGE IN APPLICATION:
====================

```rust
use crate::command::registry::{build_registry_from_inventory, SimpleDependencyContainer};

// Set up dependencies
let container = SimpleDependencyContainer::new()
    .with_dependency("LoginUseCase", login_usecase)
    .with_dependency("AuthUseCase", auth_usecase);

// Build registry automatically from all registered commands
let registry = build_registry_from_inventory(&container);
let service = GenericCommandService::new(Arc::new(registry));
```

The inventory crate automatically collects all inventory::submit! calls at compile time,
eliminating the need for manual registration in factory.rs entirely.
*/

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // Test command implementation
    #[derive(Debug, Clone)]
    struct TestCommand {
        id: Uuid,
        data: String,
    }

    impl TestCommand {
        fn new(data: String) -> Self {
            Self {
                id: Uuid::new_v4(),
                data,
            }
        }
    }

    impl Command for TestCommand {
        type Result = String;

        fn command_type(&self) -> &'static str {
            "test_command"
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

    // Test handler implementation
    struct TestHandler;

    #[async_trait]
    impl CommandHandler<TestCommand> for TestHandler {
        async fn handle(&self, command: TestCommand) -> Result<String, CommandError> {
            Ok(format!("Processed: {}", command.data))
        }
    }

    // Test error mapper implementation
    struct TestErrorMapper;

    impl CommandErrorMapper for TestErrorMapper {
        fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
            CommandError::Infrastructure(error.to_string())
        }
    }

    #[tokio::test]
    async fn test_registry_registration_and_execution() {
        let mut registry = CommandRegistry::new();
        let handler = Arc::new(TestHandler);
        let error_mapper = Arc::new(TestErrorMapper);
        
        registry.register::<TestCommand, _>("test_command".to_string(), handler, error_mapper);
        
        let command = TestCommand::new("test data".to_string());
        let context = CommandContext::new();
        
        let result = registry.execute_command(command, context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: test data");
    }

    #[tokio::test]
    async fn test_registry_builder() {
        let handler = Arc::new(TestHandler);
        let error_mapper = Arc::new(TestErrorMapper);
        
        let registry = CommandRegistryBuilder::new()
            .register::<TestCommand, _>("test_command".to_string(), handler, error_mapper)
            .build();
        
        let command = TestCommand::new("test data".to_string());
        let context = CommandContext::new();
        
        let result = registry.execute_command(command, context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: test data");
    }

    #[test]
    fn test_registry_lists_command_types() {
        let handler = Arc::new(TestHandler);
        let error_mapper = Arc::new(TestErrorMapper);
        
        let registry = CommandRegistryBuilder::new()
            .register::<TestCommand, _>("test_command".to_string(), handler, error_mapper)
            .build();
        
        let types = registry.list_command_types();
        assert_eq!(types, vec!["test_command"]);
    }

    #[tokio::test]
    async fn test_registry_handles_unknown_command() {
        let registry = CommandRegistry::new();
        let command = TestCommand::new("test data".to_string());
        let context = CommandContext::new();
        
        let result = registry.execute_command(command, context).await;
        assert!(result.is_err());
        
        if let Err(CommandError::Infrastructure(msg)) = result {
            assert!(msg.contains("No handler registered"));
        } else {
            panic!("Expected infrastructure error");
        }
    }
} 