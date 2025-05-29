use super::{Command, CommandError, CommandHandler, error_mapping::ErrorMapping};
use crate::usecase::login::{LoginUseCase, LoginResponse};
use domain::entity::provider::Provider;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

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
            return Err(CommandError::Validation(
                "Authorization code cannot be empty".to_string()
            ));
        }
        
        if self.redirect_uri.trim().is_empty() {
            return Err(CommandError::Validation(
                "Redirect URI cannot be empty".to_string()
            ));
        }
        
        // Basic URL validation for redirect_uri
        if !self.redirect_uri.starts_with("http://") && !self.redirect_uri.starts_with("https://") {
            return Err(CommandError::Validation(
                "Redirect URI must be a valid HTTP/HTTPS URL".to_string()
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
            .map_err(ErrorMapping::map_login_error)
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
            .map_err(ErrorMapping::map_login_error)
    }
} 