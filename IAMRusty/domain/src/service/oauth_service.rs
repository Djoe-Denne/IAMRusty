use crate::entity::{
    provider::{Provider, ProviderTokens, ProviderUserProfile},
    user::User,
    user_email::UserEmail,
};
use crate::error::DomainError;
use crate::port::{
    repository::{TokenRepository, UserEmailRepository, UserRepository},
    service::ProviderOAuth2Client,
};
use tracing::{debug, info};
use uuid::Uuid;

use super::TokenService;

use std::collections::HashMap;

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
    provider_clients: HashMap<Provider, Box<dyn ProviderOAuth2Client + Send + Sync>>,
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
    ) -> Self {
        Self {
            user_repository,
            token_repository,
            user_email_repository,
            token_service,
            provider_clients: HashMap::new(),
        }
    }

    /// Register an `OAuth2` provider client
    pub fn register_provider_client(
        &mut self,
        provider: Provider,
        client: Box<dyn ProviderOAuth2Client + Send + Sync>,
    ) {
        self.provider_clients.insert(provider, client);
    }

    /// Get `OAuth2` provider client for the specified provider
    fn get_provider_client(
        &self,
        provider: Provider,
    ) -> Result<&(dyn ProviderOAuth2Client + Send + Sync), DomainError> {
        self.provider_clients
            .get(&provider)
            .map(std::convert::AsRef::as_ref)
            .ok_or_else(|| {
                DomainError::AuthorizationError(format!(
                    "Provider client not configured: {}",
                    provider.as_str()
                ))
            })
    }

    /// Generate an authorization URL for the provider's `OAuth2` flow
    pub fn generate_authorize_url(&self, provider: &str) -> Result<String, DomainError> {
        let provider = Provider::from_str(provider)
            .ok_or_else(|| DomainError::ProviderNotSupported(provider.to_string()))?;

        let client = self.get_provider_client(provider)?;

        Ok(client.generate_authorize_url())
    }

    /// Process `OAuth2` callback and return user and JWT token
    pub async fn process_callback(
        &self,
        provider_name: &str,
        code: &str,
    ) -> Result<(User, String, String), DomainError> {
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

    /// Find a user by their ID
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
        _provider: Provider,
        profile: ProviderUserProfile,
    ) -> Result<User, DomainError> {
        // Email is required for linking
        let email = profile.email.ok_or_else(|| {
            DomainError::UserProfileError("Email is required from OAuth provider".to_string())
        })?;

        // Try to find the user by email (primary linking mechanism)
        if let Some(user) = self
            .user_repository
            .find_by_email(&email)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
        {
            debug!(user_id = %user.id, "Found existing user by email");

            // Update user if needed (e.g., new username, avatar)
            // In a real implementation, we might check if any fields changed

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
        let user_email = UserEmail::new_primary(created_user.id, email.clone(), false); // false = not verified yet

        self.user_email_repository
            .create(user_email)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        debug!(user_id = %created_user.id, email = %email, "Created primary email for OAuth user");

        Ok(created_user)
    }

    /// Get provider tokens for a user
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
            .get_provider_tokens(user_id, provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::NoTokenForProvider)?;

        debug!(user_id = %user_id, provider = %provider.as_str(), "Retrieved provider token");

        Ok(tokens)
    }

    /// Revoke provider tokens for a user
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
            .get_provider_tokens(user_id, provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        if existing_tokens.is_none() {
            return Err(DomainError::NoTokenForProvider);
        }

        // Delete the tokens
        self.token_repository
            .delete_provider_tokens(user_id, provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        debug!(user_id = %user_id, provider = %provider.as_str(), "Revoked provider token");

        Ok(())
    }
}
