pub mod auth_service;
pub mod oauth_service;
pub mod provider_link_service;
pub mod refresh_token_service;
pub mod registration_service;
pub mod token_service;
pub mod user_service;

use crate::error::DomainError;
use rustycog_events::event::DomainEvent;

#[async_trait::async_trait]
pub trait IamOutboxUnitOfWork: Send + Sync {
    async fn record_event(&self, event: Box<dyn DomainEvent + 'static>) -> Result<(), DomainError>;
}

pub use auth_service::{AuthError, AuthService, PasswordService};
pub use oauth_service::OAuthService;
pub use provider_link_service::{ProviderLinkResult, ProviderLinkService};
pub use refresh_token_service::{
    RefreshTokenResponse, RefreshTokenService, RefreshTokenServiceImpl,
};
pub use registration_service::{
    RegistrationCompletionResult, RegistrationService, RegistrationServiceImpl,
    UsernameCheckResult, UsernameValidator,
};
pub use token_service::TokenService;
pub use user_service::{UserProfile, UserService, UserServiceImpl};
