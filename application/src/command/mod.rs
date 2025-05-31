pub mod login;
pub mod link_provider;
pub mod token;
pub mod user;
pub mod signup;
pub mod password_login;
pub mod verify_email;
pub mod registry;
pub mod generic_service;

pub mod factory;

// Re-exports for the extensible command system
pub use registry::{
    CommandRegistry, CommandRegistryBuilder, CommandErrorMapper, DynCommandHandler,
    RetryPolicy, RegistryConfig, MetricsCollector, LoggingMetricsCollector
};
pub use generic_service::GenericCommandService;
pub use factory::CommandRegistryFactory;

// Convenience alias for the new service
pub type ExtensibleCommandService = GenericCommandService;

use async_trait::async_trait;

use std::fmt::Debug;
use thiserror::Error;
use uuid::Uuid;

/// Command execution error
#[derive(Debug, Error)]
pub enum CommandError {
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Business logic error
    #[error("Business error: {0}")]
    Business(String),
    
    /// Infrastructure error (database, external services, etc.)
    #[error("Infrastructure error: {0}")]
    Infrastructure(String),
    
    /// Timeout error
    #[error("Command execution timeout")]
    Timeout,
    
    /// Retry exhausted error
    #[error("Maximum retries exhausted: {0}")]
    RetryExhausted(String),
}

/// Command trait that all commands must implement
#[async_trait]
pub trait Command: Debug + Send + Sync {
    /// The result type returned by this command
    type Result: Send + Sync;
    
    /// Unique identifier for this command type
    fn command_type(&self) -> &'static str;
    
    /// Unique identifier for this command instance
    fn command_id(&self) -> Uuid;
    
    /// Validate the command before execution
    fn validate(&self) -> Result<(), CommandError>;
}

/// Command handler trait
#[async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    /// Execute the command
    async fn handle(&self, command: C) -> Result<C::Result, CommandError>;
}

/// Command execution context
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// Command execution ID
    pub execution_id: Uuid,
    /// User ID (if applicable)
    pub user_id: Option<Uuid>,
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl CommandContext {
    pub fn new() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            user_id: None,
            request_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
    
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Command execution metrics
#[derive(Debug, Clone)]
pub struct CommandMetrics {
    /// Command type
    pub command_type: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Whether the command succeeded
    pub success: bool,
    /// Number of retry attempts
    pub retry_attempts: u32,
    /// Error type (if failed)
    pub error_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;
    use rstest::*;
    use std::collections::HashMap;

    // Test fixtures
    #[fixture]
    fn sample_uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[fixture]
    fn sample_command_context() -> CommandContext {
        CommandContext {
            execution_id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            request_id: Some("req-123".to_string()),
            metadata: {
                let mut map = HashMap::new();
                map.insert("source".to_string(), "test".to_string());
                map
            },
        }
    }

    #[fixture]
    fn sample_command_metrics() -> CommandMetrics {
        CommandMetrics {
            command_type: "test_command".to_string(),
            duration_ms: 150,
            success: true,
            retry_attempts: 0,
            error_type: None,
        }
    }

    mod command_error {
        use super::*;

        #[test]
        fn validation_error_displays_correctly() {
            let error = CommandError::Validation("Invalid input".to_string());
            assert_eq!(error.to_string(), "Validation error: Invalid input");
        }

        #[test]
        fn business_error_displays_correctly() {
            let error = CommandError::Business("Business rule violated".to_string());
            assert_eq!(error.to_string(), "Business error: Business rule violated");
        }

        #[test]
        fn infrastructure_error_displays_correctly() {
            let error = CommandError::Infrastructure("Database connection failed".to_string());
            assert_eq!(error.to_string(), "Infrastructure error: Database connection failed");
        }

        #[test]
        fn timeout_error_displays_correctly() {
            let error = CommandError::Timeout;
            assert_eq!(error.to_string(), "Command execution timeout");
        }

        #[test]
        fn retry_exhausted_error_displays_correctly() {
            let error = CommandError::RetryExhausted("All retries failed".to_string());
            assert_eq!(error.to_string(), "Maximum retries exhausted: All retries failed");
        }

        #[test]
        fn command_error_is_debug() {
            let error = CommandError::Validation("test".to_string());
            let debug_str = format!("{:?}", error);
            assert!(debug_str.contains("Validation"));
        }

        #[test]
        fn command_error_is_send_sync() {
            fn assert_send_sync<T: Send + Sync>() {}
            assert_send_sync::<CommandError>();
        }
    }

    mod command_context {
        use super::*;

        #[test]
        fn new_creates_context_with_execution_id() {
            let context = CommandContext::new();
            
            // Should have a valid execution ID
            assert_ne!(context.execution_id, Uuid::nil());
            assert!(context.user_id.is_none());
            assert!(context.request_id.is_none());
            assert!(context.metadata.is_empty());
        }

        #[test]
        fn default_creates_same_as_new() {
            let context1 = CommandContext::new();
            let context2 = CommandContext::default();
            
            // Both should have valid execution IDs (though different)
            assert_ne!(context1.execution_id, Uuid::nil());
            assert_ne!(context2.execution_id, Uuid::nil());
            assert!(context1.user_id.is_none());
            assert!(context2.user_id.is_none());
        }

        #[rstest]
        #[test]
        fn with_user_id_sets_user_id(sample_uuid: Uuid) {
            let context = CommandContext::new().with_user_id(sample_uuid);
            
            assert_eq!(context.user_id, Some(sample_uuid));
        }

        #[test]
        fn with_request_id_sets_request_id() {
            let request_id = "req-456".to_string();
            let context = CommandContext::new().with_request_id(request_id.clone());
            
            assert_eq!(context.request_id, Some(request_id));
        }

        #[test]
        fn with_metadata_adds_metadata() {
            let context = CommandContext::new()
                .with_metadata("key1".to_string(), "value1".to_string())
                .with_metadata("key2".to_string(), "value2".to_string());
            
            assert_eq!(context.metadata.get("key1"), Some(&"value1".to_string()));
            assert_eq!(context.metadata.get("key2"), Some(&"value2".to_string()));
            assert_eq!(context.metadata.len(), 2);
        }

        #[test]
        fn builder_pattern_works() {
            let user_id = Uuid::new_v4();
            let context = CommandContext::new()
                .with_user_id(user_id)
                .with_request_id("req-789".to_string())
                .with_metadata("source".to_string(), "api".to_string());
            
            assert_eq!(context.user_id, Some(user_id));
            assert_eq!(context.request_id, Some("req-789".to_string()));
            assert_eq!(context.metadata.get("source"), Some(&"api".to_string()));
        }

        #[rstest]
        #[test]
        fn clone_creates_independent_copy(sample_command_context: CommandContext) {
            let cloned = sample_command_context.clone();
            
            assert_eq!(cloned.execution_id, sample_command_context.execution_id);
            assert_eq!(cloned.user_id, sample_command_context.user_id);
            assert_eq!(cloned.request_id, sample_command_context.request_id);
            assert_eq!(cloned.metadata, sample_command_context.metadata);
        }

        #[test]
        fn context_is_debug() {
            let context = CommandContext::new();
            let debug_str = format!("{:?}", context);
            assert!(debug_str.contains("CommandContext"));
        }
    }

    mod command_metrics {
        use super::*;

        #[rstest]
        #[test]
        fn metrics_stores_all_fields(sample_command_metrics: CommandMetrics) {
            assert_eq!(sample_command_metrics.command_type, "test_command");
            assert_eq!(sample_command_metrics.duration_ms, 150);
            assert!(sample_command_metrics.success);
            assert_eq!(sample_command_metrics.retry_attempts, 0);
            assert!(sample_command_metrics.error_type.is_none());
        }

        #[test]
        fn metrics_with_error() {
            let metrics = CommandMetrics {
                command_type: "failed_command".to_string(),
                duration_ms: 500,
                success: false,
                retry_attempts: 3,
                error_type: Some("ValidationError".to_string()),
            };
            
            assert!(!metrics.success);
            assert_eq!(metrics.retry_attempts, 3);
            assert_eq!(metrics.error_type, Some("ValidationError".to_string()));
        }

        #[rstest]
        #[test]
        fn metrics_clone_works(sample_command_metrics: CommandMetrics) {
            let cloned = sample_command_metrics.clone();
            
            assert_eq!(cloned.command_type, sample_command_metrics.command_type);
            assert_eq!(cloned.duration_ms, sample_command_metrics.duration_ms);
            assert_eq!(cloned.success, sample_command_metrics.success);
        }

        #[test]
        fn metrics_is_debug() {
            let metrics = CommandMetrics {
                command_type: "test".to_string(),
                duration_ms: 100,
                success: true,
                retry_attempts: 0,
                error_type: None,
            };
            
            let debug_str = format!("{:?}", metrics);
            assert!(debug_str.contains("CommandMetrics"));
        }
    }

    // Test command implementation for testing Command trait
    #[derive(Debug, Clone)]
    struct TestCommand {
        id: Uuid,
        data: String,
        should_fail_validation: bool,
    }

    impl TestCommand {
        fn new(data: String) -> Self {
            Self {
                id: Uuid::new_v4(),
                data,
                should_fail_validation: false,
            }
        }

        fn new_invalid() -> Self {
            Self {
                id: Uuid::new_v4(),
                data: "".to_string(),
                should_fail_validation: true,
            }
        }
    }

    #[async_trait]
    impl Command for TestCommand {
        type Result = String;

        fn command_type(&self) -> &'static str {
            "test_command"
        }

        fn command_id(&self) -> Uuid {
            self.id
        }

        fn validate(&self) -> Result<(), CommandError> {
            if self.should_fail_validation || self.data.is_empty() {
                Err(CommandError::Validation("Data cannot be empty".to_string()))
            } else {
                Ok(())
            }
        }
    }

    // Test command handler implementation
    struct TestCommandHandler {
        should_fail: bool,
    }

    impl TestCommandHandler {
        fn new() -> Self {
            Self { should_fail: false }
        }

        fn new_failing() -> Self {
            Self { should_fail: true }
        }
    }

    #[async_trait]
    impl CommandHandler<TestCommand> for TestCommandHandler {
        async fn handle(&self, command: TestCommand) -> Result<String, CommandError> {
            if self.should_fail {
                Err(CommandError::Business("Handler failed".to_string()))
            } else {
                Ok(format!("Processed: {}", command.data))
            }
        }
    }

    mod command_trait {
        use super::*;

        #[test]
        fn test_command_implements_command_trait() {
            let command = TestCommand::new("test data".to_string());
            
            assert_eq!(command.command_type(), "test_command");
            assert_ne!(command.command_id(), Uuid::nil());
            assert_ok!(command.validate());
        }

        #[test]
        fn test_command_validation_fails_for_empty_data() {
            let command = TestCommand::new("".to_string());
            
            let result = command.validate();
            assert_err!(&result);
            
            if let Err(CommandError::Validation(msg)) = result {
                assert!(msg.contains("Data cannot be empty"));
            } else {
                panic!("Expected validation error");
            }
        }

        #[test]
        fn test_command_validation_fails_when_configured() {
            let command = TestCommand::new_invalid();
            
            let result = command.validate();
            assert_err!(&result);
        }

        #[test]
        fn command_id_is_unique() {
            let command1 = TestCommand::new("data1".to_string());
            let command2 = TestCommand::new("data2".to_string());
            
            assert_ne!(command1.command_id(), command2.command_id());
        }
    }

    mod command_handler_trait {
        use super::*;

        #[tokio::test]
        async fn test_handler_processes_command_successfully() {
            let handler = TestCommandHandler::new();
            let command = TestCommand::new("test data".to_string());
            
            let result = handler.handle(command).await;
            assert_ok!(&result);
            
            let output = result.unwrap();
            assert_eq!(output, "Processed: test data");
        }

        #[tokio::test]
        async fn test_handler_can_fail() {
            let handler = TestCommandHandler::new_failing();
            let command = TestCommand::new("test data".to_string());
            
            let result = handler.handle(command).await;
            assert_err!(&result);
            
            if let Err(CommandError::Business(msg)) = result {
                assert!(msg.contains("Handler failed"));
            } else {
                panic!("Expected business error");
            }
        }

        #[tokio::test]
        async fn handler_is_send_sync() {
            fn assert_send_sync<T: Send + Sync>() {}
            assert_send_sync::<TestCommandHandler>();
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn command_context_handles_empty_metadata() {
            let context = CommandContext::new();
            assert!(context.metadata.is_empty());
        }

        #[test]
        fn command_context_metadata_can_be_overwritten() {
            let context = CommandContext::new()
                .with_metadata("key".to_string(), "value1".to_string())
                .with_metadata("key".to_string(), "value2".to_string());
            
            assert_eq!(context.metadata.get("key"), Some(&"value2".to_string()));
            assert_eq!(context.metadata.len(), 1);
        }

        #[test]
        fn command_metrics_handles_zero_duration() {
            let metrics = CommandMetrics {
                command_type: "instant_command".to_string(),
                duration_ms: 0,
                success: true,
                retry_attempts: 0,
                error_type: None,
            };
            
            assert_eq!(metrics.duration_ms, 0);
        }

        #[test]
        fn command_metrics_handles_high_retry_count() {
            let metrics = CommandMetrics {
                command_type: "retry_command".to_string(),
                duration_ms: 5000,
                success: false,
                retry_attempts: 100,
                error_type: Some("MaxRetriesExceeded".to_string()),
            };
            
            assert_eq!(metrics.retry_attempts, 100);
        }
    }
} 