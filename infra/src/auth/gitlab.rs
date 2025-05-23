use async_trait::async_trait;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl,
    AuthorizationCode, TokenResponse, reqwest::async_http_client,
};
use domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
use domain::error::DomainError;
use domain::port::service::ProviderOAuth2Client;
use application::auth::{AuthService, AuthError};
use serde::Deserialize;
use tracing::{debug, error};

const GITLAB_AUTH_URL: &str = "https://gitlab.com/oauth/authorize";
const GITLAB_TOKEN_URL: &str = "https://gitlab.com/oauth/token";
const GITLAB_USER_URL: &str = "https://gitlab.com/api/v4/user";

/// GitLab user response from the API
#[derive(Debug, Deserialize)]
struct GitLabUser {
    /// User ID
    id: i64,
    /// Username
    username: String,
    /// Email
    email: Option<String>,
    /// Avatar URL
    avatar_url: Option<String>,
}

/// GitLab OAuth2 client
pub struct GitLabOAuth2Client {
    client: BasicClient,
}

impl GitLabOAuth2Client {
    /// Create a new GitLab OAuth2 client
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_url: String,
    ) -> Self {
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(GITLAB_AUTH_URL.to_string()).unwrap(),
            Some(TokenUrl::new(GITLAB_TOKEN_URL.to_string()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap());

        Self { client }
    }

    /// Create a new GitLab OAuth2 client from a ProviderConfig
    pub fn from_config(config: &crate::config::ProviderConfig) -> Self {
        Self::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.redirect_uri.clone(),
        )
    }
}

#[async_trait]
impl ProviderOAuth2Client for GitLabOAuth2Client {
    fn generate_authorize_url(&self) -> String {
        // Generate the authorization URL
        let (auth_url, _csrf_token) = self
            .client
            .authorize_url(|| oauth2::CsrfToken::new_random())
            .add_scope(oauth2::Scope::new("read_user".to_string()))
            .url();

        auth_url.to_string()
    }

    async fn exchange_code(&self, code: &str) -> Result<ProviderTokens, DomainError> {
        debug!("Exchanging GitLab authorization code for tokens");
        
        let token_result = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| {
                error!("Failed to exchange GitLab code for token: {}", e);
                DomainError::OAuth2Error(format!("GitLab token exchange failed: {}", e))
            })?;

        // Convert to domain ProviderTokens
        let provider_tokens = ProviderTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|r| r.secret().clone()),
            expires_in: token_result.expires_in().map(|e| e.as_secs()),
        };

        debug!("Successfully exchanged GitLab code for token");
        
        Ok(provider_tokens)
    }

    async fn get_user_profile(&self, tokens: &ProviderTokens) -> Result<ProviderUserProfile, DomainError> {
        debug!("Fetching GitLab user profile");
        
        // Create HTTP client
        let client = reqwest::Client::new();
        
        // Fetch user data from GitLab API
        let gitlab_user = client
            .get(GITLAB_USER_URL)
            .header("User-Agent", "IAM-Service")
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch GitLab user profile: {}", e);
                DomainError::UserProfileError(format!("GitLab API request failed: {}", e))
            })?
            .json::<GitLabUser>()
            .await
            .map_err(|e| {
                error!("Failed to parse GitLab user response: {}", e);
                DomainError::UserProfileError(format!("Failed to parse GitLab user: {}", e))
            })?;

        // Convert to domain ProviderUserProfile
        let profile = ProviderUserProfile {
            id: gitlab_user.id.to_string(),
            username: gitlab_user.username,
            email: gitlab_user.email,
            avatar_url: gitlab_user.avatar_url,
        };

        debug!("Successfully fetched GitLab profile for user: {}", profile.username);
        
        Ok(profile)
    }
}

#[async_trait]
impl AuthService for GitLabOAuth2Client {
    type Error = AuthError;

    fn provider(&self) -> Provider {
        Provider::GitLab
    }

    async fn exchange_code(
        &self,
        code: String,
        _redirect_uri: String,
    ) -> Result<(ProviderTokens, ProviderUserProfile), Self::Error> {
        debug!("Exchanging GitLab authorization code for tokens and profile");
        
        // Exchange code for tokens using ProviderOAuth2Client trait
        let tokens = ProviderOAuth2Client::exchange_code(self, &code).await
            .map_err(|e| AuthError::AuthenticationError(format!("GitLab token exchange failed: {}", e)))?;
        
        // Get user profile using ProviderOAuth2Client trait
        let profile = ProviderOAuth2Client::get_user_profile(self, &tokens).await
            .map_err(|e| AuthError::AuthenticationError(format!("GitLab profile fetch failed: {}", e)))?;
        
        debug!("Successfully exchanged GitLab code for tokens and profile");
        
        Ok((tokens, profile))
    }
} 