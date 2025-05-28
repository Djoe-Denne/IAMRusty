use crate::entity::{
    provider::{Provider, ProviderTokens, ProviderUserProfile},
    user::User,
};
use crate::error::DomainError;
use crate::port::{
    repository::{TokenRepository, UserRepository},
    service::ProviderOAuth2Client,
};
use tracing::{debug, info};

use super::TokenService;

use std::collections::HashMap;

/// Authentication service for OAuth2 providers
pub struct AuthService<U, T> 
where
    U: UserRepository,
    T: TokenRepository,
{
    user_repository: U,
    token_repository: T,
    token_service: TokenService,
    provider_clients: HashMap<Provider, Box<dyn ProviderOAuth2Client + Send + Sync>>,
}

impl<U, T> AuthService<U, T>
where
    U: UserRepository,
    T: TokenRepository,
{
    /// Create a new auth service
    pub fn new(user_repository: U, token_repository: T, token_service: TokenService) -> Self {
        Self {
            user_repository,
            token_repository,
            token_service,
            provider_clients: HashMap::new(),
        }
    }

    /// Register an OAuth2 provider client
    pub fn register_provider_client(
        &mut self,
        provider: Provider,
        client: Box<dyn ProviderOAuth2Client + Send + Sync>,
    ) {
        self.provider_clients.insert(provider, client);
    }

    /// Get OAuth2 provider client for the specified provider
    fn get_provider_client(&self, provider: Provider) -> Result<&(dyn ProviderOAuth2Client + Send + Sync), DomainError> {
        self.provider_clients
            .get(&provider)
            .map(|client| client.as_ref())
            .ok_or_else(|| DomainError::AuthorizationError(format!("Provider client not configured: {}", provider.as_str())))
    }

    /// Generate an authorization URL for the provider's OAuth2 flow
    pub fn generate_authorize_url(&self, provider: &str) -> Result<String, DomainError> {
        let provider = Provider::from_str(provider)
            .ok_or_else(|| DomainError::ProviderNotSupported(provider.to_string()))?;
        
        let client = self.get_provider_client(provider)?;
        
        Ok(client.generate_authorize_url())
    }

    /// Process OAuth2 callback and return user and JWT token
    pub async fn process_callback(
        &self,
        provider_name: &str,
        code: &str,
    ) -> Result<(User, String), DomainError> {
        let provider = Provider::from_str(provider_name)
            .ok_or_else(|| DomainError::ProviderNotSupported(provider_name.to_string()))?;
        
        debug!("Processing OAuth2 callback for provider: {}", provider_name);
        
        // Get the provider client
        let client = self.get_provider_client(provider)?;
        
        // Exchange the authorization code for tokens
        let tokens = client.exchange_code(code).await?;
        
        debug!("Successfully exchanged code for tokens");
        
        // Get the user profile
        let profile = client.get_user_profile(&tokens).await?;
        
        debug!("Retrieved user profile: {}", profile.username);
        
        // Store the provider user ID before moving the profile
        let provider_user_id = profile.id.clone();
        
        // Find or create the user
        let user = self.find_or_create_user(provider, profile).await?;
        
        info!(user_id = %user.id, "User authenticated successfully");
        
                // Save the tokens
        self.token_repository
            .save_provider_tokens(user.id, provider, provider_user_id, tokens)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        // Generate a JWT token
        let jwt_token = self.token_service
            .generate_token(&user.id.to_string(), &user.username)?;

        Ok((user, jwt_token))
    }

    /// Find a user by their ID
    pub async fn find_user_by_id(&self, user_id: &str) -> Result<User, DomainError> {
        let uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|_| DomainError::UserNotFound)?;
        
        let user = self.user_repository
            .find_by_id(uuid)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::UserNotFound)?;
        
        debug!(user_id = %user.id, "Found user by ID");
        
        Ok(user)
    }

    /// Find or create a user based on their provider profile
    async fn find_or_create_user(
        &self,
        _provider: Provider,
        profile: ProviderUserProfile,
    ) -> Result<User, DomainError> {
        // Email is required for linking
        let email = profile.email.ok_or_else(|| {
            DomainError::UserProfileError("Email is required from OAuth provider".to_string())
        })?;
        
        // Try to find the user by email (primary linking mechanism)
        if let Some(user) = self.user_repository
            .find_by_email(&email)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
        {
            debug!(user_id = %user.id, "Found existing user by email");
            
            // Update user if needed (e.g., new username, avatar)
            // In a real implementation, we might check if any fields changed
            
            return Ok(user);
        }
        
        // Create a new user
        let user = User::new(
            profile.username,
            profile.avatar_url,
        );
        
        let created_user = self.user_repository
            .create(user)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        
        info!(user_id = %created_user.id, "Created new user");
        
        Ok(created_user)
    }

    /// Get a provider token for a user
    pub async fn get_provider_token(
        &self,
        user_id: &str,
        provider_name: &str,
    ) -> Result<ProviderTokens, DomainError> {
        let uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|_| DomainError::UserNotFound)?;
        
        let provider = Provider::from_str(provider_name)
            .ok_or_else(|| DomainError::ProviderNotSupported(provider_name.to_string()))?;
        
        let tokens = self.token_repository
            .get_provider_tokens(uuid, provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or_else(|| 
                DomainError::NoTokenForProvider(
                    provider_name.to_string(),
                    user_id.to_string()
                )
            )?;
        
        debug!(user_id = %uuid, provider = %provider_name, "Retrieved provider token");
        
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{
        provider::{Provider, ProviderTokens, ProviderUserProfile},
        user::User,
    };
    use crate::error::DomainError;
    use crate::port::{
        repository::{TokenReadRepository, TokenWriteRepository, UserReadRepository, UserWriteRepository},
        service::ProviderOAuth2Client,
    };
    use mockall::{mock, predicate::*};
    use rstest::*;
    use uuid::Uuid;
    use chrono::{Duration as ChronoDuration, Utc};
    use claims::*;
    use std::collections::HashMap;

    // Define a test error type that implements std::error::Error
    #[derive(Debug, Clone)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    // Manual mock implementations to avoid mockall macro issues
    #[derive(Default)]
    struct MockUserRepo {
        find_by_id_responses: HashMap<Uuid, Result<Option<User>, TestError>>,
        find_by_email_responses: HashMap<String, Result<Option<User>, TestError>>,
        create_responses: Vec<Result<User, TestError>>,
    }

    impl MockUserRepo {
        fn new() -> Self {
            Self::default()
        }

        fn expect_find_by_id(&mut self, id: Uuid, response: Result<Option<User>, TestError>) {
            self.find_by_id_responses.insert(id, response);
        }

        fn expect_find_by_email(&mut self, email: String, response: Result<Option<User>, TestError>) {
            self.find_by_email_responses.insert(email, response);
        }

        fn expect_create(&mut self, response: Result<User, TestError>) {
            self.create_responses.push(response);
        }
    }

    #[async_trait::async_trait]
    impl UserReadRepository for MockUserRepo {
        type Error = TestError;

        async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error> {
            self.find_by_id_responses.get(&id)
                .cloned()
                .unwrap_or(Ok(None))
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>, Self::Error> {
            self.find_by_email_responses.get(email)
                .cloned()
                .unwrap_or(Ok(None))
        }

        async fn find_by_provider_user_id(
            &self,
            _provider: Provider,
            _provider_user_id: &str,
        ) -> Result<Option<User>, Self::Error> {
            Ok(None)
        }
    }

    #[async_trait::async_trait]
    impl UserWriteRepository for MockUserRepo {
        type Error = TestError;

        async fn create(&self, user: User) -> Result<User, Self::Error> {
            if let Some(response) = self.create_responses.first() {
                response.clone()
            } else {
                Ok(user)
            }
        }

        async fn update(&self, user: User) -> Result<User, Self::Error> {
            Ok(user)
        }
    }

    #[derive(Default)]
    struct MockTokenRepo {
        get_provider_tokens_responses: HashMap<(Uuid, Provider), Result<Option<ProviderTokens>, TestError>>,
        save_calls: Vec<(Uuid, Provider, String, ProviderTokens)>,
    }

    impl MockTokenRepo {
        fn new() -> Self {
            Self::default()
        }

        fn expect_get_provider_tokens(&mut self, user_id: Uuid, provider: Provider, response: Result<Option<ProviderTokens>, TestError>) {
            self.get_provider_tokens_responses.insert((user_id, provider), response);
        }

        fn expect_save_provider_tokens(&mut self) {
            // For now, just track that save was called
        }
    }

    #[async_trait::async_trait]
    impl TokenReadRepository for MockTokenRepo {
        type Error = TestError;

        async fn get_provider_tokens(
            &self,
            user_id: Uuid,
            provider: Provider,
        ) -> Result<Option<ProviderTokens>, Self::Error> {
            self.get_provider_tokens_responses.get(&(user_id, provider))
                .cloned()
                .unwrap_or(Ok(None))
        }

        async fn get_provider_link(
            &self,
            _user_id: Uuid,
            _provider: Provider,
        ) -> Result<Option<crate::entity::provider_link::ProviderLink>, Self::Error> {
            Ok(None)
        }

        async fn get_user_provider_links(
            &self,
            _user_id: Uuid,
        ) -> Result<Vec<crate::entity::provider_link::ProviderLink>, Self::Error> {
            Ok(vec![])
        }
    }

    #[async_trait::async_trait]
    impl TokenWriteRepository for MockTokenRepo {
        type Error = TestError;

        async fn save_provider_tokens(
            &self,
            _user_id: Uuid,
            _provider: Provider,
            _provider_user_id: String,
            _tokens: ProviderTokens,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    mock! {
        OAuth2Client {}

        #[async_trait::async_trait]
        impl ProviderOAuth2Client for OAuth2Client {
            fn generate_authorize_url(&self) -> String;
            async fn exchange_code(&self, code: &str) -> Result<ProviderTokens, DomainError>;
            async fn get_user_profile(&self, tokens: &ProviderTokens) -> Result<ProviderUserProfile, DomainError>;
        }
    }

    // Mock token encoder for testing
    mock! {
        TokenEnc {}
        
        impl crate::port::service::TokenEncoder for TokenEnc {
            fn encode(&self, claims: &crate::entity::token::TokenClaims) -> Result<String, DomainError>;
            fn decode(&self, token: &str) -> Result<crate::entity::token::TokenClaims, DomainError>;
            fn jwks(&self) -> crate::entity::token::JwkSet;
        }
    }

    // Test fixtures
    #[fixture]
    fn sample_user() -> User {
        User {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            username: "testuser".to_string(),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[fixture]
    fn sample_provider_tokens() -> ProviderTokens {
        ProviderTokens {
            access_token: "github_access_token".to_string(),
            refresh_token: Some("github_refresh_token".to_string()),
            expires_in: Some(3600),
        }
    }

    #[fixture]
    fn sample_provider_profile() -> ProviderUserProfile {
        ProviderUserProfile {
            id: "github123".to_string(),
            username: "githubuser".to_string(),
            email: Some("user@example.com".to_string()),
            avatar_url: Some("https://github.com/avatar.jpg".to_string()),
        }
    }

    #[fixture]
    fn auth_service() -> AuthService<MockUserRepo, MockTokenRepo> {
        let user_repo = MockUserRepo::new();
        let token_repo = MockTokenRepo::new();
        // Create a minimal TokenService for testing - we'll test token generation separately
        let mock_encoder = Box::new(MockTokenEnc::new());
        let token_service = TokenService::new(mock_encoder, ChronoDuration::hours(1));
        
        AuthService::new(user_repo, token_repo, token_service)
    }

    mod auth_service_creation {
        use super::*;

        #[test]
        fn new_creates_auth_service_with_empty_provider_clients() {
            let user_repo = MockUserRepo::new();
            let token_repo = MockTokenRepo::new();
            let token_service = TokenService::new(
                Box::new(MockTokenEnc::new()), 
                ChronoDuration::hours(1)
            );
            
            let auth_service = AuthService::new(user_repo, token_repo, token_service);
            
            assert!(auth_service.provider_clients.is_empty());
        }

        #[test]
        fn register_provider_client_adds_client_to_map() {
            let mut auth_service = auth_service();
            let provider = Provider::GitHub;
            let client = Box::new(MockOAuth2Client::new());

            auth_service.register_provider_client(provider, client);

            assert!(auth_service.provider_clients.len() == 1);
            assert!(auth_service.provider_clients.contains_key(&provider));
        }
    }

    mod generate_authorize_url {
        use super::*;

        #[rstest]
        #[case("github", Provider::GitHub)]
        #[case("gitlab", Provider::GitLab)]
        #[test]
        fn success_with_valid_provider(#[case] provider_str: &str, #[case] provider: Provider) {
            let mut auth_service = auth_service();
            let mut mock_client = MockOAuth2Client::new();
            
            mock_client
                .expect_generate_authorize_url()
                .times(1)
                .returning(|| "https://github.com/login/oauth/authorize?client_id=test".to_string());

            auth_service.register_provider_client(provider, Box::new(mock_client));

            let result = auth_service.generate_authorize_url(provider_str);

            assert_ok!(&result);
            assert_eq!(result.unwrap(), "https://github.com/login/oauth/authorize?client_id=test");
        }

        #[test]
        fn error_with_unsupported_provider() {
            let auth_service = auth_service();

            let result = auth_service.generate_authorize_url("unsupported");

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::ProviderNotSupported(provider) => {
                    assert_eq!(provider, "unsupported");
                }
                _ => panic!("Expected ProviderNotSupported error"),
            }
        }

        #[test]
        fn error_when_provider_client_not_configured() {
            let auth_service = auth_service();

            let result = auth_service.generate_authorize_url("github");

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::AuthorizationError(msg) => {
                    assert!(msg.contains("Provider client not configured"));
                }
                _ => panic!("Expected AuthorizationError"),
            }
        }
    }

    mod process_callback {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn success_with_existing_user(
            sample_user: User,
            sample_provider_tokens: ProviderTokens,
            sample_provider_profile: ProviderUserProfile,
        ) {
            let mut user_repo = MockUserRepo::new();
            let mut token_repo = MockTokenRepo::new();
            let provider = Provider::GitHub;
            
            // Setup user repository mock - user exists by email
            user_repo.expect_find_by_email(
                "user@example.com".to_string(),
                Ok(Some(sample_user.clone()))
            );

            // Setup token repository mock
            token_repo.expect_save_provider_tokens();

            // Create a mock token encoder that returns a test token
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_encode()
                .times(1)
                .returning(|_| Ok("jwt_token".to_string()));

            let token_service = TokenService::new(Box::new(mock_encoder), ChronoDuration::hours(1));
            let mut auth_service = AuthService::new(user_repo, token_repo, token_service);
            
            // Setup mock OAuth2 client
            let mut mock_client = MockOAuth2Client::new();
            mock_client
                .expect_exchange_code()
                .with(eq("auth_code"))
                .times(1)
                .returning(move |_| Ok(sample_provider_tokens.clone()));
            
            mock_client
                .expect_get_user_profile()
                .times(1)
                .returning(move |_| Ok(sample_provider_profile.clone()));

            auth_service.register_provider_client(provider, Box::new(mock_client));

            let result = auth_service.process_callback("github", "auth_code").await;

            assert_ok!(&result);
            let (user, jwt_token) = result.unwrap();
            assert_eq!(user.id, sample_user.id);
            assert_eq!(jwt_token, "jwt_token");
        }

        #[tokio::test]
        async fn error_with_unsupported_provider() {
            let auth_service = auth_service();

            let result = auth_service.process_callback("unsupported", "auth_code").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::ProviderNotSupported(provider) => {
                    assert_eq!(provider, "unsupported");
                }
                _ => panic!("Expected ProviderNotSupported error"),
            }
        }

        #[tokio::test]
        async fn error_when_provider_client_not_configured() {
            let auth_service = auth_service();

            let result = auth_service.process_callback("github", "auth_code").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::AuthorizationError(msg) => {
                    assert!(msg.contains("Provider client not configured"));
                }
                _ => panic!("Expected AuthorizationError"),
            }
        }

        #[rstest]
        #[tokio::test]
        async fn error_when_profile_missing_email(sample_provider_tokens: ProviderTokens) {
            let mut auth_service = auth_service();
            let provider = Provider::GitHub;
            
            let mut profile_without_email = sample_provider_profile();
            profile_without_email.email = None;

            let mut mock_client = MockOAuth2Client::new();
            mock_client
                .expect_exchange_code()
                .times(1)
                .returning(move |_| Ok(sample_provider_tokens.clone()));
            
            mock_client
                .expect_get_user_profile()
                .times(1)
                .returning(move |_| Ok(profile_without_email.clone()));

            auth_service.register_provider_client(provider, Box::new(mock_client));

            let result = auth_service.process_callback("github", "auth_code").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::UserProfileError(msg) => {
                    assert!(msg.contains("Email is required"));
                }
                _ => panic!("Expected UserProfileError"),
            }
        }

        #[rstest]
        #[tokio::test]
        async fn error_when_code_exchange_fails(_sample_provider_tokens: ProviderTokens) {
            let mut auth_service = auth_service();
            let provider = Provider::GitHub;
            
            let mut mock_client = MockOAuth2Client::new();
            mock_client
                .expect_exchange_code()
                .times(1)
                .returning(|_| Err(DomainError::OAuth2Error("Invalid code".to_string())));

            auth_service.register_provider_client(provider, Box::new(mock_client));

            let result = auth_service.process_callback("github", "invalid_code").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::OAuth2Error(msg) => {
                    assert_eq!(msg, "Invalid code");
                }
                _ => panic!("Expected OAuth2Error"),
            }
        }
    }

    mod find_user_by_id {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn success_with_valid_uuid(sample_user: User) {
            let mut user_repo = MockUserRepo::new();
            let token_repo = MockTokenRepo::new();
            let user_id = sample_user.id;

            user_repo.expect_find_by_id(user_id, Ok(Some(sample_user.clone())));

            let token_service = TokenService::new(Box::new(MockTokenEnc::new()), ChronoDuration::hours(1));
            let auth_service = AuthService::new(user_repo, token_repo, token_service);

            let result = auth_service.find_user_by_id(&user_id.to_string()).await;

            assert_ok!(&result);
            let user = result.unwrap();
            assert_eq!(user.id, user_id);
        }

        #[tokio::test]
        async fn error_with_invalid_uuid() {
            let auth_service = auth_service();

            let result = auth_service.find_user_by_id("invalid-uuid").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::UserNotFound => {}
                _ => panic!("Expected UserNotFound error"),
            }
        }

        #[tokio::test]
        async fn error_when_user_not_found() {
            let mut user_repo = MockUserRepo::new();
            let token_repo = MockTokenRepo::new();
            let user_id = Uuid::new_v4();

            user_repo.expect_find_by_id(user_id, Ok(None));

            let token_service = TokenService::new(Box::new(MockTokenEnc::new()), ChronoDuration::hours(1));
            let auth_service = AuthService::new(user_repo, token_repo, token_service);

            let result = auth_service.find_user_by_id(&user_id.to_string()).await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::UserNotFound => {}
                _ => panic!("Expected UserNotFound error"),
            }
        }

        #[tokio::test]
        async fn error_when_repository_fails() {
            let mut user_repo = MockUserRepo::new();
            let token_repo = MockTokenRepo::new();
            let user_id = Uuid::new_v4();

            user_repo.expect_find_by_id(user_id, Err(TestError("Database error".to_string())));

            let token_service = TokenService::new(Box::new(MockTokenEnc::new()), ChronoDuration::hours(1));
            let auth_service = AuthService::new(user_repo, token_repo, token_service);

            let result = auth_service.find_user_by_id(&user_id.to_string()).await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::RepositoryError(msg) => {
                    assert!(msg.contains("Database error"));
                }
                _ => panic!("Expected RepositoryError"),
            }
        }
    }

    mod get_provider_token {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn success_with_existing_tokens(sample_provider_tokens: ProviderTokens) {
            let user_repo = MockUserRepo::new();
            let mut token_repo = MockTokenRepo::new();
            let user_id = Uuid::new_v4();
            let provider = Provider::GitHub;

            token_repo.expect_get_provider_tokens(user_id, provider, Ok(Some(sample_provider_tokens.clone())));

            let token_service = TokenService::new(Box::new(MockTokenEnc::new()), ChronoDuration::hours(1));
            let auth_service = AuthService::new(user_repo, token_repo, token_service);

            let result = auth_service.get_provider_token(&user_id.to_string(), "github").await;

            assert_ok!(&result);
            let tokens = result.unwrap();
            assert_eq!(tokens.access_token, "github_access_token");
        }

        #[tokio::test]
        async fn error_with_invalid_user_id() {
            let auth_service = auth_service();

            let result = auth_service.get_provider_token("invalid-uuid", "github").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::UserNotFound => {}
                _ => panic!("Expected UserNotFound error"),
            }
        }

        #[tokio::test]
        async fn error_with_unsupported_provider() {
            let auth_service = auth_service();
            let user_id = Uuid::new_v4();

            let result = auth_service.get_provider_token(&user_id.to_string(), "unsupported").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::ProviderNotSupported(provider) => {
                    assert_eq!(provider, "unsupported");
                }
                _ => panic!("Expected ProviderNotSupported error"),
            }
        }

        #[tokio::test]
        async fn error_when_no_tokens_found() {
            let user_repo = MockUserRepo::new();
            let mut token_repo = MockTokenRepo::new();
            let user_id = Uuid::new_v4();
            let provider = Provider::GitHub;

            token_repo.expect_get_provider_tokens(user_id, provider, Ok(None));

            let token_service = TokenService::new(Box::new(MockTokenEnc::new()), ChronoDuration::hours(1));
            let auth_service = AuthService::new(user_repo, token_repo, token_service);

            let result = auth_service.get_provider_token(&user_id.to_string(), "github").await;

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::NoTokenForProvider(provider_name, user_id_str) => {
                    assert_eq!(provider_name, "github");
                    assert_eq!(user_id_str, user_id.to_string());
                }
                _ => panic!("Expected NoTokenForProvider error"),
            }
        }
    }

    #[rstest]
    #[case(Provider::GitHub, "github")]
    #[case(Provider::GitLab, "gitlab")]
    #[test]
    fn get_provider_client_success(#[case] provider: Provider, #[case] _provider_str: &str) {
        let mut auth_service = auth_service();
        let mock_client = MockOAuth2Client::new();
        
        auth_service.register_provider_client(provider, Box::new(mock_client));
        
        let result = auth_service.get_provider_client(provider);
        
        assert_ok!(&result);
    }

    #[test]
    fn get_provider_client_error_when_not_configured() {
        let auth_service = auth_service();
        
        let result = auth_service.get_provider_client(Provider::GitHub);
        
        assert!(result.is_err());
        if let Err(DomainError::AuthorizationError(msg)) = result {
            assert!(msg.contains("Provider client not configured"));
        } else {
            panic!("Expected AuthorizationError");
        }
    }
} 