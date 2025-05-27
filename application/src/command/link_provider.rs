use super::{Command, CommandError, CommandHandler};
use crate::usecase::link_provider::{LinkProviderUseCase, LinkProviderError, LinkProviderResponse};
use domain::entity::provider::Provider;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Link provider command
#[derive(Debug, Clone)]
pub struct LinkProviderCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// User ID to link the provider to
    pub user_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
    /// Authorization code from OAuth callback
    pub code: String,
    /// Redirect URI used in OAuth flow
    pub redirect_uri: String,
}

impl LinkProviderCommand {
    /// Create a new link provider command
    pub fn new(
        user_id: Uuid,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            user_id,
            provider,
            code,
            redirect_uri,
        }
    }
}

#[async_trait]
impl Command for LinkProviderCommand {
    type Result = LinkProviderResponse;
    
    fn command_type(&self) -> &'static str {
        "link_provider"
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
        
        // Validate user_id is not nil
        if self.user_id.is_nil() {
            return Err(CommandError::Validation(
                "User ID cannot be nil".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Link provider command handler
pub struct LinkProviderCommandHandler<L> 
where
    L: LinkProviderUseCase + ?Sized,
{
    link_provider_use_case: Arc<L>,
}

impl<L> LinkProviderCommandHandler<L>
where
    L: LinkProviderUseCase + ?Sized,
{
    /// Create a new link provider command handler
    pub fn new(link_provider_use_case: Arc<L>) -> Self {
        Self {
            link_provider_use_case,
        }
    }
}

#[async_trait]
impl<L> CommandHandler<LinkProviderCommand> for LinkProviderCommandHandler<L>
where
    L: LinkProviderUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: LinkProviderCommand) -> Result<LinkProviderResponse, CommandError> {
        self.link_provider_use_case
            .link_provider(
                command.user_id,
                command.provider,
                command.code,
                command.redirect_uri,
            )
            .await
            .map_err(|e| match e {
                LinkProviderError::AuthError(msg) => {
                    CommandError::Business(format!("Authentication failed: {}", msg))
                }
                LinkProviderError::DbError(e) => {
                    CommandError::Infrastructure(format!("Database error: {}", e))
                }
                LinkProviderError::TokenError(e) => {
                    CommandError::Infrastructure(format!("Token service error: {}", e))
                }
                LinkProviderError::UserNotFound => {
                    CommandError::Business("User not found".to_string())
                }
                LinkProviderError::ProviderAlreadyLinked => {
                    CommandError::Business("Provider account is already linked to another user".to_string())
                }
                LinkProviderError::ProviderAlreadyLinkedToSameUser => {
                    CommandError::Business("Provider is already linked to your account".to_string())
                }
            })
    }
}

/// Generate OAuth start URL for linking command
#[derive(Debug, Clone)]
pub struct GenerateLinkProviderStartUrlCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
}

impl GenerateLinkProviderStartUrlCommand {
    /// Create a new generate link provider start URL command
    pub fn new(provider: Provider) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            provider,
        }
    }
}

#[async_trait]
impl Command for GenerateLinkProviderStartUrlCommand {
    type Result = String;
    
    fn command_type(&self) -> &'static str {
        "generate_link_provider_start_url"
    }
    
    fn command_id(&self) -> Uuid {
        self.command_id
    }
    
    fn validate(&self) -> Result<(), CommandError> {
        // Provider validation is handled by the enum itself
        Ok(())
    }
}

/// Generate link provider start URL command handler
pub struct GenerateLinkProviderStartUrlCommandHandler<L> 
where
    L: LinkProviderUseCase + ?Sized,
{
    link_provider_use_case: Arc<L>,
}

impl<L> GenerateLinkProviderStartUrlCommandHandler<L>
where
    L: LinkProviderUseCase + ?Sized,
{
    /// Create a new generate link provider start URL command handler
    pub fn new(link_provider_use_case: Arc<L>) -> Self {
        Self {
            link_provider_use_case,
        }
    }
}

#[async_trait]
impl<L> CommandHandler<GenerateLinkProviderStartUrlCommand> for GenerateLinkProviderStartUrlCommandHandler<L>
where
    L: LinkProviderUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: GenerateLinkProviderStartUrlCommand) -> Result<String, CommandError> {
        self.link_provider_use_case
            .generate_start_url(command.provider)
            .map_err(|e| match e {
                LinkProviderError::AuthError(msg) => CommandError::Business(format!("Authentication error: {}", msg)),
                LinkProviderError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e)),
                LinkProviderError::TokenError(e) => CommandError::Infrastructure(format!("Token service error: {}", e)),
                LinkProviderError::UserNotFound => CommandError::Business("User not found".to_string()),
                LinkProviderError::ProviderAlreadyLinked => CommandError::Business("Provider account is already linked to another user".to_string()),
                LinkProviderError::ProviderAlreadyLinkedToSameUser => CommandError::Business("Provider is already linked to your account".to_string()),
            })
    }
} 