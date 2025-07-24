use crate::usecase::oauth::{OAuthUseCase, OAuthUseCaseImpl};
use iam_domain::port::repository::{TokenRepository, UserEmailRepository, UserRepository};
use iam_domain::port::service::{AuthTokenService, RegistrationTokenService};
use iam_domain::service::oauth_service::OAuthService;
use std::sync::Arc;

/// Factory for creating the OAuth use case with its dependencies
pub struct OAuthFactory;

impl OAuthFactory {
    /// Create an OAuth use case instance with its dependencies
    pub fn create_oauth_use_case<UR, TR, UER, RTS, TS>(
        oauth_service: Arc<OAuthService<UR, TR, UER>>,
        registration_token_service: Arc<RTS>,
        token_service: Arc<TS>,
    ) -> Arc<dyn OAuthUseCase>
    where
        UR: UserRepository + Send + Sync + 'static,
        TR: TokenRepository + Send + Sync + 'static,
        UER: UserEmailRepository + Send + Sync + 'static,
        RTS: RegistrationTokenService + Send + Sync + 'static,
        TS: AuthTokenService + Send + Sync + 'static,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
        <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
        TS::Error: std::error::Error + Send + Sync + 'static,
    {
        Arc::new(OAuthUseCaseImpl::new(
            oauth_service,
            registration_token_service,
            token_service,
        ))
    }
}
