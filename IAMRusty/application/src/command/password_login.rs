use rustycog_command::{Command, CommandError, CommandHandler, CommandErrorMapper};
use crate::usecase::oauth::{AuthUseCase, LoginRequest, LoginResponse, AuthError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Error codes for authentication-related operations
#[derive(Debug, Clone)]
pub enum AuthErrorCode {
    InvalidCredentials,
    UserNotFound,
    EmailNotVerified,
    UserAlreadyExists,
    WeakPassword,
    InvalidEmail,
    EmailNotFound,
    EmailAlreadyVerified,
    InvalidVerificationToken,
    VerificationTokenExpired,
    RepositoryError,
    EventPublishingError,
    TokenServiceError,
    PasswordHashingError,
    VerificationTokenGenerationError,
    AuthenticationFailed,
    ValidationFailed,
}

impl AuthErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::UserNotFound => "user_not_found",
            Self::EmailNotVerified => "email_not_verified",
            Self::UserAlreadyExists => "user_already_exists",
            Self::WeakPassword => "weak_password",
            Self::InvalidEmail => "invalid_email",
            Self::EmailNotFound => "email_not_found",
            Self::EmailAlreadyVerified => "email_already_verified",
            Self::InvalidVerificationToken => "invalid_verification_token",
            Self::VerificationTokenExpired => "verification_token_expired",
            Self::RepositoryError => "repository_error",
            Self::EventPublishingError => "event_publishing_error",
            Self::TokenServiceError => "token_service_error",
            Self::PasswordHashingError => "password_hashing_error",
            Self::VerificationTokenGenerationError => "verification_token_generation_error",
            Self::AuthenticationFailed => "authentication_failed",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Error mapper for authentication-related commands (signup, password login, verify email)
pub struct AuthErrorMapper;

impl CommandErrorMapper for AuthErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(auth_error) = error.downcast_ref::<AuthError>() {
            match auth_error {
                AuthError::InvalidCredentials => CommandError::validation(
                    AuthErrorCode::InvalidCredentials.as_str(),
                    "Invalid credentials"
                ),
                AuthError::UserNotFound => CommandError::business(
                    AuthErrorCode::InvalidCredentials.as_str(),
                    "Invalid credentials" // Don't leak user existence
                ),
                AuthError::EmailNotVerified => CommandError::business(
                    AuthErrorCode::EmailNotVerified.as_str(),
                    "Email not verified"
                ),
                AuthError::UserAlreadyExists => CommandError::business(
                    AuthErrorCode::UserAlreadyExists.as_str(),
                    "User already exists"
                ),
                AuthError::WeakPassword => CommandError::validation(
                    AuthErrorCode::WeakPassword.as_str(),
                    "Password is too weak"
                ),
                AuthError::InvalidEmail => CommandError::validation(
                    AuthErrorCode::InvalidEmail.as_str(),
                    "Invalid email format"
                ),
                AuthError::EmailNotFound => CommandError::business(
                    AuthErrorCode::EmailNotFound.as_str(),
                    "Invalid verification request" // Don't leak email existence
                ),
                AuthError::EmailAlreadyVerified => CommandError::business(
                    AuthErrorCode::EmailAlreadyVerified.as_str(),
                    "Email is already verified"
                ),
                AuthError::InvalidVerificationToken => CommandError::validation(
                    AuthErrorCode::InvalidVerificationToken.as_str(),
                    "Invalid or expired verification token"
                ),
                AuthError::VerificationTokenExpired => CommandError::validation(
                    AuthErrorCode::VerificationTokenExpired.as_str(),
                    "Verification token has expired"
                ),
                AuthError::RepositoryError(_) => CommandError::infrastructure(
                    AuthErrorCode::RepositoryError.as_str(),
                    error.to_string()
                ),
                AuthError::EventPublishingError(_) => CommandError::infrastructure(
                    AuthErrorCode::EventPublishingError.as_str(),
                    error.to_string()
                ),
                AuthError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::validation(
                            AuthErrorCode::AuthenticationFailed.as_str(),
                            format!("Authentication failed: {}", error_msg)
                        )
                    } else {
                        CommandError::infrastructure(
                            AuthErrorCode::TokenServiceError.as_str(),
                            error.to_string()
                        )
                    }
                },
                AuthError::PasswordHashingError(_) => CommandError::infrastructure(
                    AuthErrorCode::PasswordHashingError.as_str(),
                    error.to_string()
                ),
                AuthError::VerificationTokenGenerationError(_) => CommandError::infrastructure(
                    AuthErrorCode::VerificationTokenGenerationError.as_str(),
                    error.to_string()
                ),
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
            return Err(CommandError::validation(
                AuthErrorCode::ValidationFailed.as_str(),
                "Email cannot be empty"
            ));
        }
        
        if self.password.trim().is_empty() {
            return Err(CommandError::validation(
                AuthErrorCode::ValidationFailed.as_str(),
                "Password cannot be empty"
            ));
        }
        
        // Basic email format validation
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err(CommandError::validation(
                AuthErrorCode::InvalidEmail.as_str(),
                "Invalid email format"
            ));
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
