use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::dto::auth::{
    CheckUsernameRequest, CheckUsernameResponse, CompleteRegistrationRequest,
    CompleteRegistrationResponse,
};
use crate::usecase::registration::{RegistrationError, RegistrationUseCase};
use iam_domain::error::DomainError;

/// Error codes for registration-related operations
#[derive(Debug, Clone)]
pub enum RegistrationErrorCode {
    RepositoryError,
    TokenServiceError,
    EventError,
    InvalidToken,
    TokenExpired,
    UsernameTaken,
    InvalidUsername,
    UserNotFound,
    RegistrationAlreadyComplete,
    ValidationFailed,
}

impl RegistrationErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RepositoryError => "repository_error",
            Self::TokenServiceError => "token_service_error",
            Self::EventError => "event_error",
            Self::InvalidToken => "invalid_token",
            Self::TokenExpired => "token_expired",
            Self::UsernameTaken => "username_taken",
            Self::InvalidUsername => "invalid_username",
            Self::UserNotFound => "user_not_found",
            Self::RegistrationAlreadyComplete => "registration_already_complete",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Error mapper for registration-related commands
pub struct RegistrationErrorMapper;

impl CommandErrorMapper for RegistrationErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(reg_error) = error.downcast_ref::<RegistrationError>() {
            match reg_error {
                RegistrationError::DomainError(domain_error) => match domain_error {
                    DomainError::RepositoryError(_) => CommandError::infrastructure(
                        RegistrationErrorCode::RepositoryError.as_str(),
                        domain_error.to_string(),
                    ),
                    DomainError::TokenServiceError(_) => CommandError::infrastructure(
                        RegistrationErrorCode::TokenServiceError.as_str(),
                        domain_error.to_string(),
                    ),
                    DomainError::EventError(_) => CommandError::infrastructure(
                        RegistrationErrorCode::EventError.as_str(),
                        domain_error.to_string(),
                    ),
                    DomainError::InvalidToken => CommandError::validation(
                        RegistrationErrorCode::InvalidToken.as_str(),
                        "Invalid or expired registration token",
                    ),
                    DomainError::TokenExpired => CommandError::validation(
                        RegistrationErrorCode::TokenExpired.as_str(),
                        "Registration token has expired",
                    ),
                    DomainError::UsernameTaken => CommandError::business(
                        RegistrationErrorCode::UsernameTaken.as_str(),
                        "Username already taken",
                    ),
                    DomainError::InvalidUsername => CommandError::validation(
                        RegistrationErrorCode::InvalidUsername.as_str(),
                        "Invalid username format",
                    ),
                    DomainError::UserNotFound => CommandError::business(
                        RegistrationErrorCode::UserNotFound.as_str(),
                        "User not found",
                    ),
                    DomainError::RegistrationAlreadyComplete => CommandError::validation(
                        RegistrationErrorCode::RegistrationAlreadyComplete.as_str(),
                        "Registration already completed",
                    ),
                    _ => CommandError::infrastructure(
                        RegistrationErrorCode::RepositoryError.as_str(),
                        "Registration failed",
                    ),
                },
            }
        } else {
            CommandError::infrastructure(
                RegistrationErrorCode::RepositoryError.as_str(),
                error.to_string(),
            )
        }
    }
}

/// Command to complete user registration with username
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRegistrationCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// RSA-signed JWT registration token
    pub registration_token: String,
    /// Chosen username
    pub username: String,
}

impl CompleteRegistrationCommand {
    pub fn new(registration_token: String, username: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            registration_token,
            username,
        }
    }
}

impl Command for CompleteRegistrationCommand {
    type Result = CompleteRegistrationResponse;

    fn command_type(&self) -> &'static str {
        "complete_registration"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.registration_token.trim().is_empty() {
            return Err(CommandError::validation(
                RegistrationErrorCode::ValidationFailed.as_str(),
                "Registration token is required",
            ));
        }

        if self.username.trim().is_empty() {
            return Err(CommandError::validation(
                RegistrationErrorCode::ValidationFailed.as_str(),
                "Username is required",
            ));
        }

        if self.username.len() < 3 || self.username.len() > 50 {
            return Err(CommandError::validation(
                RegistrationErrorCode::ValidationFailed.as_str(),
                "Username must be between 3 and 50 characters",
            ));
        }

        if !self
            .username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(CommandError::validation(
                RegistrationErrorCode::ValidationFailed.as_str(),
                "Username can only contain letters, numbers, underscores, and hyphens",
            ));
        }

        // Require at least one letter (cannot be only numbers/symbols)
        if !self.username.chars().any(|c| c.is_alphabetic()) {
            return Err(CommandError::validation(
                RegistrationErrorCode::ValidationFailed.as_str(),
                "Username must contain at least one letter",
            ));
        }

        Ok(())
    }
}

/// Command to check username availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckUsernameCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Username to check
    pub username: String,
}

impl CheckUsernameCommand {
    pub fn new(username: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            username,
        }
    }
}

impl Command for CheckUsernameCommand {
    type Result = CheckUsernameResponse;

    fn command_type(&self) -> &'static str {
        "check_username"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.username.trim().is_empty() {
            return Err(CommandError::validation(
                RegistrationErrorCode::ValidationFailed.as_str(),
                "Username is required",
            ));
        }

        if self.username.len() > 50 {
            return Err(CommandError::validation(
                RegistrationErrorCode::ValidationFailed.as_str(),
                "Username cannot be longer than 50 characters",
            ));
        }

        Ok(())
    }
}

/// Complete registration command handler
pub struct CompleteRegistrationCommandHandler<R>
where
    R: RegistrationUseCase + ?Sized,
{
    registration_use_case: Arc<R>,
}

impl<R> CompleteRegistrationCommandHandler<R>
where
    R: RegistrationUseCase + ?Sized,
{
    pub fn new(registration_use_case: Arc<R>) -> Self {
        Self {
            registration_use_case,
        }
    }
}

#[async_trait]
impl<R> CommandHandler<CompleteRegistrationCommand> for CompleteRegistrationCommandHandler<R>
where
    R: RegistrationUseCase + Send + Sync + ?Sized,
{
    async fn handle(
        &self,
        command: CompleteRegistrationCommand,
    ) -> Result<CompleteRegistrationResponse, CommandError> {
        let request = CompleteRegistrationRequest {
            registration_token: command.registration_token,
            username: command.username,
        };

        self.registration_use_case
            .complete_registration(request)
            .await
            .map_err(|e| RegistrationErrorMapper.map_error(Box::new(e)))
    }
}

/// Check username command handler
pub struct CheckUsernameCommandHandler<R>
where
    R: RegistrationUseCase + ?Sized,
{
    registration_use_case: Arc<R>,
}

impl<R> CheckUsernameCommandHandler<R>
where
    R: RegistrationUseCase + ?Sized,
{
    pub fn new(registration_use_case: Arc<R>) -> Self {
        Self {
            registration_use_case,
        }
    }
}

#[async_trait]
impl<R> CommandHandler<CheckUsernameCommand> for CheckUsernameCommandHandler<R>
where
    R: RegistrationUseCase + Send + Sync + ?Sized,
{
    async fn handle(
        &self,
        command: CheckUsernameCommand,
    ) -> Result<CheckUsernameResponse, CommandError> {
        let request = CheckUsernameRequest {
            username: command.username,
        };

        self.registration_use_case
            .check_username(request)
            .await
            .map_err(|e| RegistrationErrorMapper.map_error(Box::new(e)))
    }
}
