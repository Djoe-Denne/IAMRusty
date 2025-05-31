use super::{Command, CommandError, CommandHandler};
use super::registry::CommandErrorMapper;
use crate::usecase::user::{UserUseCase, UserProfile, UserError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Error mapper for user-related commands
pub struct UserErrorMapper;

impl CommandErrorMapper for UserErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(user_error) = error.downcast_ref::<UserError>() {
            match user_error {
                UserError::RepositoryError(_) => CommandError::Infrastructure(error.to_string()),
                UserError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::Validation(format!("Authentication failed: {}", error_msg))
                    } else {
                        CommandError::Infrastructure(error.to_string())
                    }
                },
                _ => CommandError::Authentication("Authentication failed".to_string()),
            }
        } else {
            CommandError::Authentication("Authentication failed".to_string())
        }
    }
}

impl UserErrorMapper {
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
            user_id 
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

/// Validate token command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateTokenCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Token to validate
    pub token: String,
}

impl ValidateTokenCommand {
    /// Create a new validate token command
    pub fn new(token: String) -> Self {
        Self { 
            command_id: Uuid::new_v4(),
            token 
        }
    }
}

impl Command for ValidateTokenCommand {
    type Result = Uuid;

    fn command_type(&self) -> &'static str {
        "validate_token"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.token.trim().is_empty() {
            return Err(CommandError::Validation("Token cannot be empty".to_string()));
        }
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
        Self {
            user_use_case,
        }
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

/// Validate token command handler
pub struct ValidateTokenCommandHandler<U> 
where
    U: UserUseCase + ?Sized,
{
    user_use_case: Arc<U>,
}

impl<U> ValidateTokenCommandHandler<U>
where
    U: UserUseCase + ?Sized,
{
    /// Create a new validate token command handler
    pub fn new(user_use_case: Arc<U>) -> Self {
        Self {
            user_use_case,
        }
    }
}

#[async_trait]
impl<U> CommandHandler<ValidateTokenCommand> for ValidateTokenCommandHandler<U>
where
    U: UserUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: ValidateTokenCommand) -> Result<Uuid, CommandError> {
        self.user_use_case
            .validate_token(&command.token)
            .await
            .map_err(|e| UserErrorMapper.map_error(Box::new(e)))
    }
}
