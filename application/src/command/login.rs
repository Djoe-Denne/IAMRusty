use super::{Command, CommandError, CommandHandler};
use super::registry::CommandErrorMapper;
use crate::usecase::{
    login::{LoginUseCase, LoginResponse, LoginError},
};
use domain::entity::provider::Provider;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Error mapper for login-related commands
pub struct LoginErrorMapper;

impl CommandErrorMapper for LoginErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        // Try to downcast to known error types
        if let Some(login_error) = error.downcast_ref::<LoginError>() {
            match login_error {
                LoginError::AuthError(msg) => CommandError::Business(format!("Authentication failed: {}", msg)),
                LoginError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e)),
                LoginError::TokenError(e) => CommandError::Infrastructure(format!("Token service error: {}", e)),
            }
        } else {
            // Check if it's an authentication-related error by message
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::Business(format!("Authentication failed: {}", error_msg))
            } else {
                CommandError::Infrastructure(error.to_string())
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

// Inventory-based command registration for zero-boilerplate plugin system
use super::registry::{CommandRegistration, CommandHandlerWrapper};

inventory::submit! {
    CommandRegistration {
        command_name: "login",
        handler_factory: |container| {
            // Extract the login use case dependency
            let login_use_case = container
                .get_dependency("LoginUseCase")
                .and_then(|dep| dep.downcast::<Arc<dyn crate::usecase::login::LoginUseCase>>().ok())
                .map(|boxed| *boxed)
                .expect("LoginUseCase dependency not found");
            
            // Create the command handler wrapper
            Arc::new(CommandHandlerWrapper::new(
                Arc::new(LoginCommandHandler::new(login_use_case)),
                Arc::new(LoginErrorMapper),
            ))
        },
        error_mapper_factory: || Arc::new(LoginErrorMapper),
    }
}

inventory::submit! {
    CommandRegistration {
        command_name: "generate_login_start_url",
        handler_factory: |container| {
            let login_use_case = container
                .get_dependency("LoginUseCase")
                .and_then(|dep| dep.downcast::<Arc<dyn crate::usecase::login::LoginUseCase>>().ok())
                .map(|boxed| *boxed)
                .expect("LoginUseCase dependency not found");
            
            Arc::new(CommandHandlerWrapper::new(
                Arc::new(GenerateLoginStartUrlCommandHandler::new(login_use_case)),
                Arc::new(LoginErrorMapper),
            ))
        },
        error_mapper_factory: || Arc::new(LoginErrorMapper),
    }
} 