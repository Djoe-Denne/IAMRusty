pub mod bus;
pub mod login;
pub mod link_provider;
pub mod token;
pub mod user;
pub mod service;

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