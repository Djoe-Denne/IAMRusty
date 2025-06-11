use rustycog_command::{Command, CommandError, CommandHandler, CommandErrorMapper};
use crate::usecase::{
    login::{LoginUseCase, LoginResponse, LoginError},
};
use domain::entity::provider::Provider;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Error codes for login-related operations
#[derive(Debug, Clone)]
pub enum LoginErrorCode {
    AuthenticationFailed,
    TokenExpired,
    InvalidToken,
    ProviderError,
    DatabaseError,
    TokenServiceError,
    ValidationFailed,
}

impl LoginErrorCode {
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

/// Error mapper for login-related commands
pub struct LoginErrorMapper;

impl CommandErrorMapper for LoginErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        // Try to downcast to known error types
        if let Some(login_error) = error.downcast_ref::<LoginError>() {
            match login_error {
                LoginError::AuthError(msg) => CommandError::business(
                    LoginErrorCode::AuthenticationFailed.as_str(),
                    format!("Authentication failed: {}", msg)
                ),
                LoginError::DbError(e) => CommandError::infrastructure(
                    LoginErrorCode::DatabaseError.as_str(),
                    format!("Database error: {}", e)
                ),
                LoginError::TokenError(e) => CommandError::infrastructure(
                    LoginErrorCode::TokenServiceError.as_str(),
                    format!("Token service error: {}", e)
                ),
            }
        } else {
            // Check if it's an authentication-related error by message
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::business(
                    LoginErrorCode::AuthenticationFailed.as_str(),
                    format!("Authentication failed: {}", error_msg)
                )
            } else {
                CommandError::infrastructure(
                    LoginErrorCode::ProviderError.as_str(),
                    error.to_string()
                )
            }
        }
    }
}

impl LoginErrorMapper {
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

/// Login command
#[derive(Debug, Clone)]
pub struct LoginCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
    /// Authorization code from OAuth callback
    pub code: String,
    /// Redirect URI used in OAuth flow
    pub redirect_uri: String,
}

impl LoginCommand {
    /// Create a new login command
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
impl Command for LoginCommand {
    type Result = LoginResponse;
    
    fn command_type(&self) -> &'static str {
        "login"
    }
    
    fn command_id(&self) -> Uuid {
        self.command_id
    }
    
    fn validate(&self) -> Result<(), CommandError> {
        if self.code.trim().is_empty() {
            return Err(CommandError::validation(
                LoginErrorCode::ValidationFailed.as_str(),
                "Authorization code cannot be empty"
            ));
        }
        
        if self.redirect_uri.trim().is_empty() {
            return Err(CommandError::validation(
                LoginErrorCode::ValidationFailed.as_str(),
                "Redirect URI cannot be empty"
            ));
        }
        
        // Basic URL validation for redirect_uri
        if !self.redirect_uri.starts_with("http://") && !self.redirect_uri.starts_with("https://") {
            return Err(CommandError::validation(
                LoginErrorCode::ValidationFailed.as_str(),
                "Redirect URI must be a valid HTTP/HTTPS URL"
            ));
        }
        
        Ok(())
    }
}

/// Login command handler
pub struct LoginCommandHandler<L> 
where
    L: LoginUseCase + ?Sized,
{
    login_use_case: Arc<L>,
}

impl<L> LoginCommandHandler<L>
where
    L: LoginUseCase + ?Sized,
{
    /// Create a new login command handler
    pub fn new(login_use_case: Arc<L>) -> Self {
        Self {
            login_use_case,
        }
    }
}

#[async_trait]
impl<L> CommandHandler<LoginCommand> for LoginCommandHandler<L>
where
    L: LoginUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: LoginCommand) -> Result<LoginResponse, CommandError> {
        self.login_use_case
            .login(command.provider, command.code, command.redirect_uri)
            .await
            .map_err(|e| LoginErrorMapper.map_error(Box::new(e)))
    }
}

/// Generate OAuth start URL command
#[derive(Debug, Clone)]
pub struct GenerateLoginStartUrlCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
}

impl GenerateLoginStartUrlCommand {
    /// Create a new generate login start URL command
    pub fn new(provider: Provider) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            provider,
        }
    }
}

#[async_trait]
impl Command for GenerateLoginStartUrlCommand {
    type Result = String;
    
    fn command_type(&self) -> &'static str {
        "generate_login_start_url"
    }
    
    fn command_id(&self) -> Uuid {
        self.command_id
    }
    
    fn validate(&self) -> Result<(), CommandError> {
        // Provider validation is handled by the enum itself
        Ok(())
    }
}

/// Generate login start URL command handler
pub struct GenerateLoginStartUrlCommandHandler<L> 
where
    L: LoginUseCase + ?Sized,
{
    login_use_case: Arc<L>,
}

impl<L> GenerateLoginStartUrlCommandHandler<L>
where
    L: LoginUseCase + ?Sized,
{
    /// Create a new generate login start URL command handler
    pub fn new(login_use_case: Arc<L>) -> Self {
        Self {
            login_use_case,
        }
    }
}

#[async_trait]
impl<L> CommandHandler<GenerateLoginStartUrlCommand> for GenerateLoginStartUrlCommandHandler<L>
where
    L: LoginUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: GenerateLoginStartUrlCommand) -> Result<String, CommandError> {
        self.login_use_case
            .generate_start_url(command.provider)
            .map_err(|e| LoginErrorMapper.map_error(Box::new(e)))
    }
}
