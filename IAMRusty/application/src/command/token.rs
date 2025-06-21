use crate::usecase::token::{RefreshTokenResponse, TokenError, TokenUseCase};
use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Error codes for token-related operations
#[derive(Debug, Clone)]
pub enum TokenErrorCode {
    RepositoryError,
    TokenServiceError,
    TokenNotFound,
    TokenInvalid,
    TokenExpired,
    AuthenticationFailed,
    ValidationFailed,
}

impl TokenErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RepositoryError => "repository_error",
            Self::TokenServiceError => "token_service_error",
            Self::TokenNotFound => "token_not_found",
            Self::TokenInvalid => "token_invalid",
            Self::TokenExpired => "token_expired",
            Self::AuthenticationFailed => "authentication_failed",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Error mapper for token-related commands
pub struct TokenErrorMapper;

impl CommandErrorMapper for TokenErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(token_error) = error.downcast_ref::<TokenError>() {
            match token_error {
                TokenError::DomainError(domain_error) => {
                    // Map domain errors to appropriate command errors
                    use domain::error::DomainError;
                    match domain_error {
                        DomainError::TokenNotFound => CommandError::authentication(
                            TokenErrorCode::TokenNotFound.as_str(),
                            "Authentication failed: Token not found",
                        ),
                        DomainError::TokenExpired => CommandError::authentication(
                            TokenErrorCode::TokenExpired.as_str(),
                            "Authentication failed: Token expired",
                        ),
                        DomainError::InvalidToken => CommandError::authentication(
                            TokenErrorCode::TokenInvalid.as_str(),
                            "Authentication failed: Invalid token",
                        ),
                        DomainError::TokenValidationFailed(_) => CommandError::authentication(
                            TokenErrorCode::TokenInvalid.as_str(),
                            "Authentication failed: Token validation failed",
                        ),
                        DomainError::RepositoryError(_) => CommandError::infrastructure(
                            TokenErrorCode::RepositoryError.as_str(),
                            "Repository error during token operation",
                        ),
                        _ => CommandError::infrastructure(
                            TokenErrorCode::TokenServiceError.as_str(),
                            "Token service error",
                        ),
                    }
                }
                TokenError::RepositoryError(_) => CommandError::infrastructure(
                    TokenErrorCode::RepositoryError.as_str(),
                    error.to_string(),
                ),
                TokenError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::business(
                            TokenErrorCode::AuthenticationFailed.as_str(),
                            format!("Authentication failed: {}", error_msg),
                        )
                    } else {
                        CommandError::infrastructure(
                            TokenErrorCode::TokenServiceError.as_str(),
                            error.to_string(),
                        )
                    }
                }
                // Authentication-related token errors should return 401
                TokenError::TokenNotFound => CommandError::authentication(
                    TokenErrorCode::TokenNotFound.as_str(),
                    "Authentication failed: Invalid refresh token",
                ),
                TokenError::TokenInvalid => CommandError::authentication(
                    TokenErrorCode::TokenInvalid.as_str(),
                    "Authentication failed: Invalid refresh token",
                ),
                TokenError::TokenExpired => CommandError::authentication(
                    TokenErrorCode::TokenExpired.as_str(),
                    "Authentication failed: Expired refresh token",
                ),
            }
        } else {
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::validation(
                    TokenErrorCode::AuthenticationFailed.as_str(),
                    format!("Authentication failed: {}", error_msg),
                )
            } else {
                CommandError::infrastructure(
                    TokenErrorCode::RepositoryError.as_str(),
                    error.to_string(),
                )
            }
        }
    }
}

impl TokenErrorMapper {
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
            refresh_token,
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
            return Err(CommandError::validation(
                TokenErrorCode::ValidationFailed.as_str(),
                "Refresh token cannot be empty",
            ));
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
            refresh_token,
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
            return Err(CommandError::validation(
                TokenErrorCode::ValidationFailed.as_str(),
                "Refresh token cannot be empty",
            ));
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
            user_id,
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

/// Get JWKS command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetJwksCommand {
    /// Command instance ID
    pub command_id: Uuid,
}

impl GetJwksCommand {
    /// Create a new get JWKS command
    pub fn new() -> Self {
        Self {
            command_id: Uuid::new_v4(),
        }
    }
}

impl Command for GetJwksCommand {
    type Result = domain::entity::token::JwkSet;

    fn command_type(&self) -> &'static str {
        "get_jwks"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // No validation needed for JWKS
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
        Self { token_use_case }
    }
}

#[async_trait]
impl<T> CommandHandler<RefreshTokenCommand> for RefreshTokenCommandHandler<T>
where
    T: TokenUseCase + Send + Sync + ?Sized,
{
    async fn handle(
        &self,
        command: RefreshTokenCommand,
    ) -> Result<RefreshTokenResponse, CommandError> {
        self.token_use_case
            .refresh_token(command.refresh_token)
            .await
            .map_err(|e| TokenErrorMapper.map_error(Box::new(e)))
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
        Self { token_use_case }
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
            .map_err(|e| TokenErrorMapper.map_error(Box::new(e)))
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
        Self { token_use_case }
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
            .map_err(|e| TokenErrorMapper.map_error(Box::new(e)))
    }
}

/// Get JWKS command handler
pub struct GetJwksCommandHandler<T>
where
    T: TokenUseCase + ?Sized,
{
    token_use_case: Arc<T>,
}

impl<T> GetJwksCommandHandler<T>
where
    T: TokenUseCase + ?Sized,
{
    /// Create a new get JWKS command handler
    pub fn new(token_use_case: Arc<T>) -> Self {
        Self { token_use_case }
    }
}

#[async_trait]
impl<T> CommandHandler<GetJwksCommand> for GetJwksCommandHandler<T>
where
    T: TokenUseCase + Send + Sync + ?Sized,
{
    async fn handle(
        &self,
        _command: GetJwksCommand,
    ) -> Result<domain::entity::token::JwkSet, CommandError> {
        // Get JWKS is synchronous, so we can call it directly
        Ok(self.token_use_case.get_jwks())
    }
}
