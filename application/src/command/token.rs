use super::{Command, CommandError, CommandHandler, error_mapping::ErrorMapping};
use crate::usecase::token::{TokenUseCase, TokenError, RefreshTokenResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Refresh token command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Refresh token to use
    pub refresh_token: String,
}

impl RefreshTokenCommand {
    /// Create a new refresh token command
    pub fn new(refresh_token: String) -> Self {
        Self { 
            command_id: Uuid::new_v4(),
            refresh_token 
        }
    }
}

impl Command for RefreshTokenCommand {
    type Result = RefreshTokenResponse;

    fn command_type(&self) -> &'static str {
        "refresh_token"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.refresh_token.trim().is_empty() {
            return Err(CommandError::Validation("Refresh token cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// Revoke token command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeTokenCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Refresh token to revoke
    pub refresh_token: String,
}

impl RevokeTokenCommand {
    /// Create a new revoke token command
    pub fn new(refresh_token: String) -> Self {
        Self { 
            command_id: Uuid::new_v4(),
            refresh_token 
        }
    }
}

impl Command for RevokeTokenCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "revoke_token"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.refresh_token.trim().is_empty() {
            return Err(CommandError::Validation("Refresh token cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// Revoke all tokens command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeAllTokensCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// User ID whose tokens to revoke
    pub user_id: Uuid,
}

impl RevokeAllTokensCommand {
    /// Create a new revoke all tokens command
    pub fn new(user_id: Uuid) -> Self {
        Self { 
            command_id: Uuid::new_v4(),
            user_id 
        }
    }
}

impl Command for RevokeAllTokensCommand {
    type Result = u64;

    fn command_type(&self) -> &'static str {
        "revoke_all_tokens"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // UUID validation is handled by the type system
        Ok(())
    }
}

/// Refresh token command handler
pub struct RefreshTokenCommandHandler<T> 
where
    T: TokenUseCase + ?Sized,
{
    token_use_case: Arc<T>,
}

impl<T> RefreshTokenCommandHandler<T>
where
    T: TokenUseCase + ?Sized,
{
    /// Create a new refresh token command handler
    pub fn new(token_use_case: Arc<T>) -> Self {
        Self {
            token_use_case,
        }
    }
}

#[async_trait]
impl<T> CommandHandler<RefreshTokenCommand> for RefreshTokenCommandHandler<T>
where
    T: TokenUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: RefreshTokenCommand) -> Result<RefreshTokenResponse, CommandError> {
        self.token_use_case
            .refresh_token(command.refresh_token)
            .await
            .map_err(ErrorMapping::map_token_error)
    }
}

/// Revoke token command handler
pub struct RevokeTokenCommandHandler<T> 
where
    T: TokenUseCase + ?Sized,
{
    token_use_case: Arc<T>,
}

impl<T> RevokeTokenCommandHandler<T>
where
    T: TokenUseCase + ?Sized,
{
    /// Create a new revoke token command handler
    pub fn new(token_use_case: Arc<T>) -> Self {
        Self {
            token_use_case,
        }
    }
}

#[async_trait]
impl<T> CommandHandler<RevokeTokenCommand> for RevokeTokenCommandHandler<T>
where
    T: TokenUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: RevokeTokenCommand) -> Result<(), CommandError> {
        self.token_use_case
            .revoke_token(command.refresh_token)
            .await
            .map_err(ErrorMapping::map_token_error)
    }
}

/// Revoke all tokens command handler
pub struct RevokeAllTokensCommandHandler<T> 
where
    T: TokenUseCase + ?Sized,
{
    token_use_case: Arc<T>,
}

impl<T> RevokeAllTokensCommandHandler<T>
where
    T: TokenUseCase + ?Sized,
{
    /// Create a new revoke all tokens command handler
    pub fn new(token_use_case: Arc<T>) -> Self {
        Self {
            token_use_case,
        }
    }
}

#[async_trait]
impl<T> CommandHandler<RevokeAllTokensCommand> for RevokeAllTokensCommandHandler<T>
where
    T: TokenUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: RevokeAllTokensCommand) -> Result<u64, CommandError> {
        self.token_use_case
            .revoke_all_tokens(command.user_id)
            .await
            .map_err(ErrorMapping::map_token_error)
    }
} 