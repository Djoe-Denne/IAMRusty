use crate::usecase::{
    link_provider::{LinkProviderError, LinkProviderResponse, LinkProviderUseCase},
    provider::{ProviderError, ProviderTokenResponse, ProviderUseCase},
};
use async_trait::async_trait;
use domain::entity::provider::Provider;
use domain::error::DomainError;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

/// Error codes for link provider operations
#[derive(Debug, Clone)]
pub enum LinkProviderErrorCode {
    AuthenticationFailed,
    UserNotFound,
    ProviderAlreadyLinkedSameUser,
    ProviderAlreadyLinked,
    BusinessRuleViolation,
    ProviderNotConfigured,
    RepositoryError,
    ValidationFailed,
}

impl LinkProviderErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AuthenticationFailed => "authentication_failed",
            Self::UserNotFound => "user_not_found",
            Self::ProviderAlreadyLinkedSameUser => "provider_already_linked_same_user",
            Self::ProviderAlreadyLinked => "provider_already_linked",
            Self::BusinessRuleViolation => "business_rule_violation",
            Self::ProviderNotConfigured => "provider_not_configured",
            Self::RepositoryError => "repository_error",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Error codes for provider token operations
#[derive(Debug, Clone)]
pub enum ProviderErrorCode {
    UserNotFound,
    ProviderNotSupported,
    NoTokenForProvider,
    AuthenticationFailed,
    DatabaseError,
    ValidationFailed,
}

impl ProviderErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserNotFound => "user_not_found",
            Self::ProviderNotSupported => "provider_not_supported",
            Self::NoTokenForProvider => "no_token_for_provider",
            Self::AuthenticationFailed => "authentication_failed",
            Self::DatabaseError => "database_error",
            Self::ValidationFailed => "validation_failed",
        }
    }
}

/// Error mapper for link provider commands
pub struct LinkProviderErrorMapper;

impl CommandErrorMapper for LinkProviderErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(link_error) = error.downcast_ref::<LinkProviderError>() {
            match link_error {
                LinkProviderError::AuthError(_msg) => CommandError::authentication(
                    LinkProviderErrorCode::AuthenticationFailed.as_str(),
                    "Authentication failed",
                ),
                LinkProviderError::DomainError(domain_error) => {
                    match domain_error {
                        DomainError::UserNotFound => CommandError::authentication(
                            LinkProviderErrorCode::UserNotFound.as_str(),
                            "Authentication failed",
                        ),
                        DomainError::BusinessRuleViolation(msg) => {
                            //already linked to another user
                            if msg.contains("already associated with another user") {
                                CommandError::business(
                                    LinkProviderErrorCode::ProviderAlreadyLinked.as_str(),
                                    msg.clone(),
                                )
                            } else if msg.contains("already linked to your account") {
                                CommandError::business(
                                    LinkProviderErrorCode::ProviderAlreadyLinkedSameUser.as_str(),
                                    msg.clone(),
                                )
                            } else {
                                CommandError::business(
                                    LinkProviderErrorCode::BusinessRuleViolation.as_str(),
                                    msg.clone(),
                                )
                            }
                        }
                        DomainError::RepositoryError(msg) => CommandError::infrastructure(
                            LinkProviderErrorCode::RepositoryError.as_str(),
                            format!("Database error: {}", msg),
                        ),
                        _ => CommandError::infrastructure(
                            LinkProviderErrorCode::RepositoryError.as_str(),
                            domain_error.to_string(),
                        ),
                    }
                }
                LinkProviderError::ProviderNotConfigured(provider) => CommandError::infrastructure(
                    LinkProviderErrorCode::ProviderNotConfigured.as_str(),
                    format!("Provider {} not configured", provider),
                ),
            }
        } else {
            CommandError::infrastructure(
                LinkProviderErrorCode::RepositoryError.as_str(),
                error.to_string(),
            )
        }
    }
}

/// Error mapper for provider token commands
pub struct ProviderErrorMapper;

impl CommandErrorMapper for ProviderErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(provider_error) = error.downcast_ref::<ProviderError>() {
            match provider_error {
                ProviderError::UserNotFound => CommandError::authentication(
                    ProviderErrorCode::UserNotFound.as_str(),
                    "Authentication failed",
                ),
                ProviderError::ProviderNotSupported(provider) => CommandError::validation(
                    ProviderErrorCode::ProviderNotSupported.as_str(),
                    format!("Unsupported provider: {}", provider),
                ),
                ProviderError::NoTokenForProvider => CommandError::business(
                    ProviderErrorCode::NoTokenForProvider.as_str(),
                    "No token available for the user and provider",
                ),
                ProviderError::AuthError(msg) => CommandError::authentication(
                    ProviderErrorCode::AuthenticationFailed.as_str(),
                    msg.clone(),
                ),
                ProviderError::DbError(e) => CommandError::infrastructure(
                    ProviderErrorCode::DatabaseError.as_str(),
                    format!("Database error: {}", e),
                ),
            }
        } else {
            CommandError::infrastructure(
                ProviderErrorCode::DatabaseError.as_str(),
                error.to_string(),
            )
        }
    }
}

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
    pub fn new(user_id: Uuid, provider: Provider, code: String, redirect_uri: String) -> Self {
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
            return Err(CommandError::validation(
                LinkProviderErrorCode::ValidationFailed.as_str(),
                "Authorization code cannot be empty",
            ));
        }

        if self.redirect_uri.trim().is_empty() {
            return Err(CommandError::validation(
                LinkProviderErrorCode::ValidationFailed.as_str(),
                "Redirect URI cannot be empty",
            ));
        }

        // Basic URL validation for redirect_uri
        if !self.redirect_uri.starts_with("http://") && !self.redirect_uri.starts_with("https://") {
            return Err(CommandError::validation(
                LinkProviderErrorCode::ValidationFailed.as_str(),
                "Redirect URI must be a valid HTTP/HTTPS URL",
            ));
        }

        // Validate user_id is not nil
        if self.user_id.is_nil() {
            return Err(CommandError::validation(
                LinkProviderErrorCode::ValidationFailed.as_str(),
                "User ID cannot be nil",
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
    async fn handle(
        &self,
        command: LinkProviderCommand,
    ) -> Result<LinkProviderResponse, CommandError> {
        self.link_provider_use_case
            .link_provider(
                command.user_id,
                command.provider,
                command.code,
                command.redirect_uri,
            )
            .await
            .map_err(|e| LinkProviderErrorMapper.map_error(Box::new(e)))
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
impl<L> CommandHandler<GenerateLinkProviderStartUrlCommand>
    for GenerateLinkProviderStartUrlCommandHandler<L>
where
    L: LinkProviderUseCase + Send + Sync + ?Sized,
{
    async fn handle(
        &self,
        command: GenerateLinkProviderStartUrlCommand,
    ) -> Result<String, CommandError> {
        self.link_provider_use_case
            .generate_start_url(command.provider)
            .map_err(|e| LinkProviderErrorMapper.map_error(Box::new(e)))
    }
}

/// Get provider token command
#[derive(Debug, Clone)]
pub struct GetProviderTokenCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// User ID to get the token for
    pub user_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
}

impl GetProviderTokenCommand {
    /// Create a new get provider token command
    pub fn new(user_id: Uuid, provider: Provider) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            user_id,
            provider,
        }
    }
}

#[async_trait]
impl Command for GetProviderTokenCommand {
    type Result = ProviderTokenResponse;

    fn command_type(&self) -> &'static str {
        "get_provider_token"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Validate user_id is not nil
        if self.user_id.is_nil() {
            return Err(CommandError::validation(
                ProviderErrorCode::ValidationFailed.as_str(),
                "User ID cannot be nil",
            ));
        }

        Ok(())
    }
}

/// Get provider token command handler
pub struct GetProviderTokenCommandHandler<P>
where
    P: ProviderUseCase + ?Sized,
{
    provider_use_case: Arc<P>,
}

impl<P> GetProviderTokenCommandHandler<P>
where
    P: ProviderUseCase + ?Sized,
{
    /// Create a new get provider token command handler
    pub fn new(provider_use_case: Arc<P>) -> Self {
        Self { provider_use_case }
    }
}

#[async_trait]
impl<P> CommandHandler<GetProviderTokenCommand> for GetProviderTokenCommandHandler<P>
where
    P: ProviderUseCase + Send + Sync + ?Sized,
{
    async fn handle(
        &self,
        command: GetProviderTokenCommand,
    ) -> Result<ProviderTokenResponse, CommandError> {
        self.provider_use_case
            .get_provider_token(command.user_id, command.provider)
            .await
            .map_err(|e| ProviderErrorMapper.map_error(Box::new(e)))
    }
}
