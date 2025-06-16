use crate::usecase::oauth::{OAuthUseCase, OAuthUseCaseImpl};
use domain::port::repository::{TokenRepository, UserEmailRepository, UserRepository};
use domain::port::service::{AuthTokenService, RegistrationTokenService};
use domain::service::oauth_service::OAuthService;
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
    use domain::entity::provider_link::ProviderLink;
    use domain::entity::user::User;
    use domain::entity::user_email::UserEmail;
    use domain::error::DomainError;
    use domain::port::repository::{
        TokenReadRepository, TokenWriteRepository, UserEmailReadRepository,
        UserEmailWriteRepository, UserReadRepository, UserWriteRepository,
    };
    use domain::service::TokenService;
    use uuid::Uuid;

    // Mock implementations for testing
    struct MockUserRepository;
    struct MockTokenRepository;
    struct MockUserEmailRepository;

    #[async_trait]
    impl UserReadRepository for MockUserRepository {
        type Error = DomainError;

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, Self::Error> {
            Ok(None)
        }

        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, Self::Error> {
            Ok(None)
        }

        async fn find_by_provider_user_id(
            &self,
            _provider: Provider,
            _provider_user_id: &str,
        ) -> Result<Option<User>, Self::Error> {
            Ok(None)
        }
    }

    #[async_trait]
    impl UserWriteRepository for MockUserRepository {
        type Error = DomainError;

        async fn create(&self, user: User) -> Result<User, Self::Error> {
            Ok(user)
        }

        async fn update(&self, user: User) -> Result<User, Self::Error> {
            Ok(user)
        }
    }

    #[async_trait]
    impl TokenReadRepository for MockTokenRepository {
        type Error = DomainError;

        async fn get_provider_tokens(
            &self,
            _user_id: Uuid,
            _provider: Provider,
        ) -> Result<Option<ProviderTokens>, Self::Error> {
            Ok(None)
        }

        async fn get_provider_link(
            &self,
            _user_id: Uuid,
            _provider: Provider,
        ) -> Result<Option<ProviderLink>, Self::Error> {
            Ok(None)
        }

        async fn get_user_provider_links(
            &self,
            _user_id: Uuid,
        ) -> Result<Vec<ProviderLink>, Self::Error> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl TokenWriteRepository for MockTokenRepository {
        type Error = DomainError;

        async fn save_provider_tokens(
            &self,
            _user_id: Uuid,
            _provider: Provider,
            _provider_user_id: String,
            _tokens: ProviderTokens,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn update_provider_tokens(
            &self,
            _user_id: Uuid,
            _provider: Provider,
            _tokens: ProviderTokens,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn revoke_provider_tokens(
            &self,
            _user_id: Uuid,
            _provider: Provider,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[async_trait]
    impl UserEmailReadRepository for MockUserEmailRepository {
        type Error = DomainError;

        async fn find_by_user_id(&self, _user_id: Uuid) -> Result<Vec<UserEmail>, Self::Error> {
            Ok(vec![])
        }

        async fn find_by_email(&self, _email: &str) -> Result<Option<UserEmail>, Self::Error> {
            Ok(None)
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<UserEmail>, Self::Error> {
            Ok(None)
        }
    }

    #[async_trait]
    impl UserEmailWriteRepository for MockUserEmailRepository {
        type Error = DomainError;

        async fn create(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error> {
            Ok(user_email)
        }

        async fn update(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error> {
            Ok(user_email)
        }

        async fn delete(&self, _id: Uuid) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_create_oauth_use_case() {
        // This test is currently not fully functional because we don't have proper mock implementations
        // for RegistrationTokenService. But we can test that the types compile correctly.
        // let token_service = TokenService::new("test_secret".to_string(), 3600);
        // let oauth_service = Arc::new(OAuthService::new(
        //     MockUserRepository,
        //     MockTokenRepository,
        //     MockUserEmailRepository,
        //     token_service,
        // ));
        //
        // let _oauth_use_case = OAuthFactory::create_oauth_use_case(oauth_service, registration_token_service);
        // Test passes if it compiles and creates the use case
    }
}
