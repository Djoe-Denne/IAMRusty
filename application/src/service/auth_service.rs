use domain::entity::{
    provider::{Provider, ProviderTokens, ProviderUserProfile},
    user::User,
};
use domain::error::DomainError;
use domain::port::{
    repository::{TokenRepository, UserRepository},
    service::ProviderOAuth2Client,
};
use tracing::{debug, info};

use crate::{dto::{AuthResponseDto, UserProfileDto}, error::{ApplicationError, map_repo_err}};

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
    fn get_provider_client(&self, provider: Provider) -> Result<&(dyn ProviderOAuth2Client + Send + Sync), ApplicationError> {
        self.provider_clients
            .get(&provider)
            .map(|client| client.as_ref())
            .ok_or_else(|| ApplicationError::Service(format!("Provider client not configured: {}", provider.as_str())))
    }

    /// Generate an authorization URL for the provider's OAuth2 flow
    pub fn generate_authorize_url(&self, provider: &str) -> Result<String, ApplicationError> {
        let provider = Provider::from_str(provider)
            .ok_or_else(|| DomainError::ProviderNotSupported(provider.to_string()))?;
        
        let client = self.get_provider_client(provider)?;
        
        Ok(client.generate_authorize_url())
    }

    /// Process OAuth2 callback and return a JWT token
    pub async fn process_callback(
        &self,
        provider_name: &str,
        code: &str,
    ) -> Result<AuthResponseDto, ApplicationError> {
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
            .map_err(map_repo_err)?;
        
        // Generate a JWT token
        let jwt_token = self.token_service
            .generate_token(&user.id.to_string(), &user.username)?;
        
        // Create the response DTO
        let response = AuthResponseDto {
            token: jwt_token,
            user: UserProfileDto::from(user),
        };
        
        Ok(response)
    }

    /// Find a user by their ID
    pub async fn find_user_by_id(&self, user_id: &str) -> Result<UserProfileDto, ApplicationError> {
        let uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|_| DomainError::UserNotFound)?;
        
        let user = self.user_repository
            .find_by_id(uuid)
            .await
            .map_err(map_repo_err)?
            .ok_or(DomainError::UserNotFound)?;
        
        debug!(user_id = %user.id, "Found user by ID");
        
        Ok(UserProfileDto::from(user))
    }

    /// Find or create a user based on their provider profile
    async fn find_or_create_user(
        &self,
        _provider: Provider,
        profile: ProviderUserProfile,
    ) -> Result<User, ApplicationError> {
        // Email is required for linking
        let email = profile.email.ok_or_else(|| {
            ApplicationError::Service("Email is required from OAuth provider".to_string())
        })?;
        
        // Try to find the user by email (primary linking mechanism)
        if let Some(user) = self.user_repository
            .find_by_email(&email)
            .await
            .map_err(map_repo_err)?
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
            .map_err(map_repo_err)?;
        
        info!(user_id = %created_user.id, "Created new user");
        
        Ok(created_user)
    }

    /// Get a provider token for a user
    pub async fn get_provider_token(
        &self,
        user_id: &str,
        provider_name: &str,
    ) -> Result<ProviderTokens, ApplicationError> {
        let uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|_| DomainError::UserNotFound)?;
        
        let provider = Provider::from_str(provider_name)
            .ok_or_else(|| DomainError::ProviderNotSupported(provider_name.to_string()))?;
        
        let tokens = self.token_repository
            .get_provider_tokens(uuid, provider)
            .await
            .map_err(map_repo_err)?
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
    use crate::service::TokenService;
    use domain::entity::{
        provider::{Provider, ProviderTokens, ProviderUserProfile},
        token::{JwkSet, TokenClaims},
    };
    use domain::port::{
        repository::{TokenRepository, UserRepository},
        service::ProviderOAuth2Client,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    // Mock implementations for testing
    
    // Mock implementations would be added here
} 