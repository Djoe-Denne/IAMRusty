use super::{Command, CommandError, CommandHandler, error_mappers::AuthErrorMapper};
use super::registry::CommandErrorMapper;
use crate::usecase::auth::{AuthUseCase, VerifyEmailRequest, VerifyEmailResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Email verification command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyEmailCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Email address to verify
    pub email: String,
    /// Verification token
    pub verification_token: String,
}

impl VerifyEmailCommand {
    /// Create a new email verification command
    pub fn new(email: String, verification_token: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            email,
            verification_token,
        }
    }
}

impl Command for VerifyEmailCommand {
    type Result = VerifyEmailResponse;

    fn command_type(&self) -> &'static str {
        "verify_email"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.email.trim().is_empty() {
            return Err(CommandError::Validation("Email cannot be empty".to_string()));
        }
        
        if self.verification_token.trim().is_empty() {
            return Err(CommandError::Validation("Verification token cannot be empty".to_string()));
        }
        
        // Basic email format validation
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err(CommandError::Validation("Invalid email format".to_string()));
        }
        
        Ok(())
    }
}

/// Email verification command handler
pub struct VerifyEmailCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    auth_use_case: Arc<A>,
}

impl<A> VerifyEmailCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    /// Create a new email verification command handler
    pub fn new(auth_use_case: Arc<A>) -> Self {
        Self {
            auth_use_case,
        }
    }
}

#[async_trait]
impl<A> CommandHandler<VerifyEmailCommand> for VerifyEmailCommandHandler<A>
where
    A: AuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: VerifyEmailCommand) -> Result<VerifyEmailResponse, CommandError> {
        let request = VerifyEmailRequest {
            email: command.email,
            verification_token: command.verification_token,
        };

        self.auth_use_case
            .verify_email(request)
            .await
            .map_err(|e| AuthErrorMapper.map_error(Box::new(e)))
    }
} 