use crate::usecase::oauth::{
    AuthUseCase, AuthUseCaseImpl, PasswordService
};
use domain::port::service::AuthTokenService;
use domain::port::repository::{UserRepository, UserEmailRepository, EmailVerificationRepository};
use domain::port::event_publisher::EventPublisher;
use std::sync::Arc;

/// Factory for creating the auth use case with its dependencies
pub struct OAuthFactory;

impl OAuthFactory {
    /// Create an auth use case instance with all its dependencies
    pub fn create_oauth_use_case<UR, UER, EVR, PS, TS, EP>(
        user_repository: Arc<UR>,
        user_email_repository: Arc<UER>,
        email_verification_repository: Arc<EVR>,
        password_service: Arc<PS>,
        token_service: Arc<TS>,
        event_publisher: Arc<EP>,
        jwt_secret: String,
    ) -> Arc<dyn AuthUseCase>
    where
        UR: UserRepository + Send + Sync + 'static,
        UER: UserEmailRepository + Send + Sync + 'static,
        EVR: EmailVerificationRepository + Send + Sync + 'static,
        PS: PasswordService + Send + Sync + 'static,
        TS: AuthTokenService + Send + Sync + 'static,
        EP: EventPublisher + Send + Sync + 'static,
    {
        Arc::new(AuthUseCaseImpl::new(
            user_repository,
            user_email_repository,
            email_verification_repository,
            password_service,
            token_service,
            event_publisher,
            jwt_secret,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::oauth::{AuthError};
    use domain::entity::user::User;
    use domain::port::service::JwtTokenEncoder;
    use domain::entity::user_email::UserEmail;
    use domain::entity::email_verification::EmailVerification;
    use domain::entity::events::DomainEvent;
    use domain::error::DomainError;
    use domain::port::repository::{
        UserReadRepository, UserWriteRepository, 
        UserEmailReadRepository, UserEmailWriteRepository,
        EmailVerificationReadRepository, EmailVerificationWriteRepository
    };
    use domain::entity::token::RefreshToken;
    use chrono::{Utc, Duration};
    use async_trait::async_trait;
    use uuid::Uuid;

    // Mock implementations for testing
    struct MockUserRepository;
    struct MockUserEmailRepository;
    struct MockEmailVerificationRepository;
    struct MockPasswordService;
    struct MockTokenService;
    struct MockEventPublisher;

    #[async_trait]
    impl UserReadRepository for MockUserRepository {
        type Error = DomainError;

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, Self::Error> { Ok(None) }
        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, Self::Error> { Ok(None) }
        async fn find_by_provider_user_id(&self, _provider: domain::entity::provider::Provider, _provider_user_id: &str) -> Result<Option<User>, Self::Error> { Ok(None) }
    }

    #[async_trait]
    impl UserWriteRepository for MockUserRepository {
        type Error = DomainError;

        async fn create(&self, user: User) -> Result<User, Self::Error> { Ok(user) }
        async fn update(&self, user: User) -> Result<User, Self::Error> { Ok(user) }
    }

    #[async_trait]
    impl UserEmailReadRepository for MockUserEmailRepository {
        type Error = DomainError;

        async fn find_by_user_id(&self, _user_id: Uuid) -> Result<Vec<UserEmail>, Self::Error> { Ok(vec![]) }
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<UserEmail>, Self::Error> { Ok(None) }
        async fn find_by_email(&self, _email: &str) -> Result<Option<UserEmail>, Self::Error> { Ok(None) }
        async fn find_primary_by_user_id(&self, _user_id: Uuid) -> Result<Option<UserEmail>, Self::Error> { Ok(None) }
    }

    #[async_trait]
    impl UserEmailWriteRepository for MockUserEmailRepository {
        type Error = DomainError;

        async fn create(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error> { Ok(user_email) }
        async fn update(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error> { Ok(user_email) }
        async fn delete(&self, _id: Uuid) -> Result<(), Self::Error> { Ok(()) }
        async fn set_as_primary(&self, _user_id: Uuid, _email_id: Uuid) -> Result<(), Self::Error> { Ok(()) }
    }

    #[async_trait]
    impl EmailVerificationReadRepository for MockEmailVerificationRepository {
        type Error = DomainError;

        async fn find_by_email_and_token(&self, _email: &str, _token: &str) -> Result<Option<EmailVerification>, Self::Error> { Ok(None) }
        async fn find_by_email(&self, _email: &str) -> Result<Option<EmailVerification>, Self::Error> { Ok(None) }
    }

    #[async_trait]
    impl EmailVerificationWriteRepository for MockEmailVerificationRepository {
        type Error = DomainError;

        async fn create(&self, _verification: &EmailVerification) -> Result<(), Self::Error> { Ok(()) }
        async fn delete_by_email(&self, _email: &str) -> Result<(), Self::Error> { Ok(()) }
        async fn delete_by_id(&self, _id: Uuid) -> Result<(), Self::Error> { Ok(()) }
    }

    #[async_trait]
    impl PasswordService for MockPasswordService {
        async fn hash_password(&self, _password: &str) -> Result<String, AuthError> { Ok("hashed".to_string()) }
        async fn verify_password(&self, _password: &str, _hash: &str) -> Result<bool, AuthError> { Ok(true) }
    }

    #[async_trait]
    impl AuthTokenService for MockTokenService {
        type Error = DomainError;

        async fn generate_access_token(&self, user_id: Uuid) -> Result<domain::entity::token::JwtToken, Self::Error> {
            Ok(domain::entity::token::JwtToken {
                user_id,
                token: "test_token".to_string(),
                expires_at: Utc::now() + Duration::hours(1),
            })
        }
        async fn generate_refresh_token(&self, user_id: Uuid) -> Result<RefreshToken, Self::Error> {
            Ok(RefreshToken {
                id: Uuid::new_v4(),
                user_id,
                token: "test_refresh_token".to_string(),
                is_valid: true,
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::days(30),
            })
        }
        async fn validate_access_token(&self, _token: &str) -> Result<Uuid, Self::Error> { Ok(Uuid::new_v4()) }
        async fn validate_refresh_token(&self, _token: &str) -> Result<RefreshToken, Self::Error> { 
            Ok(RefreshToken {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                token: "test_refresh_token".to_string(),
                is_valid: true,
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::days(30),
            }) 
        }
    }

    impl JwtTokenEncoder for MockTokenService {
        fn encode(&self, _claims: &domain::entity::token::TokenClaims) -> Result<String, DomainError> {
            Ok("encoded_token".to_string())
        }

        fn decode(&self, _token: &str) -> Result<domain::entity::token::TokenClaims, DomainError> {
            Ok(domain::entity::token::TokenClaims {
                sub: Uuid::new_v4().to_string(),
                username: "test_user".to_string(),
                exp: (Utc::now() + Duration::hours(1)).timestamp(),
                iat: Utc::now().timestamp(),
                jti: Uuid::new_v4().to_string(),
            })
        }

        fn jwks(&self) -> domain::entity::token::JwkSet {
            domain::entity::token::JwkSet { keys: vec![] }
        }
    }

    #[async_trait]
    impl EventPublisher for MockEventPublisher {
        async fn publish(&self, _event: DomainEvent) -> Result<(), DomainError> { Ok(()) }
        async fn publish_batch(&self, _events: Vec<DomainEvent>) -> Result<(), DomainError> { Ok(()) }
        async fn health_check(&self) -> Result<(), DomainError> { Ok(()) }
    }

    #[test]
    fn test_create_auth_use_case() {
        let user_repo = Arc::new(MockUserRepository);
        let user_email_repo = Arc::new(MockUserEmailRepository);
        let email_verification_repo = Arc::new(MockEmailVerificationRepository);
        let password_service = Arc::new(MockPasswordService);
        let token_service = Arc::new(MockTokenService);
        let event_publisher = Arc::new(MockEventPublisher);

        let auth_use_case = OAuthFactory::create_oauth_use_case(
            user_repo,
            user_email_repo,
            email_verification_repo,
            password_service,
            token_service,
            event_publisher,
            "test_secret".to_string(),
        );

        // If we get here, the factory successfully created the use case
        assert!(!Arc::ptr_eq(&auth_use_case, &auth_use_case)); // Just check it's a valid Arc
    }
} 