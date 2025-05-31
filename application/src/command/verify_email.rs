use rustycog_command::{Command, CommandError, CommandHandler, CommandErrorMapper};
use crate::usecase::auth::{AuthUseCase, VerifyEmailRequest, VerifyEmailResponse, AuthError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Error mapper for authentication-related commands (signup, password login, verify email)
pub struct AuthErrorMapper;

impl CommandErrorMapper for AuthErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(auth_error) = error.downcast_ref::<AuthError>() {
            match auth_error {
                AuthError::InvalidCredentials => CommandError::Validation("Invalid credentials".to_string()),
                AuthError::UserNotFound => CommandError::Business("Invalid credentials".to_string()), // Don't leak user existence
                AuthError::EmailNotVerified => CommandError::Business("Email not verified".to_string()),
                AuthError::UserAlreadyExists => CommandError::Business("User already exists".to_string()),
                AuthError::WeakPassword => CommandError::Validation("Password is too weak".to_string()),
                AuthError::InvalidEmail => CommandError::Validation("Invalid email format".to_string()),
                AuthError::EmailNotFound => CommandError::Business("Invalid verification request".to_string()), // Don't leak email existence
                AuthError::EmailAlreadyVerified => CommandError::Business("Email is already verified".to_string()),
                AuthError::InvalidVerificationToken => CommandError::Validation("Invalid or expired verification token".to_string()),
                AuthError::VerificationTokenExpired => CommandError::Validation("Verification token has expired".to_string()),
                AuthError::RepositoryError(_) => CommandError::Infrastructure(error.to_string()),
                AuthError::EventPublishingError(_) => CommandError::Infrastructure(error.to_string()),
                AuthError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::Validation(format!("Authentication failed: {}", error_msg))
                    } else {
                        CommandError::Infrastructure(error.to_string())
                    }
                },
                AuthError::PasswordHashingError(_) => CommandError::Infrastructure(error.to_string()),
                AuthError::VerificationTokenGenerationError(_) => CommandError::Infrastructure(error.to_string()),
            }
        } else {
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::Validation(format!("Authentication failed: {}", error_msg))
            } else {
                CommandError::Infrastructure(error.to_string())
            }
        }
    }
}

impl AuthErrorMapper {
    fn is_authentication_related_error(error_msg: &str) -> bool {
        error_msg.contains("expired") || 
        error_msg.contains("invalid") || 
        error_msg.contains("Token expired") ||
        error_msg.contains("Invalid token") ||
        error_msg.contains("JWT error") ||
        error_msg.contains("malformed") ||
        error_msg.contains("signature")
    }
}

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
