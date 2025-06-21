use async_trait::async_trait;
use rustycog_command::{Command, CommandHandler, CommandError, ValidateTokenCommand};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Simple user ID extractor (no verification)
#[derive(Clone)]
pub struct UserIdExtractor {
    /// Default user ID to use (for testing/development)
    default_user_id: Option<Uuid>,
}

impl UserIdExtractor {
    /// Create a new user ID extractor
    pub fn new() -> Self {
        Self {
            default_user_id: None,
        }
    }

    /// Create a new user ID extractor with a default user ID
    pub fn with_default_user_id(user_id: Uuid) -> Self {
        Self {
            default_user_id: Some(user_id),
        }
    }

    /// Extract user ID from token (simple string parsing, no verification)
    pub fn extract_user_id(&self, token: &str) -> Result<Uuid, CommandError> {
        debug!("Extracting user ID from token (no verification)");

        // If a default user ID is set, use it
        if let Some(user_id) = self.default_user_id {
            debug!("Using default user ID: {}", user_id);
            return Ok(user_id);
        }

        // Try to parse the token as a direct UUID
        if let Ok(user_id) = Uuid::parse_str(token) {
            debug!("Parsed token as UUID: {}", user_id);
            return Ok(user_id);
        }

        // Try to extract from a simple format: "user:UUID"
        if token.starts_with("user:") {
            let user_id_str = &token[5..];
            if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                debug!("Extracted user ID from 'user:' prefix: {}", user_id);
                return Ok(user_id);
            }
        }

        // If all else fails, generate a random UUID for development
        let user_id = Uuid::new_v4();
        debug!("Generated random user ID for development: {}", user_id);
        Ok(user_id)
    }
}

/// Command handler for simple user ID extraction
pub struct UserIdExtractionHandler {
    extractor: Arc<UserIdExtractor>,
}

impl UserIdExtractionHandler {
    /// Create a new user ID extraction handler
    pub fn new(extractor: UserIdExtractor) -> Self {
        Self {
            extractor: Arc::new(extractor),
        }
    }
}

#[async_trait]
impl CommandHandler<ValidateTokenCommand> for UserIdExtractionHandler {
    async fn handle(&self, command: ValidateTokenCommand) -> Result<Uuid, CommandError> {
        debug!("Handling ValidateTokenCommand with ID: {}", command.command_id());
        
        // Validate the command first
        command.validate()?;
        
        // Extract user ID from token (no verification)
        self.extractor.extract_user_id(&command.token)
    }
} 