use rustycog_command::{Command, CommandError, CommandHandler, CommandErrorMapper};
use crate::usecase::login::{LoginUseCase, SignupRequest, SignupResponse, LoginError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// Re-export AuthErrorCode from password_login module
pub use super::password_login::AuthErrorCode;

/// Error mapper for authentication-related commands (signup, password login, verify email)
pub struct AuthErrorMapper;

impl CommandErrorMapper for AuthErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(error) = error.downcast_ref::<LoginError>() {
            match error {
                LoginError::UserNotFound => CommandError::business(
                    AuthErrorCode::UserNotFound.as_str(),
                    "User not found"
                ),
                LoginError::InvalidCredentials => CommandError::business(
                    AuthErrorCode::InvalidCredentials.as_str(),
                    "Invalid credentials" // Don't leak user existence
                ),
                LoginError::EmailNotVerified => CommandError::business(
                    AuthErrorCode::EmailNotVerified.as_str(),
                    "Email not verified"
                ),
                LoginError::UserAlreadyExists => CommandError::business(
                    AuthErrorCode::UserAlreadyExists.as_str(),
                    "User already exists"
                ),
                LoginError::WeakPassword => CommandError::validation(
                    AuthErrorCode::WeakPassword.as_str(),
                    "Password is too weak"
                ),
                LoginError::InvalidEmail => CommandError::validation(
                    AuthErrorCode::InvalidEmail.as_str(),
                    "Invalid email format"
                ),
                LoginError::EmailNotFound => CommandError::business(
                    AuthErrorCode::EmailNotFound.as_str(),
                    "Invalid verification request" // Don't leak email existence
                ),
                LoginError::EmailAlreadyVerified => CommandError::business(
                    AuthErrorCode::EmailAlreadyVerified.as_str(),
                    "Email is already verified"
                ),
                LoginError::InvalidVerificationToken => CommandError::validation(
                    AuthErrorCode::InvalidVerificationToken.as_str(),
                    "Invalid or expired verification token"
                ),
                LoginError::VerificationTokenExpired => CommandError::validation(
                    AuthErrorCode::VerificationTokenExpired.as_str(),
                    "Verification token has expired"
                ),
                LoginError::AuthServiceError(msg) => {
                    if Self::is_authentication_related_error(msg) {
                        CommandError::validation(
                            AuthErrorCode::AuthenticationFailed.as_str(),
                            format!("Authentication failed: {}", msg)
                        )
                    } else {
                        CommandError::infrastructure(
                            AuthErrorCode::RepositoryError.as_str(),
                            msg.clone()
                        )
                    }
                },
            }
        } else {
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::validation(
                    AuthErrorCode::AuthenticationFailed.as_str(),
                    format!("Authentication failed: {}", error_msg)
                )
            } else {
                CommandError::infrastructure(
                    AuthErrorCode::RepositoryError.as_str(),
                    error.to_string()
                )
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

/// Signup command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Email address for the new user
    pub email: String,
    /// Password for the new user
    pub password: String,
}

impl SignupCommand {
    /// Create a new signup command
    pub fn new(email: String, password: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
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
        if self.email.trim().is_empty() {
            return Err(CommandError::validation(
                AuthErrorCode::ValidationFailed.as_str(),
                "Email cannot be empty"
            ));
        }
        
        if self.password.is_empty() {
            return Err(CommandError::validation(
                AuthErrorCode::ValidationFailed.as_str(),
                "Password cannot be empty"
            ));
        }
        
        Ok(())
    }
}

/// Signup command handler
pub struct SignupCommandHandler<A>
where
    A: LoginUseCase + ?Sized,
{
    login_use_case: Arc<A>,
}

impl<A> SignupCommandHandler<A>
where
    A: LoginUseCase + ?Sized,
{
    /// Create a new signup command handler
    pub fn new(login_use_case: Arc<A>) -> Self {
        Self {
            login_use_case,
        }
    }
}

#[async_trait]
impl<A> CommandHandler<SignupCommand> for SignupCommandHandler<A>
where
    A: LoginUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: SignupCommand) -> Result<SignupResponse, CommandError> {
        let request = SignupRequest {
            email: command.email,
            password: command.password,
        };

        self.login_use_case
            .signup(request)
            .await
            .map_err(|e| AuthErrorMapper.map_error(Box::new(e)))
    }
}
