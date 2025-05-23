use async_trait::async_trait;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl,
    AuthorizationCode, TokenResponse, reqwest::async_http_client,
};
use oauth2::StandardTokenResponse;
use oauth2::basic::BasicTokenType;
use domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
use domain::error::DomainError;
use domain::port::service::ProviderOAuth2Client;
use application::auth::{AuthService, AuthError};
use serde::Deserialize;
use tracing::{debug, error};

const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";

/// GitHub user response from the API
#[derive(Debug, Deserialize)]
struct GitHubUser {
    /// User ID
    id: i64,
    /// Username
    login: String,
    /// Email (may be null if not public)
    email: Option<String>,
    /// Avatar URL
    avatar_url: Option<String>,
}

/// GitHub OAuth2 client
pub struct GitHubOAuth2Client {
    client: BasicClient,
}

impl GitHubOAuth2Client {
    /// Create a new GitHub OAuth2 client
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_url: String,
    ) -> Self {
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(GITHUB_AUTH_URL.to_string()).unwrap(),
            Some(TokenUrl::new(GITHUB_TOKEN_URL.to_string()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap());

        Self { client }
    }

    /// Create a new GitHub OAuth2 client from a ProviderConfig
    pub fn from_config(config: &crate::config::ProviderConfig) -> Self {
        Self::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.redirect_uri.clone(),
        )
    }
}

#[async_trait]
impl ProviderOAuth2Client for GitHubOAuth2Client {
    fn generate_authorize_url(&self) -> String {
        // Generate the authorization URL
        let (auth_url, _csrf_token) = self
            .client
            .authorize_url(|| oauth2::CsrfToken::new_random())
            .add_scope(oauth2::Scope::new("user:email".to_string()))
            .url();

        auth_url.to_string()
    }

    async fn exchange_code(&self, code: &str) -> Result<ProviderTokens, DomainError> {
        debug!("Exchanging GitHub authorization code for tokens");
        
        let token_result = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| {
                error!("Failed to exchange GitHub code for token: {}", e);
                DomainError::OAuth2Error(format!("GitHub token exchange failed: {}", e))
            })?;

        // Convert to domain ProviderTokens
        let provider_tokens = ProviderTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|r| r.secret().clone()),
            expires_in: token_result.expires_in().map(|e| e.as_secs()),
        };

        debug!("Successfully exchanged GitHub code for token");
        
        Ok(provider_tokens)
    }

    async fn get_user_profile(&self, tokens: &ProviderTokens) -> Result<ProviderUserProfile, DomainError> {
        debug!("Fetching GitHub user profile");
        
        // Create HTTP client
        let client = reqwest::Client::new();
        
        // Fetch user data from GitHub API
        let github_user = client
            .get(GITHUB_USER_URL)
            .header("User-Agent", "IAM-Service")
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("token {}", tokens.access_token))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch GitHub user profile: {}", e);
                DomainError::UserProfileError(format!("GitHub API request failed: {}", e))
            })?
            .json::<GitHubUser>()
            .await
            .map_err(|e| {
                error!("Failed to parse GitHub user response: {}", e);
                DomainError::UserProfileError(format!("Failed to parse GitHub user: {}", e))
            })?;

        // Convert to domain ProviderUserProfile
        let profile = ProviderUserProfile {
            id: github_user.id.to_string(),
            username: github_user.login,
            email: github_user.email,
            avatar_url: github_user.avatar_url,
        };

        debug!("Successfully fetched GitHub profile for user: {}", profile.username);
        
        Ok(profile)
    }
}

#[async_trait]
impl AuthService for GitHubOAuth2Client {
    type Error = AuthError;

    fn provider(&self) -> Provider {
        Provider::GitHub
    }

    async fn exchange_code(
        &self,
        code: String,
        _redirect_uri: String,
    ) -> Result<(ProviderTokens, ProviderUserProfile), Self::Error> {
        debug!("Exchanging GitHub authorization code for tokens and profile");
        
        // Exchange code for tokens using ProviderOAuth2Client trait
        let tokens = ProviderOAuth2Client::exchange_code(self, &code).await
            .map_err(|e| AuthError::AuthenticationError(format!("GitHub token exchange failed: {}", e)))?;
        
        // Get user profile using ProviderOAuth2Client trait
        let profile = ProviderOAuth2Client::get_user_profile(self, &tokens).await
            .map_err(|e| AuthError::AuthenticationError(format!("GitHub profile fetch failed: {}", e)))?;
        
        debug!("Successfully exchanged GitHub code for tokens and profile");
        
        Ok((tokens, profile))
    }
} 