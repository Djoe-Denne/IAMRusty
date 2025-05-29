use super::{Command, CommandError, CommandHandler};
use crate::usecase::user::{UserUseCase, UserProfile};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

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
        match self.user_use_case.get_user(command.user_id).await {
            Ok(profile) => Ok(profile),
            Err(_e) => {
                // Convert user errors to appropriate command errors
                Err(CommandError::Authentication("Authentication failed".to_string()))
            }
        }
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
        match self.user_use_case.validate_token(&command.token).await {
            Ok(user_id) => Ok(user_id),
            Err(_e) => {
                // Convert user errors to appropriate command errors  
                Err(CommandError::Authentication("Authentication failed".to_string()))
            }
        }
    }
} 