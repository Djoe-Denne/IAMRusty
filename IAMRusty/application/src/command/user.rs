use crate::usecase::user::{UserError, UserProfile, UserUseCase};
use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Error codes for user-related operations
#[derive(Debug, Clone)]
pub enum UserErrorCode {
    RepositoryError,
    TokenServiceError,
    UserNotFound,
    InvalidToken,
    TokenExpired,
    AuthenticationFailed,
    ValidationFailed,
}

impl UserErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RepositoryError => "repository_error",
            Self::TokenServiceError => "token_service_error",
            Self::UserNotFound => "user_not_found",
            Self::InvalidToken => "invalid_token",
            Self::TokenExpired => "token_expired",
            Self::AuthenticationFailed => "authentication_failed",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Error mapper for user-related commands
pub struct UserErrorMapper;

impl CommandErrorMapper for UserErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(user_error) = error.downcast_ref::<UserError>() {
            match user_error {
                UserError::DomainError(domain_error) => {
                    // Map domain errors to appropriate command errors
                    use iam_domain::error::DomainError;
                    match domain_error {
                        DomainError::UserNotFound => CommandError::authentication(
                            UserErrorCode::UserNotFound.as_str(),
                            "Authentication failed: User not found",
                        ),
                        DomainError::TokenValidationFailed(_) => CommandError::authentication(
                            UserErrorCode::InvalidToken.as_str(),
                            "Authentication failed: Token validation failed",
                        ),
                        DomainError::TokenExpired => CommandError::authentication(
                            UserErrorCode::TokenExpired.as_str(),
                            "Authentication failed: Token expired",
                        ),
                        DomainError::InvalidToken => CommandError::authentication(
                            UserErrorCode::InvalidToken.as_str(),
                            "Authentication failed: Invalid token",
                        ),
                        DomainError::RepositoryError(_) => CommandError::infrastructure(
                            UserErrorCode::RepositoryError.as_str(),
                            "Repository error during user operation",
                        ),
                        _ => CommandError::infrastructure(
                            UserErrorCode::TokenServiceError.as_str(),
                            "User service error",
                        ),
                    }
                }
                UserError::RepositoryError(_) => CommandError::infrastructure(
                    UserErrorCode::RepositoryError.as_str(),
                    error.to_string(),
                ),
                UserError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::validation(
                            UserErrorCode::AuthenticationFailed.as_str(),
                            format!("Authentication failed: {}", error_msg),
                        )
                    } else {
                        CommandError::infrastructure(
                            UserErrorCode::TokenServiceError.as_str(),
                            error.to_string(),
                        )
                    }
                }
                UserError::UserNotFound => CommandError::authentication(
                    UserErrorCode::UserNotFound.as_str(),
                    "Authentication failed",
                ),
                UserError::InvalidToken => CommandError::authentication(
                    UserErrorCode::InvalidToken.as_str(),
                    "Authentication failed",
                ),
                UserError::TokenExpired => CommandError::authentication(
                    UserErrorCode::TokenExpired.as_str(),
                    "Authentication failed",
                ),
            }
        } else {
            CommandError::authentication(
                UserErrorCode::AuthenticationFailed.as_str(),
                "Authentication failed",
            )
        }
    }
}

impl UserErrorMapper {
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

/// Get user command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// User ID to retrieve
    pub user_id: Uuid,
}

impl GetUserCommand {
    /// Create a new get user command
    pub fn new(user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            user_id,
        }
    }
}

impl Command for GetUserCommand {
    type Result = UserProfile;

    fn command_type(&self) -> &'static str {
        "get_user"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // UUID validation is handled by the type system
        Ok(())
    }
}

/// Get user command handler
pub struct GetUserCommandHandler<U>
where
    U: UserUseCase + ?Sized,
{
    user_use_case: Arc<U>,
}

impl<U> GetUserCommandHandler<U>
where
    U: UserUseCase + ?Sized,
{
    /// Create a new get user command handler
    pub fn new(user_use_case: Arc<U>) -> Self {
        Self { user_use_case }
    }
}

#[async_trait]
impl<U> CommandHandler<GetUserCommand> for GetUserCommandHandler<U>
where
    U: UserUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: GetUserCommand) -> Result<UserProfile, CommandError> {
        self.user_use_case
            .get_user(command.user_id)
            .await
            .map_err(|e| UserErrorMapper.map_error(Box::new(e)))
    }
}
