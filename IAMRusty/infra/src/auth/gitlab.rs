use application::auth::{AuthError, OAuthService};
use async_trait::async_trait;
use configuration::GitLabConfig;
use domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
use domain::error::DomainError;
use domain::port::service::ProviderOAuth2Client;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, RedirectUrl, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tracing::{debug, error};

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
    user_url: String,
}

impl GitLabOAuth2Client {
    /// Create a new GitLab OAuth2 client
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_url: String,
        auth_url: String,
        token_url: String,
        user_url: String,
    ) -> Self {
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(auth_url).unwrap(),
            Some(TokenUrl::new(token_url).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap());

        Self { client, user_url }
    }

    /// Create a new GitLab OAuth2 client from a GitlabConfig
    pub fn from_config(config: &GitLabConfig) -> Self {
        Self::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.redirect_uri.clone(),
            config.auth_url.clone(),
            config.token_url.clone(),
            config.user_url.clone(),
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

    async fn get_user_profile(
        &self,
        tokens: &ProviderTokens,
    ) -> Result<ProviderUserProfile, DomainError> {
        debug!("Fetching GitLab user profile");

        // Create HTTP client
        let client = reqwest::Client::new();

        debug!("GitLab user URL: {}", self.user_url);

        // Fetch user data from GitLab API
        let response = client
            .get(self.user_url.clone())
            .header("User-Agent", "IAM-Service")
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch GitLab user profile: {}", e);
                DomainError::UserProfileError(format!("GitLab API request failed: {}", e))
            })?;

        debug!("GitLab API response status: {}", response.status());
        debug!("GitLab API response headers: {:?}", response.headers());

        let text = response.text().await.map_err(|e| {
            error!("Failed to get GitLab response text: {}", e);
            DomainError::UserProfileError(format!("Failed to get GitLab response text: {}", e))
        })?;

        debug!("GitLab API response body: {}", text);

        let gitlab_user = serde_json::from_str::<GitLabUser>(&text).map_err(|e| {
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

        debug!(
            "Successfully fetched GitLab profile for user: {}",
            profile.username
        );

        Ok(profile)
    }
}

#[async_trait]
impl OAuthService for GitLabOAuth2Client {
    type Error = AuthError;

    fn provider(&self) -> Provider {
        Provider::GitLab
    }

    fn generate_authorize_url(&self) -> String {
        ProviderOAuth2Client::generate_authorize_url(self)
    }

    async fn exchange_code(
        &self,
        code: String,
        _redirect_uri: String,
    ) -> Result<(ProviderTokens, ProviderUserProfile), Self::Error> {
        debug!("GitLab exchange_code called with code: {}", code);
        debug!(
            "GitLab token URL: {}",
            self.client.token_url().unwrap().as_str()
        );
        debug!("GitLab user URL: {}", self.user_url);

        // Exchange code for tokens using ProviderOAuth2Client trait
        let tokens = ProviderOAuth2Client::exchange_code(self, &code)
            .await
            .map_err(|e| {
                debug!("GitLab token exchange failed: {}", e);
                AuthError::AuthenticationError(format!("GitLab token exchange failed: {}", e))
            })?;

        debug!(
            "GitLab token exchange successful, access_token: {}",
            &tokens.access_token[..std::cmp::min(10, tokens.access_token.len())]
        );

        // Get user profile using ProviderOAuth2Client trait
        let profile = ProviderOAuth2Client::get_user_profile(self, &tokens)
            .await
            .map_err(|e| {
                debug!("GitLab profile fetch failed: {}", e);
                AuthError::AuthenticationError(format!("GitLab profile fetch failed: {}", e))
            })?;

        debug!("GitLab profile fetch successful: {}", profile.username);

        Ok((tokens, profile))
    }
}
