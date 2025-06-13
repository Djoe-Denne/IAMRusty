use rustycog_command::{Command, CommandError, CommandHandler, CommandErrorMapper};
use crate::usecase::login::{LoginUseCase, LoginRequest, LoginResponse, LoginError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;


/// Authentication error codes for consistent error handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthErrorCode {
    InvalidCredentials,
    EmailNotVerified,
    UserAlreadyExists,
    UserNotFound,
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
            AuthErrorCode::InvalidCredentials => "invalid_credentials",
            AuthErrorCode::EmailNotVerified => "email_not_verified",
            AuthErrorCode::UserAlreadyExists => "user_already_exists",
            AuthErrorCode::UserNotFound => "user_not_found",
            AuthErrorCode::WeakPassword => "weak_password",
            AuthErrorCode::InvalidEmail => "invalid_email",
            AuthErrorCode::EmailNotFound => "email_not_found",
            AuthErrorCode::EmailAlreadyVerified => "email_already_verified",
            AuthErrorCode::InvalidVerificationToken => "invalid_verification_token",
            AuthErrorCode::VerificationTokenExpired => "verification_token_expired",
            AuthErrorCode::RepositoryError => "repository_error",
            AuthErrorCode::EventPublishingError => "event_publishing_error",
            AuthErrorCode::TokenServiceError => "token_service_error",
            AuthErrorCode::PasswordHashingError => "password_hashing_error",
            AuthErrorCode::VerificationTokenGenerationError => "verification_token_generation_error",
            AuthErrorCode::AuthenticationFailed => "authentication_failed",
            AuthErrorCode::ValidationFailed => "validation_failed",
        }
    }
}

/// Error mapper for auth errors to command errors
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

/// Password login command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordLoginCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Email address
    pub email: String,
    /// Password
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
        // Basic validation without external validator crate
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
    A: LoginUseCase + ?Sized,
{
    login_use_case: Arc<A>,
}

impl<A> PasswordLoginCommandHandler<A>
where
    A: LoginUseCase + ?Sized,
{
    /// Create a new password login command handler
    pub fn new(login_use_case: Arc<A>) -> Self {
        Self {
            login_use_case,
        }
    }
}

#[async_trait]
impl<A> CommandHandler<PasswordLoginCommand> for PasswordLoginCommandHandler<A>
where
    A: LoginUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: PasswordLoginCommand) -> Result<LoginResponse, CommandError> {
        let request = LoginRequest {
            email: command.email,
            password: command.password,
        };

        self.login_use_case
            .login(request)
            .await
            .map_err(|e| AuthErrorMapper.map_error(Box::new(e)))
    }
}
