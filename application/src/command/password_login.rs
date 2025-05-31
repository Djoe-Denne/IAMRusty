use super::{Command, CommandError, CommandHandler};
use super::registry::CommandErrorMapper;
use crate::usecase::auth::{AuthUseCase, LoginRequest, LoginResponse, AuthError};
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

/// Password login command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordLoginCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Email address for authentication
    pub email: String,
    /// Password for authentication
    pub password: String,
}

impl PasswordLoginCommand {
    /// Create a new password login command
    pub fn new(email: String, password: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            email,
            password,
        }
    }
}

impl Command for PasswordLoginCommand {
    type Result = LoginResponse;

    fn command_type(&self) -> &'static str {
        "password_login"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.email.trim().is_empty() {
            return Err(CommandError::Validation("Email cannot be empty".to_string()));
        }
        
        if self.password.trim().is_empty() {
            return Err(CommandError::Validation("Password cannot be empty".to_string()));
        }
        
        // Basic email format validation
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err(CommandError::Validation("Invalid email format".to_string()));
        }
        
        Ok(())
    }
}

/// Password login command handler
pub struct PasswordLoginCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    auth_use_case: Arc<A>,
}

impl<A> PasswordLoginCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    /// Create a new password login command handler
    pub fn new(auth_use_case: Arc<A>) -> Self {
        Self {
            auth_use_case,
        }
    }
}

#[async_trait]
impl<A> CommandHandler<PasswordLoginCommand> for PasswordLoginCommandHandler<A>
where
    A: AuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: PasswordLoginCommand) -> Result<LoginResponse, CommandError> {
        let request = LoginRequest {
            email: command.email,
            password: command.password,
        };

        self.auth_use_case
            .login(request)
            .await
            .map_err(|e| AuthErrorMapper.map_error(Box::new(e)))
    }
}

// Inventory-based command registration for zero-boilerplate plugin system
use super::registry::{CommandRegistration, CommandHandlerWrapper};

inventory::submit! {
    CommandRegistration {
        command_name: "password_login",
        handler_factory: |container| {
            let auth_use_case = container
                .get_dependency("AuthUseCase")
                .and_then(|dep| dep.downcast::<Arc<dyn crate::usecase::auth::AuthUseCase>>().ok())
                .map(|boxed| *boxed)
                .expect("AuthUseCase dependency not found");
            
            Arc::new(CommandHandlerWrapper::new(
                Arc::new(PasswordLoginCommandHandler::new(auth_use_case)),
                Arc::new(AuthErrorMapper),
            ))
        },
        error_mapper_factory: || Arc::new(AuthErrorMapper),
    }
} 