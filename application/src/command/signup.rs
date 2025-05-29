use super::{Command, CommandError, CommandHandler, error_mapping::ErrorMapping};
use crate::usecase::auth::{AuthUseCase, SignupRequest, SignupResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Signup command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Username for the new user
    pub username: String,
    /// Email address for the new user
    pub email: String,
    /// Password for the new user
    pub password: String,
}

impl SignupCommand {
    /// Create a new signup command
    pub fn new(username: String, email: String, password: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            username,
            email,
            password,
        }
    }
}

impl Command for SignupCommand {
    type Result = SignupResponse;

    fn command_type(&self) -> &'static str {
        "signup"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Only validate business rules here, not input format
        // Input format validation is handled at the HTTP layer
        
        // These are basic sanity checks to ensure the command is properly constructed
        if self.username.trim().is_empty() {
            return Err(CommandError::Validation("Username cannot be empty".to_string()));
        }
        
        if self.email.trim().is_empty() {
            return Err(CommandError::Validation("Email cannot be empty".to_string()));
        }
        
        if self.password.is_empty() {
            return Err(CommandError::Validation("Password cannot be empty".to_string()));
        }
        
        Ok(())
    }
}

/// Signup command handler
pub struct SignupCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    auth_use_case: Arc<A>,
}

impl<A> SignupCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    /// Create a new signup command handler
    pub fn new(auth_use_case: Arc<A>) -> Self {
        Self {
            auth_use_case,
        }
    }
}

#[async_trait]
impl<A> CommandHandler<SignupCommand> for SignupCommandHandler<A>
where
    A: AuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: SignupCommand) -> Result<SignupResponse, CommandError> {
        let request = SignupRequest {
            username: command.username,
            email: command.email,
            password: command.password,
        };

        self.auth_use_case
            .signup(request)
            .await
            .map_err(ErrorMapping::map_auth_error)
    }
} 