use crate::entity::{
    provider::{Provider, ProviderTokens, ProviderUserProfile},
    user::User,
    user_email::UserEmail,
};
use crate::error::DomainError;
use crate::port::{
    repository::{TokenRepository, UserEmailRepository, UserRepository},
    service::FederatedOAuthClient,
};
use tracing::{debug, info};
use uuid::Uuid;

use super::TokenService;

use std::collections::HashMap;
use std::sync::Arc;

/// Authentication service for `OAuth2` providers
pub struct OAuthService<U, T, UE>
where
    U: UserRepository,
    T: TokenRepository,
    UE: UserEmailRepository,
{
    user_repository: U,
    token_repository: T,
    user_email_repository: UE,
    token_service: TokenService,
    provider_clients: Arc<HashMap<Provider, Arc<dyn FederatedOAuthClient>>>,
}

impl<U, T, UE> OAuthService<U, T, UE>
where
    U: UserRepository,
    T: TokenRepository,
    UE: UserEmailRepository,
{
    /// Create a new auth service
    pub fn new(
        user_repository: U,
        token_repository: T,
        user_email_repository: UE,
        token_service: TokenService,
        provider_clients: Arc<HashMap<Provider, Arc<dyn FederatedOAuthClient>>>,
    ) -> Self {
        Self {
            user_repository,
            token_repository,
            user_email_repository,
            token_service,
            provider_clients,
        }
    }

    /// Get federated client for the specified provider
    fn get_provider_client(
        &self,
        provider: &Provider,
    ) -> Result<Arc<dyn FederatedOAuthClient>, DomainError> {
        self.provider_clients
            .get(provider)
            .cloned()
            .ok_or_else(|| DomainError::ConnectorNotConfigured(provider.as_str().to_string()))
    }
}

impl<U, T, UE> OAuthService<U, T, UE>
where
    U: UserRepository + Send + Sync,
    T: TokenRepository + Send + Sync,
    UE: UserEmailRepository + Send + Sync,
{
    /// Generate an authorization URL for the provider's `OAuth2` flow.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the provider is unsupported, its client is not configured,
    /// or the federated authorize call fails.
    pub async fn generate_authorize_url(
        &self,
        provider: &Provider,
        redirect_uri: &str,
        state: &str,
    ) -> Result<String, DomainError> {
        let client = self.get_provider_client(provider)?;
        let response = client
            .authorize(redirect_uri, state)
            .await
            .map_err(|e| DomainError::OAuth2Error(e.to_string()))?;
        Ok(response.authorization_url)
    }

    /// Process `OAuth2` callback and return user and JWT token.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the provider is unsupported, code exchange or profile
    /// lookup fails, the profile has no email, persistence fails, or JWT generation fails.
    pub async fn process_callback(
        &self,
        provider: &Provider,
        code: &str,
        redirect_uri: &str,
    ) -> Result<(User, String, String), DomainError> {
        debug!(
            "Processing OAuth2 callback for provider: {}",
            provider.as_str()
        );

        let client = self.get_provider_client(provider)?;

        let tokens = client
            .exchange_code(code, redirect_uri)
            .await
            .map_err(|e| DomainError::OAuth2Error(e.to_string()))?;

        debug!("Successfully exchanged code for tokens");

        let profile = client
            .user_profile(&tokens.access_token)
            .await
            .map_err(|e| DomainError::UserProfileError(e.to_string()))?;

        debug!("Retrieved user profile: {}", profile.username);

        // Store the provider user ID and email before moving the profile
        let provider_user_id = profile.id.clone();
        let email = profile.email.clone().ok_or_else(|| {
            DomainError::UserProfileError("Email is required from OAuth provider".to_string())
        })?;

        // Find or create the user
        let user = self.find_or_create_user(provider, profile).await?;

        info!(user_id = %user.id, "User authenticated successfully");

        // Save the tokens
        self.token_repository
            .save_provider_tokens(user.id, provider, provider_user_id, tokens)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        // Check if user has a username (complete registration)
        if let Some(username) = user.username.as_ref() {
            // Generate a JWT token for complete users
            let jwt_token = self
                .token_service
                .generate_token(&user.id.to_string(), username)?;
            Ok((user, jwt_token, email))
        } else {
            // Return incomplete user - let the use case handle the registration flow
            Ok((user, String::new(), email)) // Empty JWT token indicates registration needed
        }
    }

    /// Find a user by their ID.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `user_id` is not a valid UUID, the user does not exist,
    /// or the repository lookup fails.
    pub async fn find_user_by_id(&self, user_id: &str) -> Result<User, DomainError> {
        let uuid = uuid::Uuid::parse_str(user_id).map_err(|_| DomainError::UserNotFound)?;

        let user = self
            .user_repository
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
        provider: &Provider,
        profile: ProviderUserProfile,
    ) -> Result<User, DomainError> {
        // Email is required for linking
        let email = profile.email.ok_or_else(|| {
            DomainError::UserProfileError("Email is required from OAuth provider".to_string())
        })?;

        // Returning OAuth user: already linked to this provider identity.
        if let Some(user) = self
            .user_repository
            .find_by_provider_user_id(provider, &profile.id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
        {
            debug!(user_id = %user.id, "Found existing user by provider identity");
            return Ok(user);
        }

        // Merge onto an existing email account only when the IdP marked it verified.
        if let Some(user) = self
            .user_repository
            .find_by_email(&email)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
        {
            if !profile.email_verified {
                return Err(DomainError::BusinessRuleViolation(
                    "OAuth email is not verified; sign in and link the provider".to_string(),
                ));
            }
            debug!(user_id = %user.id, "Found existing user by verified email");
            return Ok(user);
        }

        // Create a new incomplete user (requires registration completion)
        let user = User::new_incomplete(profile.avatar_url);

        let created_user = self
            .user_repository
            .create(user)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        info!(user_id = %created_user.id, "Created new user");

        // Create the user's primary email record
        let user_email =
            UserEmail::new_primary(created_user.id, email.clone(), profile.email_verified);

        self.user_email_repository
            .create(user_email)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        debug!(user_id = %created_user.id, email = %email, "Created primary email for OAuth user");

        Ok(created_user)
    }

    /// Get provider tokens for a user.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the user does not exist, no tokens are stored for the
    /// provider, or the repository lookup fails.
    pub async fn get_provider_token(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<ProviderTokens, DomainError> {
        // First verify that the user exists (security: don't reveal if user has tokens or not)
        let _user = self
            .user_repository
            .find_by_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::UserNotFound)?;

        let tokens = self
            .token_repository
            .get_provider_tokens(user_id, &provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::NoTokenForProvider)?;

        debug!(user_id = %user_id, provider = %provider.as_str(), "Retrieved provider token");

        Ok(tokens)
    }

    /// Revoke provider tokens for a user.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the user does not exist, no tokens are stored for the
    /// provider, or the repository operation fails.
    pub async fn revoke_provider_token(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<(), DomainError> {
        // First verify that the user exists
        let _user = self
            .user_repository
            .find_by_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::UserNotFound)?;

        // Check if tokens exist for this user and provider
        let existing_tokens = self
            .token_repository
            .get_provider_tokens(user_id, &provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        if existing_tokens.is_none() {
            return Err(DomainError::NoTokenForProvider);
        }

        // Delete the tokens
        self.token_repository
            .delete_provider_tokens(user_id, &provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        debug!(user_id = %user_id, provider = %provider.as_str(), "Revoked provider token");

        Ok(())
    }
}

#[cfg(test)]
mod missing_client_tests {
    use super::*;
    use crate::entity::provider_link::ProviderLink;
    use crate::entity::token::JwkSet;
    use crate::entity::user_email::UserEmail;
    use crate::port::repository::{
        TokenReadRepository, TokenWriteRepository, UserEmailReadRepository,
        UserEmailWriteRepository, UserReadRepository, UserWriteRepository,
    };
    use crate::port::service::JwtTokenEncoder;
    use chrono::Duration;

    #[derive(Debug)]
    struct StubError;

    impl std::fmt::Display for StubError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "stub")
        }
    }

    impl std::error::Error for StubError {}

    struct StubUser;
    struct StubTokens;
    struct StubEmails;
    struct StubEncoder;

    #[async_trait::async_trait]
    impl UserReadRepository for StubUser {
        type Error = StubError;
        async fn find_by_id(&self, _: Uuid) -> Result<Option<User>, Self::Error> {
            Err(StubError)
        }
        async fn find_by_email(&self, _: &str) -> Result<Option<User>, Self::Error> {
            Err(StubError)
        }
        async fn find_by_username(&self, _: &str) -> Result<Option<User>, Self::Error> {
            Err(StubError)
        }
        async fn find_by_provider_user_id(
            &self,
            _: &Provider,
            _: &str,
        ) -> Result<Option<User>, Self::Error> {
            Err(StubError)
        }
    }

    #[async_trait::async_trait]
    impl UserWriteRepository for StubUser {
        type Error = StubError;
        async fn create(&self, _: User) -> Result<User, Self::Error> {
            Err(StubError)
        }
        async fn update(&self, _: User) -> Result<User, Self::Error> {
            Err(StubError)
        }
    }

    #[async_trait::async_trait]
    impl TokenReadRepository for StubTokens {
        type Error = StubError;
        async fn get_provider_tokens(
            &self,
            _: Uuid,
            _: &Provider,
        ) -> Result<Option<ProviderTokens>, Self::Error> {
            Err(StubError)
        }
        async fn get_provider_link(
            &self,
            _: Uuid,
            _: &Provider,
        ) -> Result<Option<ProviderLink>, Self::Error> {
            Err(StubError)
        }
        async fn get_user_provider_links(&self, _: Uuid) -> Result<Vec<ProviderLink>, Self::Error> {
            Err(StubError)
        }
    }

    #[async_trait::async_trait]
    impl TokenWriteRepository for StubTokens {
        type Error = StubError;
        async fn save_provider_tokens(
            &self,
            _: Uuid,
            _: &Provider,
            _: String,
            _: ProviderTokens,
        ) -> Result<(), Self::Error> {
            Err(StubError)
        }
        async fn delete_provider_tokens(&self, _: Uuid, _: &Provider) -> Result<(), Self::Error> {
            Err(StubError)
        }
    }

    #[async_trait::async_trait]
    impl UserEmailReadRepository for StubEmails {
        type Error = StubError;
        async fn find_by_user_id(&self, _: Uuid) -> Result<Vec<UserEmail>, Self::Error> {
            Err(StubError)
        }
        async fn find_by_id(&self, _: Uuid) -> Result<Option<UserEmail>, Self::Error> {
            Err(StubError)
        }
        async fn find_by_email(&self, _: &str) -> Result<Option<UserEmail>, Self::Error> {
            Err(StubError)
        }
        async fn find_primary_by_user_id(&self, _: Uuid) -> Result<Option<UserEmail>, Self::Error> {
            Err(StubError)
        }
    }

    #[async_trait::async_trait]
    impl UserEmailWriteRepository for StubEmails {
        type Error = StubError;
        async fn create(&self, _: UserEmail) -> Result<UserEmail, Self::Error> {
            Err(StubError)
        }
        async fn update(&self, _: UserEmail) -> Result<UserEmail, Self::Error> {
            Err(StubError)
        }
        async fn delete(&self, _: Uuid) -> Result<(), Self::Error> {
            Err(StubError)
        }
        async fn set_as_primary(&self, _: Uuid, _: Uuid) -> Result<(), Self::Error> {
            Err(StubError)
        }
    }

    impl JwtTokenEncoder for StubEncoder {
        fn encode(&self, _: &crate::entity::token::TokenClaims) -> Result<String, DomainError> {
            Err(DomainError::InvalidToken)
        }
        fn decode(&self, _: &str) -> Result<crate::entity::token::TokenClaims, DomainError> {
            Err(DomainError::InvalidToken)
        }
        fn jwks(&self) -> JwkSet {
            JwkSet { keys: vec![] }
        }
    }

    #[tokio::test]
    async fn empty_client_map_returns_connector_not_configured() {
        let service = OAuthService::new(
            StubUser,
            StubTokens,
            StubEmails,
            TokenService::new(Arc::new(StubEncoder), Duration::hours(1)),
            Arc::new(HashMap::new()),
        );
        let provider = Provider::parse_slug("github").expect("github slug");
        let err = service
            .generate_authorize_url(&provider, "http://localhost/cb", "state")
            .await
            .expect_err("empty catalogue");
        assert!(
            matches!(err, DomainError::ConnectorNotConfigured(ref slug) if slug == "github"),
            "unexpected error: {err:?}"
        );
    }
}
