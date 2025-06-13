use rustycog_command::{Command, CommandError, CommandHandler, CommandErrorMapper};
use crate::usecase::{
    oauth::{OAuthUseCase, OAuthError},
};
use domain::entity::provider::Provider;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Error codes for OAuth login-related operations
#[derive(Debug, Clone)]
pub enum OAuthLoginErrorCode {
    AuthenticationFailed,
    TokenExpired,
    InvalidToken,
    ProviderError,
    DatabaseError,
    TokenServiceError,
    ValidationFailed,
}

impl OAuthLoginErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AuthenticationFailed => "authentication_failed",
            Self::TokenExpired => "token_expired", 
            Self::InvalidToken => "invalid_token",
            Self::ProviderError => "provider_error",
            Self::DatabaseError => "database_error",
            Self::TokenServiceError => "token_service_error",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Error mapper for OAuth login-related commands
pub struct OAuthLoginErrorMapper;

impl CommandErrorMapper for OAuthLoginErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        // Try to downcast to known error types
        if let Some(oauth_error) = error.downcast_ref::<OAuthError>() {
            match oauth_error {
                OAuthError::AuthError(_) => CommandError::business(
                    OAuthLoginErrorCode::AuthenticationFailed.as_str(),
                    "OAuth authentication failed"
                ),
                OAuthError::DbError(_) => CommandError::infrastructure(
                    OAuthLoginErrorCode::DatabaseError.as_str(),
                    "Database error during OAuth flow"
                ),
                OAuthError::TokenError(_) => CommandError::infrastructure(
                    OAuthLoginErrorCode::TokenServiceError.as_str(),
                    "Token service error during OAuth flow"
                ),
                OAuthError::EventPublishingError(_) => CommandError::infrastructure(
                    OAuthLoginErrorCode::ProviderError.as_str(),
                    "Event publishing error during OAuth flow"
                ),
                OAuthError::ProviderNotConfigured(_) => CommandError::infrastructure(
                    OAuthLoginErrorCode::ProviderError.as_str(),
                    "OAuth provider not configured"
                ),
            }
        } else {
            // Check if it's an authentication-related error by message
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::business(
                    OAuthLoginErrorCode::AuthenticationFailed.as_str(),
                    format!("OAuth authentication failed: {}", error_msg)
                )
            } else {
                CommandError::infrastructure(
                    OAuthLoginErrorCode::ProviderError.as_str(),
                    error.to_string()
                )
            }
        }
    }
}

impl OAuthLoginErrorMapper {
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

/// OAuth login command
#[derive(Debug, Clone)]
pub struct OAuthLoginCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
    /// Authorization code from OAuth callback
    pub code: String,
    /// Redirect URI used in OAuth flow
    pub redirect_uri: String,
}

impl OAuthLoginCommand {
    /// Create a new OAuth login command
    pub fn new(provider: Provider, code: String, redirect_uri: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            provider,
            code,
            redirect_uri,
        }
    }
}

#[async_trait]
impl Command for OAuthLoginCommand {
    type Result = crate::usecase::oauth::OAuthResult;
    
    fn command_type(&self) -> &'static str {
        "oauth_login"
    }
    
    fn command_id(&self) -> Uuid {
        self.command_id
    }
    
    fn validate(&self) -> Result<(), CommandError> {
        if self.code.trim().is_empty() {
            return Err(CommandError::validation(
                OAuthLoginErrorCode::ValidationFailed.as_str(),
                "Authorization code cannot be empty"
            ));
        }
        
        if self.redirect_uri.trim().is_empty() {
            return Err(CommandError::validation(
                OAuthLoginErrorCode::ValidationFailed.as_str(),
                "Redirect URI cannot be empty"
            ));
        }
        
        // Basic URL validation for redirect_uri
        if !self.redirect_uri.starts_with("http://") && !self.redirect_uri.starts_with("https://") {
            return Err(CommandError::validation(
                OAuthLoginErrorCode::ValidationFailed.as_str(),
                "Redirect URI must be a valid HTTP/HTTPS URL"
            ));
        }
        
        Ok(())
    }
}

/// OAuth login command handler
pub struct OAuthLoginCommandHandler<O> 
where
    O: OAuthUseCase + ?Sized,
{
    oauth_use_case: Arc<O>,
}

impl<O> OAuthLoginCommandHandler<O>
where
    O: OAuthUseCase + ?Sized,
{
    /// Create a new OAuth login command handler
    pub fn new(oauth_use_case: Arc<O>) -> Self {
        Self {
            oauth_use_case,
        }
    }
}

#[async_trait]
impl<O> CommandHandler<OAuthLoginCommand> for OAuthLoginCommandHandler<O>
where
    O: OAuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: OAuthLoginCommand) -> Result<crate::usecase::oauth::OAuthResult, CommandError> {
        self.oauth_use_case
            .oauth_login(command.provider, command.code, command.redirect_uri)
            .await
            .map_err(|e| OAuthLoginErrorMapper.map_error(Box::new(e)))
    }
}

/// Generate OAuth start URL command
#[derive(Debug, Clone)]
pub struct GenerateOAuthStartUrlCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
}

impl GenerateOAuthStartUrlCommand {
    /// Create a new generate OAuth start URL command
    pub fn new(provider: Provider) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            provider,
        }
    }
}

#[async_trait]
impl Command for GenerateOAuthStartUrlCommand {
    type Result = String;
    
    fn command_type(&self) -> &'static str {
        "generate_oauth_start_url"
    }
    
    fn command_id(&self) -> Uuid {
        self.command_id
    }
    
    fn validate(&self) -> Result<(), CommandError> {
        // Provider validation is handled by the enum itself
        Ok(())
    }
}

/// Generate OAuth start URL command handler
pub struct GenerateOAuthStartUrlCommandHandler<O> 
where
    O: OAuthUseCase + ?Sized,
{
    oauth_use_case: Arc<O>,
}

impl<O> GenerateOAuthStartUrlCommandHandler<O>
where
    O: OAuthUseCase + ?Sized,
{
    /// Create a new generate OAuth start URL command handler
    pub fn new(oauth_use_case: Arc<O>) -> Self {
        Self {
            oauth_use_case,
        }
    }
}

#[async_trait]
impl<O> CommandHandler<GenerateOAuthStartUrlCommand> for GenerateOAuthStartUrlCommandHandler<O>
where
    O: OAuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: GenerateOAuthStartUrlCommand) -> Result<String, CommandError> {
        self.oauth_use_case
            .generate_start_url(command.provider)
            .map_err(|e| OAuthLoginErrorMapper.map_error(Box::new(e)))
    }
}
