use crate::usecase::oauth::{OAuthError, OAuthResponse, OAuthUseCase};
use async_trait::async_trait;
use iam_domain::entity::provider::Provider;
use rustycog::command::{Command, CommandError, CommandErrorMapper, CommandHandler};
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
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
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
        error.downcast_ref::<OAuthError>().map_or_else(
            || {
                let error_msg = error.to_string();
                if Self::is_authentication_related_error(&error_msg) {
                    CommandError::business(
                        OAuthLoginErrorCode::AuthenticationFailed.as_str(),
                        format!("OAuth authentication failed: {error_msg}"),
                    )
                } else {
                    CommandError::infrastructure(
                        OAuthLoginErrorCode::ProviderError.as_str(),
                        error.to_string(),
                    )
                }
            },
            |oauth_error| match oauth_error {
                OAuthError::DomainError(domain_error) => {
                    use iam_domain::error::DomainError;
                    match domain_error {
                        DomainError::UserNotFound => CommandError::business(
                            OAuthLoginErrorCode::AuthenticationFailed.as_str(),
                            "User not found",
                        ),
                        DomainError::ProviderNotSupported(_) => CommandError::business(
                            OAuthLoginErrorCode::ProviderError.as_str(),
                            "Provider not supported",
                        ),
                        DomainError::ConnectorNotConfigured(provider) => CommandError::validation(
                            "connector_not_configured",
                            format!(
                                "IdP connector is not configured for this provider: {provider}"
                            ),
                        ),
                        DomainError::AuthorizationError(_) => CommandError::business(
                            OAuthLoginErrorCode::AuthenticationFailed.as_str(),
                            "Authorization failed",
                        ),
                        DomainError::RepositoryError(_) => CommandError::infrastructure(
                            OAuthLoginErrorCode::DatabaseError.as_str(),
                            "Database error during OAuth flow",
                        ),
                        _ => CommandError::infrastructure(
                            OAuthLoginErrorCode::ProviderError.as_str(),
                            "OAuth flow error",
                        ),
                    }
                }
            },
        )
    }
}

impl OAuthLoginErrorMapper {
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

/// OAuth login command
#[derive(Debug, Clone)]
pub struct OAuthLoginCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// OAuth provider
    pub provider: Provider,
    /// Authorization code from OAuth callback
    pub code: String,
    /// Redirect URI used at authorize time
    pub redirect_uri: String,
}

impl OAuthLoginCommand {
    /// Create a new OAuth login command
    #[must_use]
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
    type Result = OAuthResponse;

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
                "Authorization code cannot be empty",
            ));
        }

        if self.redirect_uri.trim().is_empty() {
            return Err(CommandError::validation(
                OAuthLoginErrorCode::ValidationFailed.as_str(),
                "Redirect URI cannot be empty",
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
    pub const fn new(oauth_use_case: Arc<O>) -> Self {
        Self { oauth_use_case }
    }
}

#[async_trait]
impl<O> CommandHandler<OAuthLoginCommand> for OAuthLoginCommandHandler<O>
where
    O: OAuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: OAuthLoginCommand) -> Result<OAuthResponse, CommandError> {
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
    /// Redirect URI for this start
    pub redirect_uri: String,
    /// Encoded IAM OAuth state
    pub state: String,
}

impl GenerateOAuthStartUrlCommand {
    /// Create a new generate OAuth start URL command
    #[must_use]
    pub fn new(provider: Provider, redirect_uri: String, state: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            provider,
            redirect_uri,
            state,
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
        if self.redirect_uri.trim().is_empty() {
            return Err(CommandError::validation(
                OAuthLoginErrorCode::ValidationFailed.as_str(),
                "Redirect URI cannot be empty",
            ));
        }
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
    pub const fn new(oauth_use_case: Arc<O>) -> Self {
        Self { oauth_use_case }
    }
}

#[async_trait]
impl<O> CommandHandler<GenerateOAuthStartUrlCommand> for GenerateOAuthStartUrlCommandHandler<O>
where
    O: OAuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: GenerateOAuthStartUrlCommand) -> Result<String, CommandError> {
        self.oauth_use_case
            .generate_start_url(command.provider, command.redirect_uri, command.state)
            .await
            .map_err(|e| OAuthLoginErrorMapper.map_error(Box::new(e)))
    }
}
