use crate::usecase::login::{LoginError, LoginUseCase, VerifyEmailRequest, VerifyEmailResponse};
use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
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
                LoginError::UserNotFound => {
                    CommandError::business(AuthErrorCode::UserNotFound.as_str(), "User not found")
                }
                LoginError::InvalidCredentials => CommandError::business(
                    AuthErrorCode::InvalidCredentials.as_str(),
                    "Invalid credentials", // Don't leak user existence
                ),
                LoginError::EmailNotVerified => CommandError::business(
                    AuthErrorCode::EmailNotVerified.as_str(),
                    "Email not verified",
                ),
                LoginError::UserAlreadyExists => CommandError::business(
                    AuthErrorCode::UserAlreadyExists.as_str(),
                    "User already exists",
                ),
                LoginError::WeakPassword => CommandError::validation(
                    AuthErrorCode::WeakPassword.as_str(),
                    "Password is too weak",
                ),
                LoginError::InvalidEmail => CommandError::validation(
                    AuthErrorCode::InvalidEmail.as_str(),
                    "Invalid email format",
                ),
                LoginError::EmailNotFound => CommandError::business(
                    AuthErrorCode::EmailNotFound.as_str(),
                    "Invalid verification request", // Don't leak email existence
                ),
                LoginError::EmailAlreadyVerified => CommandError::business(
                    AuthErrorCode::EmailAlreadyVerified.as_str(),
                    "Email is already verified",
                ),
                LoginError::InvalidVerificationToken => CommandError::validation(
                    AuthErrorCode::InvalidVerificationToken.as_str(),
                    "Invalid or expired verification token",
                ),
                LoginError::VerificationTokenExpired => CommandError::validation(
                    AuthErrorCode::VerificationTokenExpired.as_str(),
                    "Verification token has expired",
                ),
                LoginError::AuthServiceError(msg) => {
                    if Self::is_authentication_related_error(msg) {
                        CommandError::validation(
                            AuthErrorCode::AuthenticationFailed.as_str(),
                            format!("Authentication failed: {}", msg),
                        )
                    } else {
                        CommandError::infrastructure(
                            AuthErrorCode::RepositoryError.as_str(),
                            msg.clone(),
                        )
                    }
                }
            }
        } else {
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::validation(
                    AuthErrorCode::AuthenticationFailed.as_str(),
                    format!("Authentication failed: {}", error_msg),
                )
            } else {
                CommandError::infrastructure(
                    AuthErrorCode::RepositoryError.as_str(),
                    error.to_string(),
                )
            }
        }
    }
}

impl AuthErrorMapper {
    fn is_authentication_related_error(error_msg: &str) -> bool {
        error_msg.contains("expired")
            || error_msg.contains("invalid")
            || error_msg.contains("Token expired")
            || error_msg.contains("Invalid token")
            || error_msg.contains("JWT error")
            || error_msg.contains("malformed")
            || error_msg.contains("signature")
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
            return Err(CommandError::validation(
                AuthErrorCode::ValidationFailed.as_str(),
                "Email cannot be empty",
            ));
        }

        if self.verification_token.trim().is_empty() {
            return Err(CommandError::validation(
                AuthErrorCode::ValidationFailed.as_str(),
                "Verification token cannot be empty",
            ));
        }

        // Basic email format validation
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err(CommandError::validation(
                AuthErrorCode::InvalidEmail.as_str(),
                "Invalid email format",
            ));
        }

        Ok(())
    }
}

/// Email verification command handler
pub struct VerifyEmailCommandHandler<A>
where
    A: LoginUseCase + ?Sized,
{
    login_use_case: Arc<A>,
}

impl<A> VerifyEmailCommandHandler<A>
where
    A: LoginUseCase + ?Sized,
{
    /// Create a new email verification command handler
    pub fn new(login_use_case: Arc<A>) -> Self {
        Self { login_use_case }
    }
}

#[async_trait]
impl<A> CommandHandler<VerifyEmailCommand> for VerifyEmailCommandHandler<A>
where
    A: LoginUseCase + Send + Sync + ?Sized,
{
    async fn handle(
        &self,
        command: VerifyEmailCommand,
    ) -> Result<VerifyEmailResponse, CommandError> {
        let request = VerifyEmailRequest {
            email: command.email,
            verification_token: command.verification_token,
        };

        self.login_use_case
            .verify_email(request)
            .await
            .map_err(|e| AuthErrorMapper.map_error(Box::new(e)))
    }
}
