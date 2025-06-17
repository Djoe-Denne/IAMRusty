use crate::usecase::password_reset::{
    PasswordResetError, PasswordResetUseCase, RequestPasswordResetRequest, RequestPasswordResetResponse,
    ValidateResetTokenRequest, ValidateResetTokenResponse, ResetPasswordResponse, ResetPasswordRequest,
};
use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Request password reset command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPasswordResetCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Email address to send reset link to
    pub email: String,
}

impl RequestPasswordResetCommand {
    /// Create a new request password reset command
    pub fn new(email: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            email,
        }
    }
}

impl Command for RequestPasswordResetCommand {
    type Result = RequestPasswordResetResponse;

    fn command_type(&self) -> &'static str {
        "request_password_reset"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Basic validation
        if self.email.trim().is_empty() {
            return Err(CommandError::validation(
                "invalid_email",
                "Email cannot be empty",
            ));
        }

        // Basic email format validation
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err(CommandError::validation(
                "invalid_email",
                "Invalid email format",
            ));
        }

        Ok(())
    }
}

/// Request password reset command handler
pub struct RequestPasswordResetCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    password_reset_use_case: Arc<A>,
}

impl<A> RequestPasswordResetCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    /// Create a new request password reset command handler
    pub fn new(password_reset_use_case: Arc<A>) -> Self {
        Self { password_reset_use_case }
    }
}

#[async_trait]
impl<A> CommandHandler<RequestPasswordResetCommand> for RequestPasswordResetCommandHandler<A>
where
    A: PasswordResetUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: RequestPasswordResetCommand) -> Result<RequestPasswordResetResponse, CommandError> {
        let request = RequestPasswordResetRequest {
            email: command.email,
        };

        self.password_reset_use_case
            .request_password_reset(request)
            .await
            .map_err(|e| PasswordResetErrorMapper.map_error(Box::new(e)))
    }
}

/// Validate reset token command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResetTokenCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Reset token to validate
    pub reset_token: String,
}

impl ValidateResetTokenCommand {
    /// Create a new validate reset token command
    pub fn new(reset_token: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            reset_token,
        }
    }
}

impl Command for ValidateResetTokenCommand {
    type Result = ValidateResetTokenResponse;

    fn command_type(&self) -> &'static str {
        "validate_reset_token"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Basic validation - only check if token is not empty
        if self.reset_token.trim().is_empty() {
            return Err(CommandError::validation(
                "invalid_token",
                "Reset token cannot be empty",
            ));
        }

        // Don't validate business logic here - let the use case handle that
        Ok(())
    }
}

/// Validate reset token command handler
pub struct ValidateResetTokenCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    password_reset_use_case: Arc<A>,
}

impl<A> ValidateResetTokenCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    /// Create a new validate reset token command handler
    pub fn new(password_reset_use_case: Arc<A>) -> Self {
        Self { password_reset_use_case }
    }
}

#[async_trait]
impl<A> CommandHandler<ValidateResetTokenCommand> for ValidateResetTokenCommandHandler<A>
where
    A: PasswordResetUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: ValidateResetTokenCommand) -> Result<ValidateResetTokenResponse, CommandError> {
        let request = ValidateResetTokenRequest {
            reset_token: command.reset_token,
        };

        self.password_reset_use_case
            .validate_reset_token(request)
            .await
            .map_err(|e| PasswordResetErrorMapper.map_error(Box::new(e)))
    }
}

/// Reset password unauthenticated command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordUnauthenticatedCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Reset token
    pub reset_token: String,
    /// New password
    pub new_password: String,
}

impl ResetPasswordUnauthenticatedCommand {
    /// Create a new reset password unauthenticated command
    pub fn new(reset_token: String, new_password: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            reset_token,
            new_password,
        }
    }
}

impl Command for ResetPasswordUnauthenticatedCommand {
    type Result = ResetPasswordResponse;

    fn command_type(&self) -> &'static str {
        "reset_password_unauthenticated"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Basic validation - don't validate business logic here
        if self.reset_token.trim().is_empty() {
            return Err(CommandError::validation(
                "invalid_token",
                "Reset token cannot be empty",
            ));
        }

        if self.new_password.trim().is_empty() {
            return Err(CommandError::validation(
                "invalid_password",
                "New password cannot be empty",
            ));
        }

        // Basic password length check
        if self.new_password.len() < 8 {
            return Err(CommandError::validation(
                "password_too_short",
                "Password must be at least 8 characters",
            ));
        }

        Ok(())
    }
}

/// Reset password unauthenticated command handler
pub struct ResetPasswordUnauthenticatedCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    password_reset_use_case: Arc<A>,
}

impl<A> ResetPasswordUnauthenticatedCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    /// Create a new reset password unauthenticated command handler
    pub fn new(password_reset_use_case: Arc<A>) -> Self {
        Self { password_reset_use_case }
    }
}

#[async_trait]
impl<A> CommandHandler<ResetPasswordUnauthenticatedCommand> for ResetPasswordUnauthenticatedCommandHandler<A>
where
    A: PasswordResetUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: ResetPasswordUnauthenticatedCommand) -> Result<ResetPasswordResponse, CommandError> {
        let request = ResetPasswordRequest {
            email: None,
            reset_token: Some(command.reset_token),
            new_password: command.new_password,
        };

        self.password_reset_use_case
            .reset_password_unauthenticated(request)
            .await
            .map_err(|e| PasswordResetErrorMapper.map_error(Box::new(e)))
    }
}

/// Reset password authenticated command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordAuthenticatedCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// User ID from authentication
    pub user_id: Uuid,
    /// Current password for verification
    pub current_password: String,
    /// New password
    pub new_password: String,
}

impl ResetPasswordAuthenticatedCommand {
    /// Create a new reset password authenticated command
    pub fn new(user_id: Uuid, current_password: String, new_password: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            user_id,
            current_password,
            new_password,
        }
    }
}

impl Command for ResetPasswordAuthenticatedCommand {
    type Result = ResetPasswordResponse;

    fn command_type(&self) -> &'static str {
        "reset_password_authenticated"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Basic validation
        if self.new_password.len() < 8 {
            return Err(CommandError::validation(
                "weak_password",
                "Password must be at least 8 characters",
            ));
        }

        Ok(())
    }
}

/// Reset password authenticated command handler
pub struct ResetPasswordAuthenticatedCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    password_reset_use_case: Arc<A>,
}

impl<A> ResetPasswordAuthenticatedCommandHandler<A>
where
    A: PasswordResetUseCase + ?Sized,
{
    /// Create a new reset password authenticated command handler
    pub fn new(password_reset_use_case: Arc<A>) -> Self {
        Self { password_reset_use_case }
    }
}

#[async_trait]
impl<A> CommandHandler<ResetPasswordAuthenticatedCommand> for ResetPasswordAuthenticatedCommandHandler<A>
where
    A: PasswordResetUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: ResetPasswordAuthenticatedCommand) -> Result<ResetPasswordResponse, CommandError> {
        self.password_reset_use_case
            .reset_password_authenticated(command.user_id, command.current_password, command.new_password)
            .await
            .map_err(|e| PasswordResetErrorMapper.map_error(Box::new(e)))
    }
}

/// Password reset error codes
#[derive(Debug, Clone)]
pub enum PasswordResetErrorCode {
    UserNotFound,
    EmailNotFound,
    InvalidToken,
    TokenExpired,
    TokenAlreadyUsed,
    IncorrectCurrentPassword,
    RepositoryError,
    EventPublishingError,
    TokenServiceError,
    PasswordHashingError,
    AuthenticationFailed,
    ValidationFailed,
    NoPasswordAuth,
    AntiEnumerationSecurity,
}

impl PasswordResetErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordResetErrorCode::UserNotFound => "user_not_found",
            PasswordResetErrorCode::EmailNotFound => "email_not_found",
            PasswordResetErrorCode::InvalidToken => "invalid_token",
            PasswordResetErrorCode::TokenExpired => "token_expired",
            PasswordResetErrorCode::TokenAlreadyUsed => "token_already_used",
            PasswordResetErrorCode::IncorrectCurrentPassword => "incorrect_current_password",
            PasswordResetErrorCode::RepositoryError => "repository_error",
            PasswordResetErrorCode::EventPublishingError => "event_publishing_error",
            PasswordResetErrorCode::TokenServiceError => "token_service_error",
            PasswordResetErrorCode::PasswordHashingError => "password_hashing_error",
            PasswordResetErrorCode::AuthenticationFailed => "authentication_failed",
            PasswordResetErrorCode::ValidationFailed => "validation_failed",
            PasswordResetErrorCode::NoPasswordAuth => "no_password_auth",
            PasswordResetErrorCode::AntiEnumerationSecurity => "anti_enumeration_security",
        }
    }
}

/// Error mapper for password reset errors to command errors
pub struct PasswordResetErrorMapper;

impl CommandErrorMapper for PasswordResetErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(error) = error.downcast_ref::<PasswordResetError>() {
            match error {
                PasswordResetError::UserNotFound => {
                    // Anti-enumeration: Don't reveal user existence
                    CommandError::business(
                        PasswordResetErrorCode::AntiEnumerationSecurity.as_str(),
                        "Password reset request processed",
                    )
                }
                PasswordResetError::NoPasswordAuth => {
                    // Anti-enumeration: Don't reveal auth method details
                    CommandError::business(
                        PasswordResetErrorCode::AntiEnumerationSecurity.as_str(),
                        "Password reset request processed",
                    )
                }
                PasswordResetError::InvalidResetToken => CommandError::validation(
                    PasswordResetErrorCode::InvalidToken.as_str(),
                    "Invalid or malformed reset token",
                ),
                PasswordResetError::ResetTokenExpired => CommandError::validation(
                    PasswordResetErrorCode::TokenExpired.as_str(),
                    "Reset token has expired",
                ),
                PasswordResetError::ResetTokenAlreadyUsed => CommandError::validation(
                    PasswordResetErrorCode::TokenAlreadyUsed.as_str(),
                    "Reset token has already been used",
                ),
                PasswordResetError::IncorrectCurrentPassword => CommandError::validation(
                    PasswordResetErrorCode::IncorrectCurrentPassword.as_str(),
                    "Current password is incorrect",
                ),
                PasswordResetError::WeakPassword => CommandError::validation(
                    PasswordResetErrorCode::ValidationFailed.as_str(),
                    "Password does not meet security requirements",
                ),
                PasswordResetError::RepositoryError(msg) => {
                    CommandError::infrastructure(
                        PasswordResetErrorCode::RepositoryError.as_str(),
                        format!("Repository error: {}", msg),
                    )
                }
                PasswordResetError::EventPublishingError(msg) => {
                    CommandError::infrastructure(
                        PasswordResetErrorCode::EventPublishingError.as_str(),
                        format!("Event publishing error: {}", msg),
                    )
                }
                PasswordResetError::ServiceError(msg) => {
                    CommandError::infrastructure(
                        PasswordResetErrorCode::TokenServiceError.as_str(),
                        format!("Service error: {}", msg),
                    )
                }
            }
        } else {
            // Handle unknown errors
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::authentication(
                    PasswordResetErrorCode::AuthenticationFailed.as_str(),
                    format!("Authentication failed: {}", error_msg),
                )
            } else if Self::is_validation_related_error(&error_msg) {
                CommandError::validation(
                    PasswordResetErrorCode::ValidationFailed.as_str(),
                    format!("Validation failed: {}", error_msg),
                )
            } else {
                CommandError::infrastructure(
                    PasswordResetErrorCode::RepositoryError.as_str(),
                    error.to_string(),
                )
            }
        }
    }
}

impl PasswordResetErrorMapper {
    fn is_authentication_related_error(error_msg: &str) -> bool {
        error_msg.contains("expired")
            || error_msg.contains("invalid")
            || error_msg.contains("Token expired")
            || error_msg.contains("Invalid token")
            || error_msg.contains("JWT error")
            || error_msg.contains("malformed")
            || error_msg.contains("signature")
            || error_msg.contains("authentication")
            || error_msg.contains("unauthorized")
    }

    fn is_validation_related_error(error_msg: &str) -> bool {
        error_msg.contains("validation")
            || error_msg.contains("format")
            || error_msg.contains("required")
            || error_msg.contains("invalid")
            || error_msg.contains("missing")
            || error_msg.contains("empty")
    }
} 